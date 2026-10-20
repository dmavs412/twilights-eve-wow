<#
.SYNOPSIS
Work through GitHub issues labelled ready-for-agent with no one at the keyboard.

One fresh `claude -p` session per ticket, Sonnet, dollar-capped. After each session the script
re-runs the gate (cargo check on every crate the commits touched, realm tests when the realm or
data changed) and resets main to the pre-ticket commit if it fails. Rules the session follows:
docs/agents/unattended.md. Logs: .claude/overnight/<timestamp>/.

.EXAMPLE
.\scripts\overnight.ps1                   # every open ready-for-agent issue, oldest first
.\scripts\overnight.ps1 -StopAt 07:00     # start no new ticket after 7am
.\scripts\overnight.ps1 -Issues 71,72     # only these, in this order
.\scripts\overnight.ps1 -DryRun           # show the queue and exit
#>
param(
    [int[]]$Issues,
    [string]$Label = "ready-for-agent",
    [string]$Model = "claude-sonnet-5-5",
    [double]$BudgetUsd = 3.0,
    [string]$StopAt,
    [int]$Max = 30,
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
    if ($ran.Count -eq 0) { $ran = @("no crate touched, no cargo gate") }
    return @{ ok = $true; what = ($ran -join "; "); out = "" }
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
if (@(git status --porcelain).Count -gt 0) { throw "working tree is not clean; commit or stash first" }
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

$startSha = (git rev-parse HEAD).Trim()
$failures = @()
$settings = Join-Path $Root "scripts\overnight-settings.json"
$rules = Join-Path $Root "docs\agents\unattended.md"

# ---- tickets ---------------------------------------------------------------------------------
foreach ($t in $queue) {
    $n = $t.number
    $title = ($t.title -replace '"', "'")
    if ($stopTime -and (Get-Date) -ge $stopTime) { Write-Log "stop time $StopAt reached before #$n"; break }

    Write-Log "==== #$n $title"
    Set-Lock
    $before = (git rev-parse HEAD).Trim()
    $prompt = "Unattended run, nobody is at the keyboard. Read $rules first and follow it. " +
              "Then implement GitHub issue #$n ($title) on $Repo."
    $cargs = @("-p", $prompt, "--model", $Model, "--settings", $settings,
               "--permission-mode", "dontAsk", "--max-budget-usd", $BudgetUsd,
               "--output-format", "json")
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
    Write-Log "#${n}: done ($shas); gate passed: $($gate.what)"
    Add-Content $summary ("| #{0} | {1} | done | {2} | {3} |" -f $n, $title, $shas, $cost) -Encoding utf8
}

# ---- morning report --------------------------------------------------------------------------
Write-Log "==== finished"
Remove-Item $lock -Force -ErrorAction SilentlyContinue

$published = "none (no commit landed)"
if ((git rev-parse HEAD).Trim() -ne $startSha) {
    Write-Log "publishing a play copy"
    & (Join-Path $PSScriptRoot "publish-play.ps1") | ForEach-Object { Write-Log $_ }
    $published = if ($LASTEXITCODE -eq 0) { "play/" + (git rev-parse --short HEAD).Trim() } else { "failed, see run.log" }
    if ($LASTEXITCODE -ne 0) { $failures += "publish-play.ps1 failed" }
}

# One comment per night on the pinned "Overnight log" issue; format in docs/agents/unattended.md.
$assumed = @(git log --format=%B "$startSha..HEAD" | Where-Object { $_ -match '^Assumed:' })
$body = @(
    "## Night of $stamp",
    "",
    "### Tickets",
    (Get-Content $summary),
    "",
    "### Assumptions",
    $(if ($assumed.Count) { $assumed | ForEach-Object { "- $_" } } else { "- none" }),
    "",
    "### Play copy",
    "- $published",
    "",
    "### Failures",
    $(if ($failures.Count) { $failures | ForEach-Object { "- $_" } } else { "- none" })
) -join "`n"
$bodyFile = Join-Path $script:LogDir "overnight-log.md"
$body | Set-Content $bodyFile -Encoding utf8
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
