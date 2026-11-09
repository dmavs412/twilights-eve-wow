<#
.SYNOPSIS
Take the mist look's capture shots at the Westfall and Steppes edges, or compare two sets (#662).

Each shot is an ON_DEMAND capture scenario in crates/benilla-app/src/capture/scenarios.rs. A
labelled run builds the client in this checkout's target, then captures every shot (or the ones
-Only picks) into target\visual\mist\<label>\. -Compare puts each shot that is in both the before
and the after set side by side in target\visual\mist\compare\, as a PNG and a JPEG small enough to
upload. WOW_CAPTURE=list prints only the golden scenarios, so the shot list lives here; a test in
scenarios.rs (mist_tests) keeps it in step with the Rust table.

.EXAMPLE
powershell -NoProfile -File scripts/mist-shots.ps1 -Label before
.EXAMPLE
powershell -NoProfile -File scripts/mist-shots.ps1 -Label after -Only mist-close-*
.EXAMPLE
powershell -NoProfile -File scripts/mist-shots.ps1 -Compare
#>
param(
    [string]$Label,
    [string]$Only = '*',
    [switch]$Compare
)

$Shots = @(
    'mist-straight-noon'
    'mist-straight-dusk'
    'mist-straight-night'
    'mist-step-noon'
    'mist-step-dusk'
    'mist-step-night'
    'mist-stair-noon'
    'mist-stair-dusk'
    'mist-stair-night'
    'mist-close-noon'
    'mist-close-dusk'
    'mist-close-night'
    'mist-far-noon'
    'mist-far-dusk'
    'mist-far-night'
    'mist-steppes-noon'
    'mist-steppes-dusk'
    'mist-steppes-night'
)

# No $ErrorActionPreference = 'Stop': on Windows PowerShell 5.1 it turns cargo's progress lines
# on stderr into terminating errors. Failures are checked explicitly instead.
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root
$Visual = Join-Path $Root 'target\visual\mist'
$Picked = @($Shots | Where-Object { $_ -like $Only })
if ($Picked.Count -eq 0) { Write-Host "no shot matches $Only"; exit 1 }

if ($Compare) {
    Add-Type -AssemblyName System.Drawing
    $img = Join-Path $PSScriptRoot 'img.ps1'
    $out = Join-Path $Visual 'compare'
    New-Item -ItemType Directory -Force $out | Out-Null
    $missing = 0
    foreach ($name in $Picked) {
        $before = Join-Path $Visual "before\$name.png"
        $after = Join-Path $Visual "after\$name.png"
        if (-not ((Test-Path $before) -and (Test-Path $after))) {
            Write-Host "$name missing a before or after shot"
            $missing++
            continue
        }
        $bmp = [System.Drawing.Image]::FromFile($before)
        $rect = '0,0,{0},{1}' -f $bmp.Width, $bmp.Height
        $bmp.Dispose()
        $png = Join-Path $out "$name.png"
        $jpg = Join-Path $out "$name.jpg"
        powershell -NoProfile -File $img compare -File $before -Other $after -Rect $rect -Out $png
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path $png)) { Write-Host "$name compare failed"; $missing++; continue }
        powershell -NoProfile -File $img shrink -File $png -Width 1600 -Out $jpg
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path $jpg)) { Write-Host "$name shrink failed"; $missing++; continue }
        Write-Host "$name compared: $jpg"
    }
    if ($missing -gt 0) { exit 1 }
    exit 0
}

if (-not $Label) { Write-Host 'give -Label <word> to take shots, or -Compare'; exit 1 }

# Outside a Claude session no hook guards cargo, so take the build lock here.
$lock = Join-Path $Root 'target\.claude-build.lock'
$me = "mist-shots-$PID"
if (Test-Path $lock) {
    $holder = Get-Content $lock -Raw | ConvertFrom-Json
    $age = [DateTimeOffset]::UtcNow - [DateTimeOffset]::Parse($holder.started)
    if ($env:CLAUDE_BUILD_LOCK_HOLDER -ne $holder.session -and $age.TotalHours -lt 2) {
        Write-Host ("build lock held by {0} for {1:N0} min; wait for it" -f $holder.session, $age.TotalMinutes)
        exit 1
    }
}
$ownLock = -not (Test-Path $lock)
if ($ownLock) {
    New-Item -ItemType Directory -Force (Split-Path $lock) | Out-Null
    @{ session = $me; started = (Get-Date).ToUniversalTime().ToString('o') } | ConvertTo-Json -Compress | Set-Content $lock -Encoding utf8
}
# A client build waits for the machine-wide client lock that the builders share
# (scripts/client-lock.ps1).
. (Join-Path $PSScriptRoot 'client-lock.ps1')
$dir = Join-Path $Visual $Label
$missing = 0
try {
    if (-not (Enter-ClientLock $me $PID 60)) { Write-Host ("client build lock held by {0}; gave up after 60 min" -f (Get-ClientLockHolder).holder); exit 1 }
    cargo build -q -p benilla
    if ($LASTEXITCODE -ne 0) { Write-Host 'client build failed'; exit 1 }
    New-Item -ItemType Directory -Force $dir | Out-Null
    foreach ($name in $Picked) {
        $png = Join-Path $dir "$name.png"
        Remove-Item $png -Force -ErrorAction SilentlyContinue
        $env:WOW_CAPTURE = $name
        $env:WOW_CAPTURE_OUT = $png
        cargo run -q -p benilla
        if (Test-Path $png) { Write-Host "$name written" }
        else { Write-Host "$name missing"; $missing++ }
    }
} finally {
    Remove-Item Env:WOW_CAPTURE -ErrorAction SilentlyContinue
    Remove-Item Env:WOW_CAPTURE_OUT -ErrorAction SilentlyContinue
    Exit-ClientLock $me
    if ($ownLock) { Remove-Item $lock -Force -ErrorAction SilentlyContinue }
}
if ($missing -gt 0) { exit 1 }
