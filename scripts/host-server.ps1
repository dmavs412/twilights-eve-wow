<#
.SYNOPSIS
Run Davis's twilight-server from the host folder in the foreground (#310, ADR 0016).

The host folder is $env:TWILIGHT_HOST_DIR, default C:\Users\davis\twilight-host. It holds
twilight-server.exe (placed there by promote-live.ps1), host.json and server-data\ (the saves).
With no host.json the script writes a template and exits 1 so it can be filled in.

.EXAMPLE
.\scripts\host-server.ps1
#>
. "$PSScriptRoot\release-lib.ps1"

$hostDir = Get-HostDir
$config = Join-Path $hostDir 'host.json'
if (-not (Test-Path $config)) {
    New-Item -ItemType Directory -Force $hostDir | Out-Null
    $template = @'
{
  "listen": "0.0.0.0:3724",
  "join_password": "",
  "data": "C:\\Users\\davis\\wow-data\\Data"
}
'@
    Write-Utf8NoBom $config $template
    Write-Host "wrote $config; fill in host.json"
    exit 1
}

$settings = Get-Content $config -Raw | ConvertFrom-Json
if (-not $settings.join_password) {
    Write-Host "join_password in $config is empty; fill it in"
    exit 1
}
$exe = Join-Path $hostDir 'twilight-server.exe'
if (-not (Test-Path $exe)) {
    Write-Host "$exe is missing; promote-live.ps1 puts it there"
    exit 1
}

$dataDir = Join-Path $hostDir 'server-data'
Write-Host "starting $exe on $($settings.listen), saves in $dataDir"
& $exe --listen $settings.listen --join-password $settings.join_password --data $settings.data --data-dir $dataDir
exit $LASTEXITCODE
