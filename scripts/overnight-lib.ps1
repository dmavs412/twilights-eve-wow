# Shared by overnight.ps1 (the fleet: starts the builders, writes the morning log) and worker.ps1
# (one builder: claims tickets and lands them on origin/main). Dot-source it; it expects $Repo.

. (Join-Path $PSScriptRoot 'client-lock.ps1')

$WorkflowLabels = @("ready-for-agent", "blocked", "needs-triage", "needs-info", "in-progress")
$ClaimDir = Join-Path $env:LOCALAPPDATA 'twilights-eve\claims'

# Headless claude keys workspace trust on "C:/Users/..." (uppercase drive, forward slashes),
# a different entry from the "c:/..." one VS Code trusted. Untrusted means the allowlist in
# .claude/settings.json is ignored and every ticket stalls on a denied Bash call.
function Get-TrustKey([string]$path) {
    $key = ($path -replace '\\', '/').TrimEnd('/')
    return $key.Substring(0, 1).ToUpper() + $key.Substring(1)
}

function Test-Trusted([string]$path) {
    $cfg = Join-Path $env:USERPROFILE ".claude.json"
    if (-not (Test-Path $cfg)) { return $false }
    $key = Get-TrustKey $path
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
    # A reader (a tail in another shell) can hold the file for a moment; retry instead of losing the line.
    for ($i = 0; $i -lt 20; $i++) {
        try { [IO.File]::AppendAllText($script:RunLog, "$line`r`n", (New-Object Text.UTF8Encoding $false)); return }
        catch { Start-Sleep -Milliseconds 250 }
    }
}

# The gate for the commits $Before..$After in the current directory's checkout. Runs cargo via
# cmd so stderr merges cleanly under PowerShell 5.1. A client crate waits for the machine-wide
# client lock first. Returns @{ok; what; out}.
function Invoke-Gate([string]$Before, [string]$After, [string]$LockHolder) {
    $here = (Get-Location).Path
    $files = @(git diff --name-only $Before $After)
    # A crate the commits deleted has nothing left to check.
    $crates = @($files | ForEach-Object { if ($_ -match '^crates/([^/]+)/') { $Matches[1] } } | Sort-Object -Unique |
        Where-Object { Test-Path (Join-Path $here "crates\$_\Cargo.toml") })
    $realm = @($files | Where-Object { $_ -match '^(crates/twilight-realm/|data/|tools/)' }).Count -gt 0
    $ran = @()
    foreach ($c in $crates) {
        $client = $c -match '^(benilla-|twilights-eve$)'
        if ($client) {
            Write-Log "gate: waiting for the client build lock"
            if (-not (Enter-ClientLock $LockHolder $PID 120)) {
                return @{ ok = $false; what = "client build lock (waited 2 h)"; out = "held by $((Get-ClientLockHolder).holder)" }
            }
        }
        Write-Log "gate: cargo check -p $c"
        $out = cmd /c "cargo check -q -p $c 2>&1"
        $code = $LASTEXITCODE
        if ($client) { Exit-ClientLock $LockHolder }
        $ran += "cargo check -p $c"
        if ($code -ne 0) {
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
        foreach ($tool in @("ground_check.py", "icon_check.py")) {
            Write-Log "gate: $tool"
            $out = cmd /c "`"$HOME\.local\bin\python3.15`" -I tools\twilights-eve\$tool 2>&1"
            $ran += $tool
            if ($LASTEXITCODE -ne 0) {
                return @{ ok = $false; what = $tool; out = (($out | Select-Object -Last 60) -join "`n") }
            }
        }
    }
    if ($ran.Count -eq 0) { $ran = @("no crate touched, no cargo gate") }
    return @{ ok = $true; what = ($ran -join "; "); out = "" }
}

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

function Get-OpenBlockers([int]$n) {
    return @(Get-Blockers $n | Where-Object { (gh issue view -R $Repo $_ --json state --jq .state) -eq "OPEN" })
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
    gh issue edit -R $Repo $n --add-label blocked --remove-label "ready-for-agent,in-progress" | Out-Null
    gh issue comment -R $Repo $n --body-file $tmp | Out-Null
}

# Claims are files, created atomically, so two builders on this machine never take one ticket.
# The `in-progress` label shows the claim on GitHub.
function Request-Claim([int]$n, [string]$worker) {
    New-Item -ItemType Directory -Force $ClaimDir | Out-Null
    try {
        $fs = [IO.File]::Open((Join-Path $ClaimDir "$n"), 'CreateNew', 'Write')
        $bytes = [Text.Encoding]::UTF8.GetBytes($worker)
        $fs.Write($bytes, 0, $bytes.Length); $fs.Close()
    } catch { return $false }
    gh issue edit -R $Repo $n --add-label in-progress | Out-Null
    return $true
}

function Clear-Claim([int]$n) {
    Remove-Item (Join-Path $ClaimDir "$n") -Force -ErrorAction SilentlyContinue
}

# The open, unblocked, unclaimed tickets with $Label: bugs first, then the rest, oldest first.
# With $Only, just those numbers, in that order.
function Get-Queue([string]$Label, [int[]]$Only) {
    # Filter here, not in --jq: PowerShell 5.1 strips the double quotes a jq string needs.
    $raw = (gh issue list -R $Repo --state open --label $Label --limit 200 --json number,title,labels) -join "`n" | ConvertFrom-Json
    $q = @($raw |
        Where-Object { @($_.labels | Where-Object { $_.name -in @("blocked", "in-progress") }).Count -eq 0 } |
        Where-Object { -not (Test-Path (Join-Path $ClaimDir "$($_.number)")) } |
        Sort-Object @{ e = { if (@($_.labels | Where-Object { $_.name -eq "bug" }).Count -gt 0) { 0 } else { 1 } } }, @{ e = { [int]$_.number } } |
        ForEach-Object { @{ number = [int]$_.number; title = $_.title } })
    if ($Only) { $q = @($Only | ForEach-Object { $o = $_; $q | Where-Object { $_.number -eq $o } }) }
    return $q
}
