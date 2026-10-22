<#
.SYNOPSIS
Build HEAD in release and publish it to the Testing channel: a pre-release tagged b-<short> (#310, ADR 0016).

Refuses a dirty tree, a HEAD that origin/main does not contain, and a tag that already exists.
Builds the game, the Launcher and the server, stages them with manifest.json under
target\release-stage\b-<short>\, then creates the pre-release with the four files.
Promote it to Live with promote-live.ps1 once it has been played.

.PARAMETER Notes
The patch notes for the release body. The Launcher shows them.

.PARAMETER DryRun
Build and write the stage folder and manifest, but create no release. Skips the origin/main
and existing-release checks; still refuses a dirty tree.

.EXAMPLE
.\scripts\release-testing.ps1 -Notes "Hogger siege: the Noise meter now shows on the boss bar."
#>
param(
    [Parameter(Mandatory)][string]$Notes,
    [switch]$DryRun
)

# No $ErrorActionPreference = 'Stop': on Windows PowerShell 5.1 it turns cargo's progress lines
# on stderr into terminating errors. Failures are checked explicitly instead.
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root
. "$PSScriptRoot\release-lib.ps1"

if (git status --porcelain) {
    Write-Host 'the working tree is dirty; commit or stash first'
    exit 1
}
$commit = git rev-parse HEAD
$short = git rev-parse --short HEAD
$tag = "b-$short"

if (-not $DryRun) {
    git fetch -q origin main
    git merge-base --is-ancestor HEAD origin/main
    if ($LASTEXITCODE -ne 0) {
        Write-Host "HEAD ($short) is not on origin/main; push first, the tag must point at a commit GitHub has"
        exit 1
    }
    gh release view $tag -R $ReleaseRepo --json tagName 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "release $tag already exists"
        exit 1
    }
}

# Outside a Claude session no hook guards cargo, so take the build lock here.
$lock = Join-Path $Root 'target\.claude-build.lock'
$me = "release-testing-$PID"
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
try {
    cargo build --release -q -p twilights-eve -p twilights-launcher -p twilight-server
    if ($LASTEXITCODE -ne 0) { Write-Host 'release build failed'; exit 1 }
} finally {
    if ($ownLock) { Remove-Item $lock -Force -ErrorAction SilentlyContinue }
}

$stage = Join-Path $Root "target\release-stage\$tag"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force $stage | Out-Null
foreach ($name in $ReleaseFiles) {
    Copy-Item (Join-Path $Root "target\release\$name") $stage -Force
}
try {
    $manifestPath = Write-ReleaseManifest $stage $tag $commit
    Test-ReleaseManifest $stage | Out-Null
} catch {
    Write-Host "manifest failed: $_"
    exit 1
}
Write-Host "staged $tag in $stage"

if ($DryRun) {
    Write-Host 'dry run: no release created'
    exit 0
}

$assets = @($ReleaseFiles | ForEach-Object { Join-Path $stage $_ }) + @($manifestPath)
gh release create $tag -R $ReleaseRepo --prerelease --target $commit --title $tag --notes $Notes @assets
if ($LASTEXITCODE -ne 0) { Write-Host 'gh release create failed'; exit 1 }
$url = gh release view $tag -R $ReleaseRepo --json url -q .url
Write-Host "Testing is now $tag"
Write-Host $url
