<#
.SYNOPSIS
Work through GitHub issues labelled ready-for-agent with no one at the keyboard.

One fresh `claude -p` session per ticket, Opus, dollar-capped. A ticket whose "Blocked by" names
an open issue is skipped. After each session the script re-runs the gate (cargo check on every
crate the commits touched, realm tests when the realm or data changed) and resets main to the
pre-ticket commit if it fails. A spec whose last ticket closes is closed with `unplayed`. A dirty
tree is stashed for the night and put back at the end; main is pushed once the tickets are done.
Rules the session follows: docs/agents/unattended.md. Logs: .claude/overnight/<timestamp>/, kept
14 days.

.EXAMPLE
.\scripts\overnight.ps1                   # every open ready-for-agent issue, oldest first
.\scripts\overnight.ps1 -StopAt 07:00     # start no new ticket after 7am
.\scripts\overnight.ps1 -Issues 71,72     # only these, in this order
.\scripts\overnight.ps1 -DryRun           # show the queue and exit
.\scripts\overnight.ps1 -BudgetUsd 0 -AllowAgents   # no dollar cap, subagents allowed
#>
param(
    [int[]]$Issues,
    [string]$Label = "ready-for-agent",
    [string]$Model = "claude-opus-5-5",
    [double]$BudgetUsd = 8.0,   # 0 means no cap
    [string]$StopAt,
    [int]$Max = 30,
    [switch]$AllowAgents,       # let the sessions spawn subagents (the Agent tool) tonight
    [switch]$DryRun
)

$ErrorActionPreference = "Continue"
$Repo = "dmavs412/twilights-eve-wow"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

# Headless claude keys workspace trust on "C:/Users/..." (uppercase drive, forward slashes),
# a different entry from the "c:/..." one VS Code trusted. Untrusted means the allowlist in
# .claude/settings.json is ignored and every ticket stalls on a denied Bash call.
function Test-Trusted {
    $cfg = Join-Path $env:USERPROFILE ".claude.json"
    if (-not (Test-Path $cfg)) { return $false }
    $key = ($Root -replace '\\', '/')
    $key = $key.Substring(0, 1).ToUpper() + $key.Substring(1)
    # ConvertFrom-Json is case-insensitive on keys and throws on "c:/..." next to "C:/...";
    # JavaScriptSerializer keeps both.
    Add-Type -AssemblyName System.Web.Extensions
    $ser = New-Object System.Web.Script.Serialization.JavaScriptSerializer
    $ser.MaxJsonLength = [int]::MaxValue
    $d = $ser.DeserializeObject((Get-Content $cfg -Raw))
    $projects = $d["projects"]
    if ($null -eq $projects -or -not $projects.ContainsKey($key)) { return $false }
    return [bool]$projects[$key]["hasTrustDialogAccepted"]
}

function Find-Claude {
    $cmd = Get-Command claude -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $ext = Get-ChildItem "$env:USERPROFILE\.vscode\extensions" -Directory -Filter "anthropic.claude-code-*" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($ext) {
        $p = Join-Path $ext.FullName "resources\native-binary\claude.exe"
        if (Test-Path $p) { return $p }
    }
    throw "claude CLI not found on PATH or inside the VS Code extension"
}

function Write-Log([string]$msg) {
    $line = "[{0}] {1}" -f (Get-Date -Format "HH:mm:ss"), $msg
    Write-Host $line
    Add-Content -Path $script:RunLog -Value $line -Encoding utf8
}

# Runs cargo via cmd so stderr merges cleanly under PowerShell 5.1. Returns @{ok; what; out}.
function Invoke-Gate([string]$Before, [string]$After) {
    $files = @(git diff --name-only $Before $After)
    # A crate the commits deleted has nothing left to check.
    $crates = @($files | ForEach-Object { if ($_ -match '^crates/([^/]+)/') { $Matches[1] } } | Sort-Object -Unique |
        Where-Object { Test-Path (Join-Path $Root "crates\$_\Cargo.toml") })
    $realm = @($files | Where-Object { $_ -match '^(crates/twilight-realm/|data/|tools/)' }).Count -gt 0
    $ran = @()
    foreach ($c in $crates) {
        Write-Log "gate: cargo check -p $c"
        $out = cmd /c "cargo check -q -p $c 2>&1"
        $ran += "cargo check -p $c"
        if ($LASTEXITCODE -ne 0) {
            return @{ ok = $false; what = "cargo check -p $c"; out = (($out | Select-Object -Last 60) -join "`n") }
        }
    }
    if ($realm) {
        Write-Log "gate: cargo test -p twilight-realm"
        $out = cmd /c "cargo test -q -p twilight-realm 2>&1"
        $ran += "cargo test -p twilight-realm"
        if ($LASTEXITCODE -ne 0) {
            return @{ ok = $false; what = "cargo test -p twilight-realm"; out = (($out | Select-Object -Last 60) -join "`n") }
        }
    }
    if (@($files | Where-Object { $_ -match '^crates/twilight-realm/content/' }).Count -gt 0) {
        Write-Log "gate: ground_check.py"
        $out = cmd /c "`"$HOME\.local\bin\python3.15`" tools\twilights-eve\ground_check.py 2>&1"
        $ran += "ground_check.py"
        if ($LASTEXITCODE -ne 0) {
            return @{ ok = $false; what = "ground_check.py"; out = (($out | Select-Object -Last 60) -join "`n") }
        }
    }
    if ($ran.Count -eq 0) { $ran = @("no crate touched, no cargo gate") }
    return @{ ok = $true; what = ($ran -join "; "); out = "" }
}

$WorkflowLabels = @("ready-for-agent", "blocked", "needs-triage", "needs-info", $Label)

# A closed issue keeps no workflow label (docs/agents/triage-labels.md).
function Remove-WorkflowLabels([int]$n) {
    $have = @(((gh issue view -R $Repo $n --json labels) -join "`n" | ConvertFrom-Json).labels | ForEach-Object { $_.name })
    $drop = @($have | Where-Object { $WorkflowLabels -contains $_ } | Sort-Object -Unique)
    if ($drop.Count) { gh issue edit -R $Repo $n --remove-label ($drop -join ",") | Out-Null }
}

# The issues a ticket waits for: each "Blocked by" sentence, the list under a "Blocked by"
# heading, and GitHub's own dependency links.
function Get-Blockers([int]$n) {
    $body = (gh issue view -R $Repo $n --json body --jq .body) -join "`n"
    $text = @([regex]::Matches($body, '(?i)blocked by:?([^.\n]*)') | ForEach-Object { $_.Groups[1].Value })
    $heading = [regex]::Match($body, '(?ims)^#+\s*blocked by\s*$(.*?)(?=^#|\z)')
    if ($heading.Success) { $text += $heading.Groups[1].Value }
    $nums = @($text | ForEach-Object { [regex]::Matches($_, '#(\d+)') | ForEach-Object { [int]$_.Groups[1].Value } })
    $nums += @(gh api "repos/$Repo/issues/$n/dependencies/blocked_by" --jq '.[].number' 2>$null | ForEach-Object { [int]$_ })
    return @($nums | Where-Object { $_ -ne $n } | Sort-Object -Unique)
}

# A spec closes with `unplayed` once its last ticket closes (CLAUDE.md). The spec is the ticket's
# GitHub parent, or else the "(#N)" that ends its title. Returns the spec's number, or 0.
function Close-SpecIfDone([int]$n, [string]$title) {
    # No spaces or double quotes in the query: PowerShell 5.1 mangles both on the way to gh.
    $q = 'query($o:String!,$r:String!,$n:Int!){repository(owner:$o,name:$r){issue(number:$n){parent{number}}}}'
    $res = (gh api graphql -f query=$q -F o=dmavs412 -F r=twilights-eve-wow -F n=$n) -join "`n" | ConvertFrom-Json
    $spec = 0
    if ($res.data.repository.issue.parent) { $spec = [int]$res.data.repository.issue.parent.number }
    elseif ($title -match '\(#(\d+)\)\s*$') { $spec = [int]$Matches[1] }
    if (-not $spec) { return 0 }
    $s = (gh issue view -R $Repo $spec --json state,title) -join "`n" | ConvertFrom-Json
    if ($s.state -ne "OPEN" -or $s.title -eq "Overnight log") { return 0 }
    $subs = @((gh api "repos/$Repo/issues/$spec/sub_issues" --jq '.[].state' 2>$null) | Where-Object { $_ -eq "open" })
    $open = @(((gh issue list -R $Repo --state open --limit 200 --json number,title) -join "`n" | ConvertFrom-Json) |
        Where-Object { $_.number -ne $spec -and $_.title -match "\(#$spec\)" })
    if ($subs.Count -or $open.Count) { return 0 }
    Remove-WorkflowLabels $spec
    gh issue edit -R $Repo $spec --add-label unplayed | Out-Null
    gh issue close -R $Repo $spec --comment "Every ticket has landed, the last one #$n. Closed with ``unplayed`` by the overnight run; your play note removes the label." | Out-Null
    return $spec
}

function Set-Blocked([int]$n, [string]$why) {
    $tmp = Join-Path $script:LogDir "$n.blocked.md"
    $why | Set-Content -Path $tmp -Encoding utf8
    gh issue reopen -R $Repo $n 2>$null | Out-Null
    gh issue edit -R $Repo $n --add-label blocked --remove-label ready-for-agent | Out-Null
    gh issue comment -R $Repo $n --body-file $tmp | Out-Null
}

# ---- preflight -------------------------------------------------------------------------------
$claude = Find-Claude
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($branch -ne "main") { throw "on branch '$branch', not main" }
$dirtyAtStart = @(git status --porcelain).Count -gt 0
gh auth status 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { throw "gh is not authenticated" }
if (-not (Test-Trusted)) {
    throw "workspace not trusted for headless runs: set projects[`"C:/Users/davis/code/twilights-eve-wow`"].hasTrustDialogAccepted = true in ~/.claude.json"
}

$stamp = Get-Date -Format "yyyy-MM-dd_HHmm"
$script:LogDir = Join-Path $Root ".claude\overnight\$stamp"
New-Item -ItemType Directory -Force $script:LogDir | Out-Null
$script:RunLog = Join-Path $script:LogDir "run.log"
$summary = Join-Path $script:LogDir "summary.md"
"| issue | title | result | commits | cost |`n|---|---|---|---|---|" | Set-Content $summary -Encoding utf8

$stopTime = $null
if ($StopAt) {
    $stopTime = [datetime]::Parse($StopAt)
    if ($stopTime -lt (Get-Date)) { $stopTime = $stopTime.AddDays(1) }
}

# ---- queue -----------------------------------------------------------------------------------
if ($Issues) {
    $queue = @($Issues | ForEach-Object {
        $t = gh issue view -R $Repo $_ --json title --jq .title
        @{ number = $_; title = $t }
    })
} else {
    # Filter here, not in --jq: PowerShell 5.1 strips the double quotes a jq string needs.
    $raw = (gh issue list -R $Repo --state open --label $Label --limit 100 --json number,title,labels) -join "`n" | ConvertFrom-Json
    # Bugs first, oldest first; then everything else, oldest first.
    $queue = @($raw |
        Where-Object { @($_.labels | Where-Object { $_.name -eq "blocked" }).Count -eq 0 } |
        Sort-Object @{ e = { if (@($_.labels | Where-Object { $_.name -eq "bug" }).Count -gt 0) { 0 } else { 1 } } }, @{ e = { [int]$_.number } } |
        ForEach-Object { @{ number = [int]$_.number; title = $_.title } })
}
if ($queue.Count -gt $Max) { $queue = $queue[0..($Max - 1)] }

Write-Log ("claude: {0}" -f $claude)
Write-Log ("queue: {0} ticket(s): {1}" -f $queue.Count, (($queue | ForEach-Object { "#" + $_.number }) -join " "))
if ($DryRun) { $queue | ForEach-Object { "#{0}  {1}" -f $_.number, $_.title }; exit 0 }
if ($queue.Count -eq 0) { Write-Log "nothing to do"; exit 0 }

# Hold the build lock all night (same 2 h stale rule as scripts/build-lock.ps1). The sessions
# below inherit CLAUDE_BUILD_LOCK_HOLDER, so their own cargo hook lets them through.
$lock = Join-Path $Root "target\.claude-build.lock"
if (Test-Path $lock) {
    $holder = Get-Content $lock -Raw | ConvertFrom-Json
    $age = [DateTimeOffset]::UtcNow - [DateTimeOffset]::Parse($holder.started)
    if ($age.TotalHours -lt 2) { throw ("build lock held by {0} for {1:N0} min" -f $holder.session, $age.TotalMinutes) }
}
$env:CLAUDE_BUILD_LOCK_HOLDER = "overnight-$PID"
New-Item -ItemType Directory -Force (Split-Path $lock) | Out-Null
# Re-stamped before each ticket, so a long night never looks stale.
function Set-Lock { @{ session = $env:CLAUDE_BUILD_LOCK_HOLDER; started = (Get-Date).ToUniversalTime().ToString('o') } | ConvertTo-Json -Compress | Set-Content $lock -Encoding utf8 }
Set-Lock

# Someone's uncommitted work is set aside for the night and put back at the end, so it never
# sits in a stash nobody pops.
$stashName = $null
if ($dirtyAtStart) {
    $stashName = "overnight $stamp"
    git stash push -u -q -m $stashName | Out-Null
    if (@(git status --porcelain).Count -gt 0) { throw "could not stash the working tree" }
    Write-Log "stashed the dirty tree as '$stashName'"
}

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

$startSha = (git rev-parse HEAD).Trim()
$failures = @()
$specsClosed = @()
$settings = Join-Path $Root "scripts\overnight-settings.json"
$rules = Join-Path $Root "docs\agents\unattended.md"

# ---- tickets ---------------------------------------------------------------------------------
foreach ($t in $queue) {
    $n = $t.number
    $title = ($t.title -replace '"', "'")
    if ($stopTime -and (Get-Date) -ge $stopTime) { Write-Log "stop time $StopAt reached before #$n"; break }

    # A blocker that blocked earlier tonight is open again, so its followers wait for the next night.
    $waiting = @(Get-Blockers $n | Where-Object { (gh issue view -R $Repo $_ --json state --jq .state) -eq "OPEN" })
    if ($waiting.Count) {
        $names = ($waiting | ForEach-Object { "#$_" }) -join ", "
        Write-Log "#${n}: skipped, blocked by $names"
        Add-Content $summary ("| #{0} | {1} | skipped: blocked by {2} | | |" -f $n, $title, $names) -Encoding utf8
        continue
    }

    Write-Log "==== #$n $title"
    Set-Lock
    $before = (git rev-parse HEAD).Trim()
    $prompt = "Unattended run, nobody is at the keyboard. Read $rules first and follow it. " +
              "Then implement GitHub issue #$n ($title) on $Repo."
    if ($AllowAgents) {
        $prompt += " Tonight the Agent tool is allowed. " +
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
        $diffFile = Join-Path $script:LogDir "$n.discarded.diff"
        # Stage everything first so the diff keeps staged edits and new files, then reset:
        # `git checkout -- .` left staged work behind for the next ticket to trip on.
        git add -A | Out-Null
        git diff --cached --binary | Set-Content $diffFile -Encoding utf8
        git reset --hard -q | Out-Null
        git clean -fdq | Out-Null
        Write-Log "discarded $($dirty.Count) uncommitted path(s); see $n.discarded.diff"
    }

    $after = (git rev-parse HEAD).Trim()
    if ($after -eq $before) {
        $labels = ((gh issue view -R $Repo $n --json labels) -join "`n" | ConvertFrom-Json).labels | ForEach-Object { $_.name }
        if ($labels -notcontains "blocked") {
            $tail = $text.Substring([Math]::Max(0, $text.Length - 1500))
            Set-Blocked $n "Overnight run produced no commit (exit $exit, $turns turns). Last words of the session:`n`n$tail"
        }
        $failures += "#${n}: blocked, no commit"
        Write-Log "#${n}: no commit; labelled blocked"
        Add-Content $summary ("| #{0} | {1} | blocked, no commit | | {2} |" -f $n, $title, $cost) -Encoding utf8
        continue
    }

    $gate = Invoke-Gate $before $after
    if (-not $gate.ok) {
        $range = "$($before.Substring(0,7))..$($after.Substring(0,7))"
        git reset --hard $before | Out-Null
        $fence = '```'
        Set-Blocked $n "Overnight run: commits $range failed ``$($gate.what)`` and were rolled back.`n`n${fence}text`n$($gate.out)`n${fence}"
        $failures += "#${n}: rolled back, $($gate.what) failed"
        Write-Log "#${n}: gate failed ($($gate.what)); rolled back to $($before.Substring(0,7))"
        Add-Content $summary ("| #{0} | {1} | rolled back: {2} | | {3} |" -f $n, $title, $gate.what, $cost) -Encoding utf8
        continue
    }

    $shas = (git log --format=%h "$before..$after") -join " "
    $state = gh issue view -R $Repo $n --json state --jq .state
    if ($state -eq "OPEN") {
        gh issue close -R $Repo $n --comment "Closed by the overnight run. Commits $shas passed: $($gate.what)." | Out-Null
    }
    Remove-WorkflowLabels $n
    $spec = Close-SpecIfDone $n $t.title
    if ($spec) { $specsClosed += $spec; Write-Log "#${n} was the last ticket of spec #${spec}: closed, unplayed" }
    Write-Log "#${n}: done ($shas); gate passed: $($gate.what)"
    Add-Content $summary ("| #{0} | {1} | done | {2} | {3} |" -f $n, $title, $shas, $cost) -Encoding utf8
}

# ---- morning report --------------------------------------------------------------------------
Write-Log "==== finished"
Remove-Item $lock -Force -ErrorAction SilentlyContinue

# Main goes up once a night. Every commit on it passed its gate: a failed gate was rolled back.
if ((git rev-parse HEAD).Trim() -ne $startSha) {
    git push -q origin main 2>$null
    if ($LASTEXITCODE -eq 0) { Write-Log "pushed main" } else { $failures += "git push origin main failed" }
}

$published = "none (no commit landed)"
if ((git rev-parse HEAD).Trim() -ne $startSha) {
    Write-Log "publishing a play copy"
    # `*>&1`: publish-play reports through Write-Host, which skips the pipeline, so without it a
    # refusal or a failed build reached run.log as nothing at all.
    & (Join-Path $PSScriptRoot "publish-play.ps1") *>&1 | ForEach-Object { Write-Log "$_" }
    $published = if ($LASTEXITCODE -eq 0) { "play/" + (git rev-parse --short HEAD).Trim() } else { "failed, see run.log" }
    if ($LASTEXITCODE -ne 0) { $failures += "publish-play.ps1 failed" }
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
        # No quotes inside the search: PowerShell 5.1 drops them on the way to gh, which then
        # prints its usage and finds nothing. The next line keeps only the exact title.
        $open = (gh issue list -R $Repo --state open --search "in:title $title" --json number,title) -join "`n" | ConvertFrom-Json
        $issue = @($open | Where-Object { $_.title -eq $title }) | Select-Object -First 1
        if ($issue) { $no = $issue.number } else {
            $body = "The overnight smoke run ($published, ``scripts/smoke.ps1``) cast $($b.name) ($($b.spell_id)) as a $($b.calling) on a Test Grounds dummy and the client answered:`n`n> $($b.error)`n`nReport: ``$smokeReport``."
            $url = gh issue create -R $Repo --title $title --label bug --body $body
            $no = ($url -split '/')[-1]
        }
        $smoke += "- $($b.calling) $($b.name) ($($b.spell_id)): $($b.error) (#$no)"
    }
    $late = @($rows | Where-Object { $_.timed_out_at })
    foreach ($t in $late) { $smoke += "- timed out at Calling $($t.timed_out_at)" }
    if (-not $smoke.Count) { $smoke = @("- clean, $($casts.Count) spells") }
}

# The tree stashed at the start goes back. If it no longer applies over tonight's commits, the
# stash stays and the log's first line says so.
$stashKept = $null
if ($stashName) {
    $entry = @(git stash list --format="%gd %s" | Where-Object { $_ -like "* $stashName" }) | Select-Object -First 1
    $ref = ($entry -split ' ')[0]
    git stash apply -q $ref 2>$null
    if ($LASTEXITCODE -eq 0) {
        git stash drop -q $ref | Out-Null
        Write-Log "put back the tree stashed at the start"
    } else {
        git reset --hard -q | Out-Null
        git clean -fdq | Out-Null
        $stashKept = $ref
        $failures += "the stash '$stashName' did not apply; it is kept"
        Write-Log "stash '$stashName' did not apply over tonight's commits; kept as $ref"
    }
}

# One comment per night on the pinned "Overnight log" issue; format in docs/agents/unattended.md.
$assumed = @(git log --format=%B "$startSha..HEAD" | Where-Object { $_ -match '^Assumed:' })
# Built with `+=`, which flattens: a nested array inside `@(a, b, c)` stays one element, and
# `-join` then prints it as `System.Object[]`.
$lines = @("## Night of $stamp")
if ($stashKept) {
    $lines += "**Your uncommitted work is still stashed** as ``$stashName``: it did not apply over tonight's commits. Run ``git stash apply`` on it by hand."
}
$lines += @("", "### Tickets")
$lines += @(Get-Content $summary)
$lines += @("", "### Assumptions")
$lines += $(if ($assumed.Count) { @($assumed | ForEach-Object { "- $_" }) } else { @("- none") })
$lines += @("", "### Specs closed, unplayed")
$lines += $(if ($specsClosed.Count) { @($specsClosed | ForEach-Object { "- #$_" }) } else { @("- none") })
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

Get-Content $summary | Write-Host
Write-Host ""
Write-Host "commits since start:"
git log --oneline "$startSha..HEAD"
Write-Host ""
Write-Host "logs: $script:LogDir"
