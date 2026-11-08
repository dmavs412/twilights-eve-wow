<#
.SYNOPSIS
Run the builders: work through GitHub issues labelled ready-for-agent with no one at the keyboard,
day or night.

Starts -Workers builders (scripts/worker.ps1), each in its own worktree beside this checkout
(..\twilights-eve-w1, -w2, ...) with its own target folder. Each builder claims one ticket at a
time, runs a fresh `claude -p` session on it, gates the commits and lands them on origin/main.
Realm and data builds run in parallel; a client build takes the machine-wide client lock
(scripts/client-lock.ps1). This checkout is never touched while they run, so Davis and the
interactive chats keep working in it. When every builder stops, this checkout fast-forwards to
origin/main, a play copy is published and smoke-run, and one morning log goes on the pinned
"Overnight log" issue. Rules the sessions follow: docs/agents/unattended.md. Logs:
.claude/overnight/<timestamp>/, kept 14 days.

.EXAMPLE
.\scripts\overnight.ps1                        # two builders until the queue is empty
.\scripts\overnight.ps1 -StopAt 07:00          # start no new ticket after 7am
.\scripts\overnight.ps1 -LingerMinutes 10 -StopAt 23:00   # day mode: wait for tickets grills file
.\scripts\overnight.ps1 -Issues 71,72 -Workers 1          # only these, in this order
.\scripts\overnight.ps1 -DryRun                # show the queue and exit
.\scripts\overnight.ps1 -BudgetUsd 0 -AllowAgents         # no dollar cap, subagents allowed
#>
param(
    [int]$Workers = 2,
    [int[]]$Issues,
    [string]$Label = "ready-for-agent",
    [string]$Model = "claude-opus-5-5",
    [double]$BudgetUsd = 8.0,   # per ticket; 0 means no cap
    [string]$StopAt,
    [int]$Max = 30,             # per builder
    [int]$LingerMinutes = 0,
    [switch]$AllowAgents,
    [switch]$NoPublish,
    [switch]$DryRun
)

$ErrorActionPreference = "Continue"
$Repo = "dmavs412/twilights-eve-wow"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root
. (Join-Path $PSScriptRoot 'overnight-lib.ps1')

$stamp = Get-Date -Format "yyyy-MM-dd_HHmm"
$script:LogDir = Join-Path $Root ".claude\overnight\$stamp"
New-Item -ItemType Directory -Force $script:LogDir | Out-Null
$script:RunLog = Join-Path $script:LogDir "run.log"

# ---- preflight -------------------------------------------------------------------------------
$claude = Find-Claude
gh auth status 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { throw "gh is not authenticated" }

# One fleet at a time: a second would hand out the same worktrees.
$fleetLock = Join-Path $env:LOCALAPPDATA 'twilights-eve\fleet.lock'
New-Item -ItemType Directory -Force (Split-Path $fleetLock) | Out-Null
if (Test-Path $fleetLock) {
    $other = [int](Get-Content $fleetLock -Raw)
    # After a reboot Windows hands old pids to other programs, so the pid must still be an
    # overnight.ps1, not merely alive.
    $proc = Get-CimInstance Win32_Process -Filter "ProcessId=$other" -ErrorAction SilentlyContinue
    if ($other -ne $PID -and $proc -and $proc.CommandLine -match 'overnight\.ps1') { throw "another overnight.ps1 runs (pid $other)" }
}
"$PID" | Set-Content $fleetLock -Encoding ascii

$parent = Split-Path $Root -Parent
$trees = @(1..$Workers | ForEach-Object { @{ name = "w$_"; path = (Join-Path $parent "twilights-eve-w$_") } })
$untrusted = @($trees | Where-Object { -not (Test-Trusted $_.path) } | ForEach-Object { Get-TrustKey $_.path })
if ($untrusted.Count) {
    Remove-Item $fleetLock -Force
    throw ("workspace not trusted for headless runs: set projects[`"{0}`"].hasTrustDialogAccepted = true in ~/.claude.json" -f ($untrusted -join '", "'))
}

if ($DryRun) {
    $q = Get-Queue $Label $Issues
    Write-Log ("queue: {0} ticket(s)" -f $q.Count)
    foreach ($t in $q) {
        $w = @(Get-OpenBlockers $t.number)
        $note = if ($w.Count) { "  (waits for " + (($w | ForEach-Object { "#$_" }) -join ", ") + ")" } else { "" }
        "#{0}  {1}{2}" -f $t.number, $t.title, $note
    }
    Remove-Item $fleetLock -Force
    exit 0
}

# Claims left by a run that died: no builder runs now, so every claim and in-progress label is stale.
Remove-Item $ClaimDir -Recurse -Force -ErrorAction SilentlyContinue
$stale = (gh issue list -R $Repo --state open --label in-progress --limit 100 --json number) -join "`n" | ConvertFrom-Json
foreach ($s in @($stale)) { if ($s) { gh issue edit -R $Repo $s.number --remove-label in-progress | Out-Null; Write-Log "cleared a stale claim on #$($s.number)" } }

# Run folders are named by date; those older than 14 days go.
$cutoff = (Get-Date).Date.AddDays(-14)
foreach ($dir in @(".claude\overnight", ".claude\smoke")) {
    Get-ChildItem (Join-Path $Root $dir) -Directory -ErrorAction SilentlyContinue | Where-Object {
        $d = [datetime]::MinValue
        $_.Name.Length -ge 10 -and $_.FullName -ne $script:LogDir -and
            [datetime]::TryParseExact($_.Name.Substring(0, 10), 'yyyy-MM-dd', [Globalization.CultureInfo]::InvariantCulture, 'None', [ref]$d) -and
            $d -lt $cutoff
    } | ForEach-Object {
        Remove-Item $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
        Write-Log "pruned $dir\$($_.Name)"
    }
}

# ---- worktrees -------------------------------------------------------------------------------
git fetch -q origin
$startSha = (git rev-parse origin/main).Trim()
foreach ($t in $trees) {
    if (-not (Test-Path (Join-Path $t.path ".git"))) {
        Write-Log "adding worktree $($t.path)"
        git worktree add -q -f -B "worker/$($t.name)" $t.path origin/main | Out-Null
        if ($LASTEXITCODE -ne 0) { Remove-Item $fleetLock -Force; throw "git worktree add $($t.path) failed" }
    }
}

# ---- builders --------------------------------------------------------------------------------
Write-Log ("starting {0} builder(s) from origin/main {1}" -f $Workers, $startSha.Substring(0, 8))
$procs = @()
foreach ($t in $trees) {
    $wargs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Join-Path $PSScriptRoot "worker.ps1"),
               "-Name", $t.name, "-Worktree", $t.path, "-RunDir", $script:LogDir,
               "-Label", $Label, "-Model", $Model, "-BudgetUsd", "$BudgetUsd", "-Max", "$Max",
               "-LingerMinutes", "$LingerMinutes")
    if ($StopAt) { $wargs += @("-StopAt", $StopAt) }
    if ($Issues) { $wargs += @("-Only", ($Issues -join ",")) }
    if ($AllowAgents) { $wargs += "-AllowAgents" }
    $procs += Start-Process powershell -ArgumentList $wargs -WindowStyle Minimized -PassThru
    Write-Log "builder $($t.name) started (pid $($procs[-1].Id)); log $($script:LogDir)\$($t.name)\run.log"
    # Staggered, so the first claims settle before the next builder reads the queue.
    Start-Sleep -Seconds 20
}
$procs | Wait-Process
Write-Log "==== every builder stopped"

# ---- this checkout catches up ----------------------------------------------------------------
$failures = @()
foreach ($t in $trees) { $failures += @(Get-Content (Join-Path $script:LogDir "$($t.name).failures.txt") -ErrorAction SilentlyContinue) }
$specsClosed = @()
foreach ($t in $trees) { $specsClosed += @(Get-Content (Join-Path $script:LogDir "$($t.name).specs.txt") -ErrorAction SilentlyContinue) }
git fetch -q origin
$endSha = (git rev-parse origin/main).Trim()
$landed = $endSha -ne $startSha

$published = "none (no commit landed)"
if ($landed) {
    $branch = (git rev-parse --abbrev-ref HEAD).Trim()
    git pull -q --ff-only origin main 2>$null
    if ($branch -ne "main" -or $LASTEXITCODE -ne 0) {
        $failures += "this checkout could not fast-forward to origin/main (branch $branch); run git pull --rebase"
        $published = "none (this checkout is behind)"
    } elseif ($NoPublish) {
        $published = "none (-NoPublish)"
    } else {
        Write-Log "publishing a play copy"
        # `*>&1`: publish-play reports through Write-Host, which skips the pipeline.
        & (Join-Path $PSScriptRoot "publish-play.ps1") *>&1 | ForEach-Object { Write-Log "$_" }
        $published = if ($LASTEXITCODE -eq 0) { "play/" + (git rev-parse --short HEAD).Trim() } else { "failed, see run.log" }
        if ($LASTEXITCODE -ne 0) { $failures += "publish-play.ps1 failed" }
    }
}

# The smoke run against the copy just published (#200). Report only: one bug per failing spell,
# never a reset or a block.
$smoke = @("- not run")
if ($published -like "play/*") {
    Write-Log "smoke run against $published"
    & (Join-Path $PSScriptRoot "smoke.ps1") -Play (Join-Path $Root $published) *>&1 | ForEach-Object { Write-Log "$_" }
    $smokeDir = Get-ChildItem (Join-Path $Root ".claude\smoke") -Directory | Sort-Object Name | Select-Object -Last 1
    $smokeReport = Join-Path $smokeDir.FullName "smoke-report.jsonl"
    $rows = if (Test-Path $smokeReport) { @(Get-Content $smokeReport | ForEach-Object { $_ | ConvertFrom-Json }) } else { @() }
    $casts = @($rows | Where-Object { $null -ne $_.spell_id })
    $bad = @($casts | Where-Object { -not $_.ok })
    $smoke = @()
    foreach ($b in $bad) {
        $title = "Smoke: $($b.calling) $($b.name) ($($b.spell_id)) fails"
        # No quotes inside the search: PowerShell 5.1 drops them on the way to gh.
        $open = (gh issue list -R $Repo --state open --search "in:title $title" --json number,title) -join "`n" | ConvertFrom-Json
        $issue = @($open | Where-Object { $_.title -eq $title }) | Select-Object -First 1
        if ($issue) { $no = $issue.number } else {
            $body = "The smoke run ($published, ``scripts/smoke.ps1``) cast $($b.name) ($($b.spell_id)) as a $($b.calling) on a Test Grounds dummy and the client answered:`n`n> $($b.error)`n`nReport: ``$smokeReport``."
            $url = gh issue create -R $Repo --title $title --label bug --body $body
            $no = ($url -split '/')[-1]
        }
        $smoke += "- $($b.calling) $($b.name) ($($b.spell_id)): $($b.error) (#$no)"
    }
    $late = @($rows | Where-Object { $_.timed_out_at })
    foreach ($l in $late) { $smoke += "- timed out at Calling $($l.timed_out_at)" }
    if (-not $smoke.Count) { $smoke = @("- clean, $($casts.Count) spells") }
}

# ---- morning log -----------------------------------------------------------------------------
# One comment per run on the pinned "Overnight log" issue; format in docs/agents/unattended.md.
$assumed = @(git log --format=%B "$startSha..$endSha" | Where-Object { $_ -match '^Assumed:' })
# Built with `+=`, which flattens: a nested array inside `@(a, b, c)` stays one element.
$lines = @("## Night of $stamp ($Workers builders)", "", "### Tickets", "| issue | title | result | commits | cost |", "|---|---|---|---|---|")
foreach ($t in $trees) { $lines += @(Get-Content (Join-Path $script:LogDir "$($t.name).summary.md") -ErrorAction SilentlyContinue) }
$lines += @("", "### Assumptions")
$lines += $(if ($assumed.Count) { @($assumed | ForEach-Object { "- $_" }) } else { @("- none") })
$lines += @("", "### Specs closed, unplayed")
$lines += $(if ($specsClosed.Count) { @($specsClosed | Sort-Object -Unique | ForEach-Object { "- #$_" }) } else { @("- none") })
$lines += @("", "### Play copy", "- $published", "", "### Smoke")
$lines += $smoke
$lines += @("", "### Failures")
$lines += $(if ($failures.Count) { @($failures | ForEach-Object { "- $_" }) } else { @("- none") })
$body = $lines -join "`n"
$bodyFile = Join-Path $script:LogDir "overnight-log.md"
# No BOM: PowerShell 5.1's `Set-Content -Encoding utf8` writes one, and it lands in the comment.
[IO.File]::WriteAllText($bodyFile, $body, (New-Object Text.UTF8Encoding $false))
$raw = (gh issue list -R $Repo --state open --search "in:title Overnight log" --json number,title) -join "`n" | ConvertFrom-Json
$logIssue = @($raw | Where-Object { $_.title -eq "Overnight log" }) | Select-Object -First 1
if ($logIssue) { $logNo = $logIssue.number } else {
    $url = gh issue create -R $Repo --title "Overnight log" --body "One comment per overnight run (scripts/overnight.ps1)."
    $logNo = ($url -split '/')[-1]
    gh issue pin -R $Repo $logNo | Out-Null
}
gh issue comment -R $Repo $logNo --body-file $bodyFile | Out-Null
Write-Log "morning log posted on #$logNo"
Remove-Item $fleetLock -Force -ErrorAction SilentlyContinue

Write-Host $body
Write-Host ""
Write-Host "logs: $script:LogDir"
