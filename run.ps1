# Windows counterpart to `make start` / `make native`.
#
#   .\run.ps1            # serve the browser build at http://127.0.0.1:8090 (build web/dist first)
#   .\run.ps1 native     # build + run the native desktop version
#
# The game data (a WoW 1.12.1 client's Data folder) lives outside the repo on this machine.
# Override per-run with -Data or $env:WOW_DATA.
param(
    [ValidateSet('start', 'native')]
    [string]$Mode = 'start',
    [string]$Data = $(if ($env:WOW_DATA) { $env:WOW_DATA } else { 'C:\Users\davis\wow-data\Data' })
)
# No $ErrorActionPreference = 'Stop': on Windows PowerShell 5.1 it turns cargo's progress lines
# on stderr into terminating errors. Failures are checked explicitly instead.
Set-Location $PSScriptRoot

if (-not ((Test-Path (Join-Path $Data 'terrain.MPQ')) -and (Test-Path (Join-Path $Data 'patch.MPQ')))) {
    Write-Host "no game data at $Data (expected terrain.MPQ and patch.MPQ)"
    exit 1
}

switch ($Mode) {
    'start' {
        if (-not (Test-Path 'web\dist\wenilla_bg.wasm')) {
            Write-Host 'not built yet: run scripts/web-build.sh (Git Bash) first'
            exit 1
        }
        cargo run --release -p wenilla-host -- --www web/dist --data $Data
    }
    'native' {
        $env:WOW_DATA = $Data
        cargo run --release -p twilights-eve
    }
}
exit $LASTEXITCODE
