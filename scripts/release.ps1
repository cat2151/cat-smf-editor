# Adapter for the shared host and static-CRT distribution build.
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
# Keep the reader-facing prose identical to the resolved player source.
if ((Get-Content -Raw -LiteralPath '.github/release-notes.md') -cne
    (Get-Content -Raw -LiteralPath '../cat-plugin-player/.github/release-notes.md')) {
    throw 'Release note templates differ between editor and player'
}
if ($env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS -notmatch '(?:^|\s)target-feature=\+crt-static(?:\s|$)') {
    throw 'Distribution builds require -C target-feature=+crt-static (local builds use cargo build --release)'
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
$package = (python -c "import tomllib; print(tomllib.load(open('Cargo.toml', 'rb'))['package']['name'])")
if ($LASTEXITCODE) { exit $LASTEXITCODE }
python ../cat-plugin-player/scripts/crt_linkage.py "target/release/$package.exe" target/shim/out/uapmd_shim.dll
if ($LASTEXITCODE) { exit $LASTEXITCODE }
