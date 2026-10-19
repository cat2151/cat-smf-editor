param(
    [int]$Runs = 3,
    [string]$Executable = "$PSScriptRoot/../target/release/cat-smf-editor.exe",
    [string]$OutputDirectory = (Join-Path ([IO.Path]::GetTempPath()) ('cat-smf-startup-' + [guid]::NewGuid())),
    [string]$SourceDataRoot = (Join-Path $env:LOCALAPPDATA 'cat-smf-editor'),
    [string]$Midi,
    [hashtable]$Environment = @{}
)
$ErrorActionPreference = 'Stop'
$Executable = (Resolve-Path -LiteralPath $Executable).Path
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
$results = @()
function Copy-CacheTree([string]$Source, [string]$Destination) {
    # Preserve resource junctions: never traverse/copy Surge's installed data tree.
    $item = Get-Item -LiteralPath $Source
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        New-Item -ItemType Junction -Path $Destination -Target $item.Target | Out-Null
    } elseif ($item.PSIsContainer) {
        New-Item -ItemType Directory -Path $Destination -Force | Out-Null
        foreach ($child in Get-ChildItem -LiteralPath $Source -Force) {
            Copy-CacheTree $child.FullName (Join-Path $Destination $child.Name)
        }
    } else {
        Copy-Item -LiteralPath $Source -Destination $Destination
    }
}
for ($run = 1; $run -le $Runs; $run++) {
    # Each process gets the same real settings/caches; it cannot rewrite the user's status.
    # The referenced MIDI and plugins stay at their actual paths and are not edited.
    $localData = Join-Path $OutputDirectory "run-$run"
    $appData = Join-Path $localData 'cat-smf-editor'
    New-Item -ItemType Directory -Path "$appData/cache" -Force | Out-Null
    foreach ($name in @('status.json', 'settings.toml', 'recent-files.json', 'startup.json')) {
        $source = Join-Path $SourceDataRoot $name
        if (Test-Path -LiteralPath $source) { Copy-Item -LiteralPath $source -Destination $appData }
    }
    foreach ($name in @('native', 'patch-states-v1', 'patch-roots-v1.json')) {
        $source = Join-Path "$SourceDataRoot/cache" $name
        if (Test-Path -LiteralPath $source) { Copy-Item -LiteralPath $source -Destination "$appData/cache" -Recurse }
    }
    foreach ($name in @('surge-data-min-v1', 'surge-data-min-v2')) {
        $surgeCache = Join-Path "$SourceDataRoot/cache" $name
        if (Test-Path -LiteralPath $surgeCache) {
            Copy-CacheTree $surgeCache (Join-Path "$appData/cache" $name)
        }
    }
    $trace = Join-Path $localData 'trace.json'
    $info = [Diagnostics.ProcessStartInfo]::new($Executable)
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    if ($Midi) { $info.ArgumentList.Add($Midi) }
    $info.Environment['LOCALAPPDATA'] = $localData
    $info.Environment['CAT_SMF_STARTUP_TRACE'] = $trace
    $info.Environment['CAT_SMF_STARTUP_TRACE_QUIT'] = '1'
    foreach ($entry in $Environment.GetEnumerator()) {
        $info.Environment[$entry.Key] = $entry.Value
    }
    $process = [Diagnostics.Process]::Start($info)
    try {
        $processStart = $process.StartTime.ToUniversalTime()
        $deadline = [DateTime]::UtcNow.AddSeconds(45)
        $data = $null
        do {
            Start-Sleep -Milliseconds 20
            if (Test-Path -LiteralPath $trace) {
                try { $data = Get-Content -LiteralPath $trace -Raw | ConvertFrom-Json } catch { }
            }
            if ($process.HasExited) { throw "Editor exited before audio, code $($process.ExitCode)" }
        } while ((!$data -or !$data.first_nonzero_output_us) -and [DateTime]::UtcNow -lt $deadline)
        if (!$data -or !$data.first_nonzero_output_us) { throw "No nonzero output within 45s: $trace" }
        $mainStart = [DateTimeOffset]::FromUnixTimeMilliseconds([long]($data.main_start_unix_us / 1000)).UtcDateTime
        $entry = [pscustomobject]@{
            Run = $run
            ProcessToMainMs = [math]::Round(($mainStart - $processStart).TotalMilliseconds, 2)
            MainToCallbackMs = [math]::Round($data.first_sequence_callback_us / 1000, 2)
            MainToNonzeroMs = [math]::Round($data.first_nonzero_output_us / 1000, 2)
            ProcessToNonzeroMs = [math]::Round(($mainStart - $processStart).TotalMilliseconds + $data.first_nonzero_output_us / 1000, 2)
            Trace = $trace
        }
        $results += $entry
        $entry | Format-List | Out-Host
    } finally {
        if (!$process.HasExited) {
            # Trace mode requests a normal editor close after deferred restoration.
            # The fallback also supports the pre-change executable for comparisons.
            if (!$process.WaitForExit(1500)) { $null = $process.CloseMainWindow() }
            if (!$process.WaitForExit(15000)) { $process.Kill(); $process.WaitForExit() }
        }
        $process.Dispose()
    }
}
$results | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'results.json') -Encoding utf8
Write-Output "Results: $OutputDirectory"
