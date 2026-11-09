<#
.SYNOPSIS
One builder: take ready-for-agent tickets one at a time in this builder's own worktree and land
each on origin/main.

Started by overnight.ps1, one process per builder; not meant to be run by hand. Each ticket: reset
the worktree to origin/main, claim the ticket, run a fresh `claude -p` session, gate the commits,
rebase them onto origin/main (re-gating when main moved) and push. A rebase that conflicts puts
the ticket back in the queue once; a second conflict blocks it. Rules the session follows:
docs/agents/unattended.md.
#>
param(
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][string]$Worktree,
    [Parameter(Mandatory)][string]$RunDir,
    [string]$Label = "ready-for-agent",
    [string]$Model = "claude-opus-5-5",
    [double]$BudgetUsd = 8.0,
    [string]$StopAt,
    [int]$Max = 30,
    [int]$LingerMinutes = 0,    # poll an empty queue this often until StopAt; 0 means stop when empty
    [string]$Only,              # comma-separated issue numbers; -File passes every argument as a string
    [switch]$AllowAgents
)

$ErrorActionPreference = "Continue"
$OnlyList = [int[]]@(if ($Only) { $Only -split ',' | ForEach-Object { [int]$_.Trim() } })
$Repo = "dmavs412/twilights-eve-wow"
. (Join-Path $PSScriptRoot 'overnight-lib.ps1')

Set-Location $Worktree
$script:LogDir = Join-Path $RunDir $Name
New-Item -ItemType Directory -Force $script:LogDir | Out-Null
$script:RunLog = Join-Path $script:LogDir "run.log"
$summary = Join-Path $RunDir "$Name.summary.md"
$failFile = Join-Path $RunDir "$Name.failures.txt"
$specFile = Join-Path $RunDir "$Name.specs.txt"
foreach ($f in @($summary, $failFile, $specFile)) { New-Item -ItemType File -Force $f | Out-Null }

$stopTime = $null
if ($StopAt) {
    $stopTime = [datetime]::Parse($StopAt)
    if ($stopTime -lt (Get-Date)) { $stopTime = $stopTime.AddDays(1) }
}

$claude = Find-Claude
$holder = "worker-$Name-$PID"
# Hold this worktree's build lock for the whole run (same 2 h stale rule as build-lock.ps1). The
# sessions below inherit CLAUDE_BUILD_LOCK_HOLDER, so their cargo hook lets them through; a
# client build still waits for the machine-wide client lock.
$env:CLAUDE_BUILD_LOCK_HOLDER = $holder
$lock = Join-Path $Worktree "target\.claude-build.lock"
New-Item -ItemType Directory -Force (Split-Path $lock) | Out-Null
function Set-Lock { @{ session = $holder; started = (Get-Date).ToUniversalTime().ToString('o') } | ConvertTo-Json -Compress | Set-Content $lock -Encoding utf8 }
Set-Lock

function Add-Row([string]$row) { Add-Content $summary $row -Encoding utf8 }
function Add-Failure([string]$f) { Add-Content $failFile $f -Encoding utf8 }

# Resets the worktree to origin/main. The worktree belongs to this builder alone.
function Reset-Worktree {
    git fetch -q origin 2>$null
    git rebase --abort 2>$null | Out-Null
    git reset --hard -q origin/main | Out-Null
    git clean -fdq | Out-Null
}

# Rebases this ticket's commits onto origin/main and pushes. Returns @{ok; conflict; what; out; shas}.
function Invoke-Land {
    for ($try = 1; $try -le 4; $try++) {
        git fetch -q origin 2>$null
        $base = (git rev-parse origin/main).Trim()
        $mergeBase = (git merge-base HEAD origin/main).Trim()
        if ($mergeBase -ne $base) {
            git rebase -q origin/main 2>$null | Out-Null
            if ($LASTEXITCODE -ne 0) {
                $files = @(git diff --name-only --diff-filter=U) -join ", "
                git rebase --abort 2>$null | Out-Null
                return @{ ok = $false; conflict = $true; what = "rebase onto main conflicted on $files" }
            }
            # Main moved under us: the rebased commits need the gate again.
            $gate = Invoke-Gate $base (git rev-parse HEAD).Trim() $holder
            if (-not $gate.ok) { return @{ ok = $false; conflict = $false; what = "after rebase, $($gate.what) failed"; out = $gate.out } }
        }
        git push -q origin HEAD:main 2>$null
        if ($LASTEXITCODE -eq 0) {
            return @{ ok = $true; shas = ((git log --format=%h "$base..HEAD") -join " ") }
        }
        Write-Log "push refused (main moved); retry $try"
    }
    return @{ ok = $false; conflict = $true; what = "push to main refused four times" }
}

$settings = Join-Path $Worktree "scripts\overnight-settings.json"
$rules = Join-Path $Worktree "docs\agents\unattended.md"
$done = 0
$requeued = @{}
$skippedOnce = @{}
$finished = @{}

Write-Log "builder $Name in $Worktree (claude: $claude)"

# Warm this worktree's target before the first ticket. A session may not run a command past the
# Bash tool's 10 minutes, and a cold client build takes longer; once warm, an incremental client
# test fits. The client part waits for the machine-wide client lock.
Reset-Worktree
Write-Log "warming the realm test build"
cmd /c "cargo test -q -p twilight-realm --no-run 2>&1" | Out-Null
if (Enter-ClientLock $holder $PID 120) {
    Write-Log "warming the client test build"
    cmd /c "cargo test -q -p benilla-app --lib --no-run 2>&1" | Out-Null
    Exit-ClientLock $holder
    Write-Log "warm-up done"
} else {
    Write-Log "client lock busy for 2 h; skipped the client warm-up"
}

while ($true) {
    if ($stopTime -and (Get-Date) -ge $stopTime) { Write-Log "stop time $StopAt reached"; break }
    if ($done -ge $Max) { Write-Log "reached -Max $Max"; break }

    Reset-Worktree
    $t = $null
    foreach ($c in (Get-Queue $Label $OnlyList)) {
        $waiting = @(Get-OpenBlockers $c.number)
        if ($waiting.Count) {
            if (-not $skippedOnce.ContainsKey($c.number)) {
                $skippedOnce[$c.number] = $true
                $names = ($waiting | ForEach-Object { "#$_" }) -join ", "
                Write-Log "#$($c.number): waits for $names"
            }
            continue
        }
        # The list comes from GitHub's search index, which lags a minute or two behind: a ticket
        # closed just now can still show as open and ready (#633 was built twice). Skip what this
        # builder finished, and confirm the rest through a direct read before claiming.
        if ($finished.ContainsKey($c.number)) { continue }
        $live = (gh issue view -R $Repo $c.number --json state,labels) -join "`n" | ConvertFrom-Json
        $liveLabels = @($live.labels | ForEach-Object { $_.name })
        if ($live.state -ne "OPEN" -or $liveLabels -notcontains $Label -or $liveLabels -contains "in-progress" -or $liveLabels -contains "blocked") { continue }
        if (Request-Claim $c.number $Name) { $t = $c; break }
    }
    if (-not $t) {
        if ($LingerMinutes -gt 0 -and -not ($stopTime -and (Get-Date).AddMinutes($LingerMinutes) -ge $stopTime)) {
            Write-Log "queue empty; checking again in $LingerMinutes min"
            Start-Sleep -Seconds ($LingerMinutes * 60)
            continue
        }
        Write-Log "queue empty; stopping"
        break
    }

    $n = $t.number
    $title = ($t.title -replace '"', "'")
    Write-Log "==== #$n $title"
    Set-Lock
    $before = (git rev-parse HEAD).Trim()
    $prompt = "Unattended run, nobody is at the keyboard. Read $rules first and follow it. " +
              "You are builder $Name, working in your own worktree $Worktree; commit here and do not push, the script lands your commits on main. " +
              "Then implement GitHub issue #$n ($title) on $Repo."
    if ($AllowAgents) {
        $prompt += " This run the Agent tool is allowed. " +
                   "Use subagents for reading, searching and independent sub-tasks; only the main session runs cargo " +
                   "and commits, so two sessions never edit the same file."
    }
    $cargs = @("-p", $prompt, "--model", $Model, "--settings", $settings,
               "--permission-mode", "dontAsk", "--output-format", "json")
    if ($BudgetUsd -gt 0) { $cargs += @("--max-budget-usd", $BudgetUsd) }
    if ($AllowAgents) { $cargs += @("--allowedTools", "Agent") }
    $stderr = Join-Path $script:LogDir "$n.stderr.log"
    $json = & $claude @cargs 2> $stderr
    $exit = $LASTEXITCODE
    $done++

    $cost = "?"; $turns = "?"; $text = ($json -join "`n")
    try {
        $res = $text | ConvertFrom-Json
        if ($null -ne $res.total_cost_usd) { $cost = ("{0:N2}" -f $res.total_cost_usd) }
        if ($null -ne $res.num_turns) { $turns = $res.num_turns }
        if ($res.result) { $text = $res.result }
        if ($res.permission_denials) {
            $res.permission_denials | ConvertTo-Json -Depth 6 |
                Set-Content (Join-Path $script:LogDir "$n.denials.json") -Encoding utf8
        }
    } catch { }
    $text | Set-Content (Join-Path $script:LogDir "$n.log") -Encoding utf8
    Write-Log "session exit $exit, $turns turns, `$$cost"

    # The session must leave a clean tree: commits or nothing.
    $dirty = @(git status --porcelain)
    if ($dirty.Count -gt 0) {
        git add -A | Out-Null
        git diff --cached --binary | Set-Content (Join-Path $script:LogDir "$n.discarded.diff") -Encoding utf8
        git reset --hard -q | Out-Null
        git clean -fdq | Out-Null
        Write-Log "discarded $($dirty.Count) uncommitted path(s); see $n.discarded.diff"
    }

    $after = (git rev-parse HEAD).Trim()
    if ($after -eq $before) {
        $labels = ((gh issue view -R $Repo $n --json labels) -join "`n" | ConvertFrom-Json).labels | ForEach-Object { $_.name }
        if ($labels -notcontains "blocked") {
            $tail = $text.Substring([Math]::Max(0, $text.Length - 1500))
            Set-Blocked $n "Builder $Name produced no commit (exit $exit, $turns turns). Last words of the session:`n`n$tail"
        } else {
            gh issue edit -R $Repo $n --remove-label in-progress | Out-Null
        }
        Clear-Claim $n
        Add-Failure "#${n}: blocked, no commit"
        Write-Log "#${n}: no commit; labelled blocked"
        Add-Row ("| #{0} | {1} | blocked, no commit | | {2} |" -f $n, $title, $cost)
        continue
    }

    $gate = Invoke-Gate $before $after $holder
    $land = if ($gate.ok) { Invoke-Land } else { $null }
    if (-not $gate.ok -or -not $land.ok) {
        $why = if (-not $gate.ok) { $gate.what } else { $land.what }
        $out = if (-not $gate.ok) { $gate.out } else { $land.out }
        $range = "$($before.Substring(0,7))..$($after.Substring(0,7))"
        if ($gate.ok -and $land.conflict -and -not $requeued.ContainsKey($n)) {
            # The other builder landed something on the same files. Run the ticket again on the new main.
            $requeued[$n] = $true
            gh issue reopen -R $Repo $n 2>$null | Out-Null
            gh issue edit -R $Repo $n --remove-label in-progress --add-label $Label | Out-Null
            Clear-Claim $n
            Write-Log "#${n}: $why; back in the queue for one more try"
            Add-Row ("| #{0} | {1} | requeued: {2} | | {3} |" -f $n, $title, $why, $cost)
            continue
        }
        $fence = '```'
        Set-Blocked $n "Builder ${Name}: commits $range did not land: $why.`n`n${fence}text`n$out`n${fence}"
        Clear-Claim $n
        Add-Failure "#${n}: not landed, $why"
        Write-Log "#${n}: not landed ($why)"
        Add-Row ("| #{0} | {1} | not landed: {2} | | {3} |" -f $n, $title, $why, $cost)
        continue
    }

    $state = gh issue view -R $Repo $n --json state --jq .state
    if ($state -eq "OPEN") {
        gh issue close -R $Repo $n --comment "Closed by builder $Name. Commits $($land.shas) passed: $($gate.what), and landed on main." | Out-Null
    }
    Remove-WorkflowLabels $n
    Clear-Claim $n
    $finished[$n] = $true
    $spec = Close-SpecIfDone $n $t.title
    if ($spec) { Add-Content $specFile "$spec" -Encoding utf8; Write-Log "#${n} was the last ticket of spec #${spec}: closed, unplayed" }
    Write-Log "#${n}: landed ($($land.shas)); gate: $($gate.what)"
    Add-Row ("| #{0} | {1} | done ({2}) | {3} | {4} |" -f $n, $title, $Name, $land.shas, $cost)
}

Remove-Item $lock -Force -ErrorAction SilentlyContinue
Reset-Worktree
Write-Log "==== builder $Name finished"
