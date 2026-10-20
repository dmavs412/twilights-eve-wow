# Build + run the native desktop game.
#
#   .\run.ps1
#
# The game data (a WoW 1.12.1 client's Data folder) lives outside the repo on this machine.
# Override per-run with -Data or $env:WOW_DATA.
param(
    [string]$Data = $(if ($env:WOW_DATA) { $env:WOW_DATA } else { 'C:\Users\davis\wow-data\Data' })
)
# No $ErrorActionPreference = 'Stop': on Windows PowerShell 5.1 it turns cargo's progress lines
# on stderr into terminating errors. Failures are checked explicitly instead.
Set-Location $PSScriptRoot

if (-not ((Test-Path (Join-Path $Data 'terrain.MPQ')) -and (Test-Path (Join-Path $Data 'patch.MPQ')))) {
    Write-Host "no game data at $Data (expected terrain.MPQ and patch.MPQ)"
    exit 1
}

$env:WOW_DATA = $Data
cargo run --release -p twilights-eve
exit $LASTEXITCODE
