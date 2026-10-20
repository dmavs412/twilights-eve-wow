<#
.SYNOPSIS
Build the launcher in release and copy it to play/<short-commit>/, the folder Davis plays from (#192).

Content and the DBC patch are compiled into the exe, so a copy is the exe plus a run.ps1 that
points it at the game data. A release build keeps benilla-config beside the exe, so each copy
has its own settings. The newest 3 copies are kept; a copy whose exe is running is never
deleted, so playing never blocks the next publish.

.EXAMPLE
.\scripts\publish-play.ps1
#>
param([int]$Keep = 3)

# No $ErrorActionPreference = 'Stop': on Windows PowerShell 5.1 it turns cargo's progress lines
# on stderr into terminating errors. Failures are checked explicitly instead.
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

# Outside a Claude session no hook guards cargo, so take the build lock here.
$lock = Join-Path $Root 'target\.claude-build.lock'
$me = "publish-play-$PID"
if (Test-Path $lock) {
    $holder = Get-Content $lock -Raw | ConvertFrom-Json
    $age = [DateTimeOffset]::UtcNow - [DateTimeOffset]::Parse($holder.started)
    if ($env:CLAUDE_CODE_SESSION_ID -ne $holder.session -and $age.TotalHours -lt 2) {
        Write-Host ("build lock held by {0} for {1:N0} min; wait for it" -f $holder.session, $age.TotalMinutes)
        exit 1
    }
}
$ownLock = -not (Test-Path $lock)
if ($ownLock) {
    New-Item -ItemType Directory -Force (Split-Path $lock) | Out-Null
    @{ session = $me; started = (Get-Date).ToUniversalTime().ToString('o') } | ConvertTo-Json -Compress | Set-Content $lock -Encoding utf8
}
try {
    cargo build --release -q -p warcraft-survivors
    if ($LASTEXITCODE -ne 0) { Write-Host 'release build failed'; exit 1 }
} finally {
    if ($ownLock) { Remove-Item $lock -Force -ErrorAction SilentlyContinue }
}

$commit = git rev-parse --short HEAD
$dest = Join-Path $Root "play\$commit"
$running = Get-Process warcraft-survivors -ErrorAction SilentlyContinue | ForEach-Object { $_.Path }
New-Item -ItemType Directory -Force $dest | Out-Null
if ($running -contains (Join-Path $dest 'warcraft-survivors.exe')) {
    Write-Host "play\$commit is running; left as it is"
} else {
    Copy-Item 'target\release\warcraft-survivors.exe' $dest -Force
}
@'
# Play this copy. Game data defaults to this machine's client; override with -Data or $env:WOW_DATA.
param([string]$Data = $(if ($env:WOW_DATA) { $env:WOW_DATA } else { 'C:\Users\davis\wow-data\Data' }))
$env:WOW_DATA = $Data
$env:WOW_TWILIGHT = '1'
& "$PSScriptRoot\warcraft-survivors.exe"
'@ | Set-Content (Join-Path $dest 'run.ps1') -Encoding utf8
Write-Host "published play\$commit"

# Keep the newest $Keep copies; skip any whose exe is running.
Get-ChildItem (Join-Path $Root 'play') -Directory | Sort-Object LastWriteTime -Descending |
    Select-Object -Skip $Keep | ForEach-Object {
        $exe = Join-Path $_.FullName 'warcraft-survivors.exe'
        if ($running -contains $exe) { Write-Host "kept $($_.Name): its exe is running" }
        else { Remove-Item $_.FullName -Recurse -Force }
    }
