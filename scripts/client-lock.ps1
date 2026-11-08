# One client build at a time on this machine, across every checkout and worktree.
# Dot-sourced by build-lock.ps1 (the cargo hook), worker.ps1 (the gate) and publish-play.ps1.
#
# Two builders run tickets in parallel, each in its own worktree with its own target, so a realm
# or data build never waits for the other builder. A client build (benilla-*, the twilights-eve
# launcher, or the whole workspace) links a large binary, and two at once have run rustc out of
# memory and ended in LNK1103 (2026-10-15). Those take this machine-wide lock and wait for it.

$ClientLockPath = Join-Path $env:LOCALAPPDATA 'twilights-eve\client-build.lock'
$ClientLockStaleHours = 2

# True when $command runs a cargo subcommand that compiles the client: it names a client package,
# names the whole workspace, or names no package at all (the workspace root builds everything).
function Test-ClientCargo([string]$command) {
    $build = '(?m)(^|[;&|({])\s*((\w+=\S*|cmd(\.exe)?\s+/c\s+["'']?|(ba)?sh\s+-c\s+["'']?)\s*)*cargo(\.exe)?\s+(\+\S+\s+)?(build|check|test|run|clippy|bench|doc|b|c|t|r)\b(?<rest>[^\n;&|]*)'
    foreach ($m in [regex]::Matches($command, $build)) {
        $rest = $m.Groups['rest'].Value
        if ($rest -match '(-p|--package)[\s=]+(benilla-|twilights-eve\b)') { return $true }
        if ($rest -match '--workspace|--all\b') { return $true }
        if ($rest -notmatch '(-p|--package)[\s=]') { return $true }
    }
    return $false
}

function Read-ClientLock {
    if (-not (Test-Path $ClientLockPath)) { return $null }
    try { return Get-Content $ClientLockPath -Raw | ConvertFrom-Json } catch { return $null }
}

function Write-ClientLock([string]$holder, [int]$ownerPid, [string]$started, [bool]$background) {
    New-Item -ItemType Directory -Force (Split-Path $ClientLockPath) | Out-Null
    @{ holder = $holder; pid = $ownerPid; started = $started; background = $background } |
        ConvertTo-Json -Compress | Set-Content $ClientLockPath -Encoding utf8
}

# A holder still holds while: it is younger than the stale limit, its process (when it named
# one) is alive, and, when it went to the background, a cargo process for the client still runs.
function Test-ClientLockHeld($h) {
    if (-not $h) { return $false }
    $age = [DateTimeOffset]::UtcNow - [DateTimeOffset]::Parse($h.started)
    if ($age.TotalHours -ge $ClientLockStaleHours) { return $false }
    if ($h.pid -and $h.pid -gt 0 -and -not (Get-Process -Id $h.pid -ErrorAction SilentlyContinue)) { return $false }
    if ($h.background) {
        $procs = @(Get-CimInstance Win32_Process -Filter "Name='cargo.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.CommandLine -match 'benilla|twilights-eve\b|--workspace' -or $_.CommandLine -notmatch '\s-p\s' })
        return $procs.Count -gt 0
    }
    return $true
}

# Waits up to $WaitMinutes for the lock. Returns $true once $Holder has it (or already had it).
function Enter-ClientLock([string]$Holder, [int]$OwnerPid = 0, [int]$WaitMinutes = 25) {
    $deadline = (Get-Date).AddMinutes($WaitMinutes)
    while ($true) {
        $h = Read-ClientLock
        if ($h -and $h.holder -eq $Holder) { return $true }
        if (-not (Test-ClientLockHeld $h)) {
            Write-ClientLock $Holder $OwnerPid (Get-Date).ToUniversalTime().ToString('o') $false
            Start-Sleep -Milliseconds 200
            $again = Read-ClientLock
            if ($again -and $again.holder -eq $Holder) { return $true }
        }
        if ((Get-Date) -ge $deadline) { return $false }
        Start-Sleep -Seconds 10
    }
}

# Releases the lock if $Holder has it. -Background keeps it, marked, while the build runs on.
function Exit-ClientLock([string]$Holder, [switch]$Background) {
    $h = Read-ClientLock
    if (-not ($h -and $h.holder -eq $Holder)) { return }
    if ($Background) { Write-ClientLock $h.holder 0 $h.started $true }
    else { Remove-Item $ClientLockPath -Force -ErrorAction SilentlyContinue }
}

function Get-ClientLockHolder {
    $h = Read-ClientLock
    if (Test-ClientLockHeld $h) { return $h } else { return $null }
}
