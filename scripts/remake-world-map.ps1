param(
    [string]$Data = 'C:\Users\davis\wow-data\Data',
    [string]$Python
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    if (-not $Python) {
        $Python = & uv --cache-dir target/uv-cache python find
        if ($LASTEXITCODE -ne 0) { throw 'Set -Python to an installed Python executable.' }
        $Python = $Python.Trim()
    }
    & (Join-Path $PSScriptRoot 'remake-world-survey.ps1') -Data $Data
    & cargo run -q --release -p benilla-formats --example map_region -- --out target/map_region
    if ($LASTEXITCODE -ne 0) { throw 'Regional map assembly or coordinate validation failed.' }
    $copies = @{
        'world-map-geography.png' = 'world-map-geography.png'
        'world-map-detail.png' = 'world-map-detail.png'
        'world-map-exact-fog.png' = 'world-map-exact-fog.png'
        'world-map-exact.png' = 'world-map-exact.png'
        'world-map-region.json' = 'world-map-region.json'
    }
    foreach ($entry in $copies.GetEnumerator()) {
        Copy-Item -LiteralPath (Join-Path 'target/map_region' $entry.Key) -Destination (Join-Path 'data/twilights-eve/map' $entry.Value)
    }
    Copy-Item -LiteralPath 'target/map_region/world-map-region.json' -Destination 'data/twilights-eve/map/world-map-exact-validation.json'
    & $Python tools/twilights-eve/import_art.py data/twilights-eve/map crates/benilla-app/assets/ui/map --only world-map-geography,world-map-detail,world-map-exact-fog --tile 256
    if ($LASTEXITCODE -ne 0) { throw 'World map tile packaging failed.' }
    & $Python tools/twilights-eve/import_art.py target/map_region/covers crates/benilla-app/assets/ui/map/covers
    if ($LASTEXITCODE -ne 0) { throw 'Exploration cover packaging failed.' }
    Write-Output 'World map rebuilt and packaged. Rebuild the client to include the new tiles.'
} finally {
    Pop-Location
}
