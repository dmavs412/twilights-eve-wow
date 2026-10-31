# Claude Code hook: one cargo command at a time across sessions (#191, spec #190).
# PreToolUse runs -Mode acquire, PostToolUse/PostToolUseFailure run -Mode release.
# Reads the hook JSON on stdin; acts only on Bash/PowerShell commands that run cargo.
#
# A command that goes to the background (`run_in_background`, or moved there at the tool's
# 10-minute cap) returns while cargo still compiles. Its release keeps the lock, marked
# `background`, and another session may take it once no cargo process runs.
param([Parameter(Mandatory)][ValidateSet('acquire', 'release')][string]$Mode)

$ErrorActionPreference = 'Stop'
$StaleHours = 2

$hook = [Console]::In.ReadToEnd() | ConvertFrom-Json
$command = "$($hook.tool_input.command)"
# Cargo as the command run: at a line's start or after `;`, `&`, `|`, `(` or `{`, behind any
# `NAME=value`, `timeout ...` or `cmd /c` prefix. A grep, a path or a message naming cargo passes.
$runsCargo = '(?m)(^|[;&|({])\s*((\w+=\S*|g?timeout(\s+\S+){1,4}|cmd(\.exe)?\s+/c\s+["'']?|(ba)?sh\s+-c\s+["'']?)\s*)*cargo(\.exe)?\b'
if ($command -notmatch $runsCargo) { exit 0 }

$root = if ($env:CLAUDE_PROJECT_DIR) { $env:CLAUDE_PROJECT_DIR } else { "$PSScriptRoot\.." }
$targetDir = Join-Path $root 'target'
$lock = Join-Path $targetDir '.claude-build.lock'
$me = "$($hook.session_id)"

function Read-Holder {
    if (-not (Test-Path $lock)) { return $null }
    try { return Get-Content $lock -Raw | ConvertFrom-Json } catch { return $null }
}

function Write-Holder([string]$started, [bool]$background) {
    New-Item -ItemType Directory -Force $targetDir | Out-Null
    @{ session = $me; started = $started; background = $background } |
        ConvertTo-Json -Compress | Set-Content $lock -Encoding utf8
}

if ($Mode -eq 'release') {
    $holder = Read-Holder
    if (-not ($holder -and $holder.session -eq $me)) { exit 0 }
    $response = "$($hook.tool_response | ConvertTo-Json -Compress -Depth 6)"
    $backgrounded = $hook.hook_event_name -ne 'PostToolUseFailure' -and
        ($hook.tool_input.run_in_background -eq $true -or $response -match 'background')
    if ($backgrounded) { Write-Holder $holder.started $true } else { Remove-Item $lock -Force }
    exit 0
}

# `timeout` stops cargo mid-compile, which corrupts the build (CLAUDE.md). A long build goes to
# the background instead.
# Only `timeout` as the command itself (at the start, or after `;`, `&`, `|` or `(`), so a commit
# message that mentions it passes.
if ($command -match '(?m)(^|[;&|(])\s*g?timeout\s+(\S+\s+){1,4}cargo\b') {
    [Console]::Error.WriteLine('Refused: `timeout` in front of cargo stops it mid-compile. Run a long build with run_in_background and wait for its notification.')
    exit 2
}

$holder = Read-Holder
# A script holding the lock for a whole run (overnight.ps1) names itself in CLAUDE_BUILD_LOCK_HOLDER;
# the sessions it starts inherit that and pass without touching its lock.
if ($holder -and $env:CLAUDE_BUILD_LOCK_HOLDER -and $holder.session -eq $env:CLAUDE_BUILD_LOCK_HOLDER) { exit 0 }
if ($holder -and $holder.session -ne $me) {
    $age = [DateTimeOffset]::UtcNow - [DateTimeOffset]::Parse($holder.started)
    # A background holder holds only while some cargo process still runs.
    $running = -not $holder.background -or [bool](Get-Process -Name cargo -ErrorAction SilentlyContinue)
    if ($running -and $age.TotalHours -lt $StaleHours) {
        $how = if ($holder.background) { ' (a background build still running)' } else { '' }
        [Console]::Error.WriteLine(("Build lock held by session {0} for {1:N0} min{2} (target/.claude-build.lock). Wait for it or stop." -f $holder.session, $age.TotalMinutes, $how))
        exit 2
    }
}

Write-Holder (Get-Date).ToUniversalTime().ToString('o') $false
exit 0
