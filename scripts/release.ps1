# Adapter for the shared host, static SMF bridge and this app's DLL CRT.
$ErrorActionPreference = 'Stop'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format json | ConvertFrom-Json)[0]
if (-not $installation) { throw 'MSVC x64 tools not found' }
$major = [int]($installation.installationVersion.Split('.')[0])
$generator = switch ($major) {
    17 { 'Visual Studio 17 2022' }
    18 { 'Visual Studio 18 2026' }
    default { throw "Unsupported Visual Studio major version: $major" }
}
if (-not (Test-Path -LiteralPath '../cat-plugin-player/shared/host.cmake')) {
    throw 'cat-plugin-player sibling checkout must contain the shared host source'
}
# The shared build driver alone owns configure/build and the existing cache.
$previousGenerator = $env:UH_GENERATOR
try {
    $env:UH_GENERATOR = $generator
    cargo build --release --locked
    if ($LASTEXITCODE) { exit $LASTEXITCODE }
} finally {
    $env:UH_GENERATOR = $previousGenerator
}
# Check the executable and its exact embedded host DLL; no installed DLL is borrowed.
$dumpbin = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -find 'VC/Tools/MSVC/**/bin/Hostx64/x64/dumpbin.exe' | Select-Object -Last 1)
if (-not $dumpbin) { throw 'dumpbin not found' }
$package = (python -c "import tomllib; print(tomllib.load(open('Cargo.toml', 'rb'))['package']['name'])")
if ($LASTEXITCODE) { exit $LASTEXITCODE }
foreach ($binary in @("target/release/$package.exe", 'target/shim/out/uapmd_shim.dll')) {
    $imports = & $dumpbin /nologo /dependents $binary
    if ($LASTEXITCODE) { exit $LASTEXITCODE }
    $imports | Write-Output
    if ($imports -match '(?i)\b(?:(?:vcruntime|msvcp)\d+(?:_\d+)?d|ucrtbased)\.dll\b') {
        throw "Release binary imports debug CRT DLLs: $binary"
    }
}
