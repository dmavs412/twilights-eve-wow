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
# Lowercase drive letter: Claude keys workspace trust by cwd as spelled, and VS Code trusted
# "c:/Users/...". An uppercase "C:" counts as a different, untrusted project.
$Root = Split-Path $PSScriptRoot -Parent
$Root = $Root.Substring(0, 1).ToLower() + $Root.Substring(1)
Set-Location $Root

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
    $crates = @($files | ForEach-Object { if ($_ -match '^crates/([^/]+)/') { $Matches[1] } } | Sort-Object -Unique)
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
    gh issue edit -R $Repo $n --add-label blocked | Out-Null
    gh issue comment -R $Repo $n --body-file $tmp | Out-Null
}

# ---- preflight -------------------------------------------------------------------------------
$claude = Find-Claude
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($branch -ne "main") { throw "on branch '$branch', not main" }
if (@(git status --porcelain).Count -gt 0) { throw "working tree is not clean; commit or stash first" }
gh auth status 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { throw "gh is not authenticated" }

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
    $jq = '[.[] | select([.labels[].name] | index("blocked") | not)] | sort_by(.number) | .[] | "\(.number)\t\(.title)"'
    $raw = gh issue list -R $Repo --state open --label $Label --limit 100 --json number,title,labels --jq $jq
    $queue = @($raw | Where-Object { $_ } | ForEach-Object {
        $parts = $_ -split "`t", 2
        @{ number = [int]$parts[0]; title = $parts[1] }
    })
}
if ($queue.Count -gt $Max) { $queue = $queue[0..($Max - 1)] }

Write-Log ("claude: {0}" -f $claude)
Write-Log ("queue: {0} ticket(s): {1}" -f $queue.Count, (($queue | ForEach-Object { "#" + $_.number }) -join " "))
if ($DryRun) { $queue | ForEach-Object { "#{0}  {1}" -f $_.number, $_.title }; exit 0 }
if ($queue.Count -eq 0) { Write-Log "nothing to do"; exit 0 }

$startSha = (git rev-parse HEAD).Trim()
$settings = Join-Path $Root "scripts\overnight-settings.json"
$rules = Join-Path $Root "docs\agents\unattended.md"

# ---- tickets ---------------------------------------------------------------------------------
foreach ($t in $queue) {
    $n = $t.number
    $title = ($t.title -replace '"', "'")
    if ($stopTime -and (Get-Date) -ge $stopTime) { Write-Log "stop time $StopAt reached before #$n"; break }

    Write-Log "==== #$n $title"
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
    } catch { }
    $text | Set-Content (Join-Path $script:LogDir "$n.log") -Encoding utf8
    Write-Log "session exit $exit, $turns turns, `$$cost"

    # The session must leave a clean tree: commits or nothing.
    $dirty = @(git status --porcelain)
    if ($dirty.Count -gt 0) {
        $diffFile = Join-Path $script:LogDir "$n.discarded.diff"
        git diff | Set-Content $diffFile -Encoding utf8
        $dirty | Add-Content $diffFile -Encoding utf8
        git checkout -- . | Out-Null
        git clean -fd | Out-Null
        Write-Log "discarded $($dirty.Count) uncommitted path(s); see $n.discarded.diff"
    }

    $after = (git rev-parse HEAD).Trim()
    if ($after -eq $before) {
        $labels = gh issue view -R $Repo $n --json labels --jq '[.labels[].name] | join(",")'
        if ($labels -notmatch "blocked") {
            $tail = $text.Substring([Math]::Max(0, $text.Length - 1500))
            Set-Blocked $n "Overnight run produced no commit (exit $exit, $turns turns). Last words of the session:`n`n$tail"
        }
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
Get-Content $summary | Write-Host
Write-Host ""
Write-Host "commits since start:"
git log --oneline "$startSha..HEAD"
Write-Host ""
Write-Host "logs: $script:LogDir"
