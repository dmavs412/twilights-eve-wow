<#
.SYNOPSIS
Promote a Testing release to Live and restart the host server on it (#310, ADR 0016).

Without -Commit, takes the newest release and refuses unless it is a pre-release. With -Commit,
takes b-<short>, which may already be Live: that is a rollback. Live is the release GitHub
marks latest; this script changes the flags and never rebuilds.

Then it downloads twilight-server.exe and manifest.json from that release, checks the hash,
asks a running host server to stop through <host>\server-data\stop (which saves everyone),
swaps the exe into the host folder and starts host-server.ps1 in a new window. If the server
does not stop within 60 s the old exe stays and the release is Live anyway; the script says so.

.PARAMETER Commit
The short commit of the release to make Live. Default: the newest release.

.EXAMPLE
.\scripts\promote-live.ps1
.EXAMPLE
.\scripts\promote-live.ps1 -Commit 4b473aa
#>
param([string]$Commit)

# No $ErrorActionPreference = 'Stop' (see publish-play.ps1); $LASTEXITCODE is checked instead.
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root
. "$PSScriptRoot\release-lib.ps1"

if ($Commit) {
    $tag = "b-$Commit"
    $release = gh release view $tag -R $ReleaseRepo --json tagName,isPrerelease,isLatest | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { Write-Host "release $tag not found"; exit 1 }
} else {
    $list = gh release list -R $ReleaseRepo --limit 1 --json tagName,isPrerelease,isLatest | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { Write-Host 'gh release list failed'; exit 1 }
    $release = $list | Select-Object -First 1
    if (-not $release) { Write-Host 'no releases yet; run release-testing.ps1 first'; exit 1 }
    if (-not $release.isPrerelease) { Write-Host "nothing in Testing to promote: newest release $($release.tagName) is already Live"; exit 1 }
    $tag = $release.tagName
}

gh release edit $tag -R $ReleaseRepo --prerelease=false --latest | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Host "gh release edit $tag failed"; exit 1 }
Write-Host "Live is now $tag"

# Fetch the server from the same release and check it against the manifest.
$hostDir = Get-HostDir
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "twilight-promote-$tag-$PID"
if (Test-Path $tmp) { Remove-Item $tmp -Recurse -Force }
New-Item -ItemType Directory -Force $tmp | Out-Null
gh release download $tag -R $ReleaseRepo -p twilight-server.exe -p manifest.json -D $tmp
if ($LASTEXITCODE -ne 0) { Write-Host "download of $tag failed; $tag is Live, the host server was not restarted"; exit 1 }
try {
    $manifestPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath((Join-Path $tmp 'manifest.json'))
    $manifest = [System.IO.File]::ReadAllText($manifestPath) | ConvertFrom-Json
    $want = $manifest.files.'twilight-server.exe'.sha256
    $have = Get-Sha256 (Join-Path $tmp 'twilight-server.exe')
    if ($have -ne $want) { throw "twilight-server.exe hash $have, manifest says $want" }
} catch {
    Write-Host "manifest check failed: $_"
    Write-Host "$tag is Live, the host server was not restarted"
    exit 1
}

# Ask a server running from the host folder to stop; the stop file saves everyone (#307).
$hostExe = Join-Path $hostDir 'twilight-server.exe'
$dataDir = Join-Path $hostDir 'server-data'
$running = Get-Process twilight-server -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $hostExe }
if ($running) {
    New-Item -ItemType Directory -Force $dataDir | Out-Null
    New-Item -ItemType File -Force (Join-Path $dataDir 'stop') | Out-Null
    Write-Host 'asked the host server to stop; waiting up to 60 s'
    $stopped = $running | ForEach-Object { $_.WaitForExit(60000) }
    if ($stopped -contains $false) {
        Write-Host "the host server did not stop in 60 s; old exe left in place"
        Write-Host "$tag is Live but the host server still runs the old build; stop it by hand and run host-server.ps1"
        exit 1
    }
    Remove-Item (Join-Path $dataDir 'stop') -Force -ErrorAction SilentlyContinue
}

New-Item -ItemType Directory -Force $hostDir | Out-Null
Copy-Item (Join-Path $tmp 'twilight-server.exe') $hostExe -Force
Copy-Item (Join-Path $tmp 'manifest.json') (Join-Path $hostDir 'manifest.json') -Force
Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
$hostScript = Join-Path $PSScriptRoot 'host-server.ps1'
Start-Process powershell -ArgumentList @('-NoExit', '-ExecutionPolicy', 'Bypass', '-File', "`"$hostScript`"")
if ($running) { Write-Host "host server restarted on $tag in a new window" }
else { Write-Host "host server started on $tag in a new window (none was running)" }
