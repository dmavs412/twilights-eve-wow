# Shared pieces of the release scripts (#310, ADR 0016). Dot-source it:
#   . "$PSScriptRoot\release-lib.ps1"

$ReleaseRepo = 'dmavs412/twilights-eve-wow'
$ReleaseFiles = @('twilights-eve.exe', 'twilights-launcher.exe', 'twilight-server.exe')

function Get-HostDir {
    if ($env:TWILIGHT_HOST_DIR) { $env:TWILIGHT_HOST_DIR } else { 'C:\Users\davis\twilight-host' }
}

# Writes text as UTF-8 without a BOM. Set-Content -Encoding utf8 on PowerShell 5.1 adds one, and
# the Launcher's JSON parser rejects it.
function Write-Utf8NoBom([string]$Path, [string]$Text) {
    # .NET resolves a relative path against the process directory, not PowerShell's location.
    $full = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
    $enc = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($full, $Text, $enc)
}

function Get-Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

# Writes <Dir>\manifest.json in the shape crates/twilights-launcher/src/manifest.rs reads:
# {"tag": ..., "commit": <full sha>, "files": {"<exe>": {"sha256": <lowercase>, "size": <bytes>}}}.
# Every exe in $ReleaseFiles must already be in $Dir.
function Write-ReleaseManifest([string]$Dir, [string]$Tag, [string]$Commit) {
    $files = [ordered]@{}
    foreach ($name in $ReleaseFiles) {
        $path = Join-Path $Dir $name
        if (-not (Test-Path -LiteralPath $path)) { throw "manifest: $path is missing" }
        $files[$name] = [ordered]@{
            sha256 = Get-Sha256 $path
            size   = (Get-Item -LiteralPath $path).Length
        }
    }
    $manifest = [ordered]@{ tag = $Tag; commit = $Commit; files = $files }
    $json = ($manifest | ConvertTo-Json -Depth 4) -replace "`r`n", "`n"
    $out = Join-Path $Dir 'manifest.json'
    Write-Utf8NoBom $out ($json + "`n")
    $out
}

# Reads a manifest and checks each listed file in $Dir against its hash and size.
# Returns the manifest object; throws on the first mismatch.
function Test-ReleaseManifest([string]$Dir) {
    $path = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath((Join-Path $Dir 'manifest.json'))
    $bytes = [System.IO.File]::ReadAllBytes($path)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        throw "$path starts with a BOM"
    }
    $manifest = [System.Text.Encoding]::UTF8.GetString($bytes) | ConvertFrom-Json
    foreach ($entry in $manifest.files.PSObject.Properties) {
        $file = Join-Path $Dir $entry.Name
        if (-not (Test-Path -LiteralPath $file)) { throw "manifest lists $($entry.Name) but it is not in $Dir" }
        $hash = Get-Sha256 $file
        $size = (Get-Item -LiteralPath $file).Length
        if ($hash -ne $entry.Value.sha256) { throw "$($entry.Name): hash $hash, manifest says $($entry.Value.sha256)" }
        if ($size -ne $entry.Value.size) { throw "$($entry.Name): size $size, manifest says $($entry.Value.size)" }
    }
    $manifest
}
