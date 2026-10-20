# Claude Code hook: one cargo command at a time across sessions (#191, spec #190).
# PreToolUse runs -Mode acquire, PostToolUse/PostToolUseFailure run -Mode release.
# Reads the hook JSON on stdin; acts only on Bash/PowerShell commands that mention cargo.
param([Parameter(Mandatory)][ValidateSet('acquire', 'release')][string]$Mode)

$ErrorActionPreference = 'Stop'
$StaleHours = 2

$hook = [Console]::In.ReadToEnd() | ConvertFrom-Json
$command = "$($hook.tool_input.command)"
if ($command -notmatch '\bcargo\b') { exit 0 }

$root = if ($env:CLAUDE_PROJECT_DIR) { $env:CLAUDE_PROJECT_DIR } else { "$PSScriptRoot\.." }
$targetDir = Join-Path $root 'target'
$lock = Join-Path $targetDir '.claude-build.lock'
$me = "$($hook.session_id)"

function Read-Holder {
    if (-not (Test-Path $lock)) { return $null }
    try { return Get-Content $lock -Raw | ConvertFrom-Json } catch { return $null }
}

if ($Mode -eq 'release') {
    $holder = Read-Holder
    if ($holder -and $holder.session -eq $me) { Remove-Item $lock -Force }
    exit 0
}

$holder = Read-Holder
if ($holder -and $holder.session -ne $me) {
    $age = [DateTimeOffset]::UtcNow - [DateTimeOffset]::Parse($holder.started)
    if ($age.TotalHours -lt $StaleHours) {
        [Console]::Error.WriteLine(("Build lock held by session {0} for {1:N0} min (target/.claude-build.lock). Wait for it or stop." -f $holder.session, $age.TotalMinutes))
        exit 2
    }
}

New-Item -ItemType Directory -Force $targetDir | Out-Null
@{ session = $me; started = (Get-Date).ToUniversalTime().ToString('o') } | ConvertTo-Json -Compress | Set-Content $lock -Encoding utf8
exit 0
