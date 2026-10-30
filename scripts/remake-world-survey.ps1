param([string]$Data = 'C:\Users\davis\wow-data\Data')
$ErrorActionPreference = 'Stop'
Push-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
try {
    & cargo run -q --release -p benilla-formats --example map_draft -- --data $Data --out target/map_stock_rebuild
    if ($LASTEXITCODE -ne 0) { throw 'Original map extraction failed.' }
    & cargo run -q --release -p benilla-formats --example map_survey -- --data $Data --out target/map_survey
    if ($LASTEXITCODE -ne 0) { throw 'Terrain survey build or projection validation failed.' }
    $destination = 'data/twilights-eve/map/survey'
    New-Item -ItemType Directory -Path $destination -Force > $null
    foreach ($name in @('terrain-survey.png','terrain-survey-fog.png','terrain-survey-playable.png','survey.json')) {
        Copy-Item -LiteralPath (Join-Path 'target/map_survey' $name) -Destination (Join-Path $destination $name)
    }
    Copy-Item -LiteralPath 'target/map_stock_rebuild/stock/Stormwind.png' -Destination (Join-Path $destination 'Stormwind.png')
    $surveyJson = Get-Content -LiteralPath (Join-Path $destination 'survey.json') -Raw
    Set-Content -LiteralPath (Join-Path $destination 'survey-data.js') -Value ('window.SURVEY = ' + $surveyJson + ';') -Encoding UTF8
    Write-Output 'Survey and original Stormwind reference saved. Open data/twilights-eve/map/survey/index.html.'
} finally { Pop-Location }
