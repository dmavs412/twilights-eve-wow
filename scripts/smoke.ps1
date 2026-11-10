<#
.SYNOPSIS
Cast every spell of every Calling on the Test Grounds' training dummies, from a play/ copy (#200).

Creates one character in a home folder of its own, then for each Calling rewrites that character's
save (its Calling, level 60) and enters the world with WOW_PROBE_TEST_GROUNDS and
WOW_PROBE_SPELLS, which casts every spell in the spell book. The save's Talent Rows stay empty, so
the plain skills are cast and no Skill Choice is.

Writes smoke-report.jsonl (one line per spell: calling, spell id, name, cast ok / error, damage,
aura) and a screenshot per Calling under .claude/smoke/<timestamp>/. Exits non-zero on any failed
cast, naming it, or on running out of time, naming the Calling it stopped at.

.EXAMPLE
.\scripts\smoke.ps1                         # the newest play/ copy, 30 minutes
.\scripts\smoke.ps1 -Play play\887d452 -BudgetMin 45
.\scripts\smoke.ps1 -Callings vanguard      # one Calling only
#>
param(
    [string]$Play,
    [int]$BudgetMin = 30,
    [string[]]$Callings,
    [string]$Data = $(if ($env:WOW_DATA) { $env:WOW_DATA } else { 'C:\Users\davis\wow-data\Data' })
)

# No $ErrorActionPreference = 'Stop': on Windows PowerShell 5.1 it turns native stderr lines into
# terminating errors. Failures are checked explicitly instead.
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

$CharName = "Smoker"
# The longest one launch may take before it is stopped; the overall budget is the real limit.
$LaunchLimitSec = 600

if (-not $Play) {
    $Play = Get-ChildItem (Join-Path $Root 'play') -Directory -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
}
$exe = if ($Play) { Join-Path (Resolve-Path $Play) 'twilights-eve.exe' }
if (-not $exe -or -not (Test-Path $exe)) {
    Write-Host "no play copy (run scripts/publish-play.ps1 first)"
    exit 2
}

$stamp = Get-Date -Format "yyyy-MM-dd_HHmm"
$out = Join-Path $Root ".claude\smoke\$stamp"
$home_ = Join-Path $out "home"
New-Item -ItemType Directory -Force $home_ | Out-Null
$report = Join-Path $out "smoke-report.jsonl"
$log = Join-Path $out "smoke.log"
$deadline = (Get-Date).AddMinutes($BudgetMin)

function Write-Log([string]$msg) {
    $line = "[{0}] {1}" -f (Get-Date -Format "HH:mm:ss"), $msg
    Write-Host $line
    Add-Content $log $line -Encoding utf8
}

function Add-Report([string]$line) {
    [IO.File]::AppendAllText($report, $line + "`n", (New-Object Text.UTF8Encoding $false))
}

# One launch of the play copy with the given variables; stopped at the budget or the launch
# limit. Returns $true when it ran to its own exit.
function Invoke-Game([hashtable]$vars, [string]$name) {
    $left = ($deadline - (Get-Date)).TotalSeconds
    if ($left -le 0) { return $false }
    $limit = [Math]::Min($left, $LaunchLimitSec)
    $psi = New-Object Diagnostics.ProcessStartInfo $exe
    $psi.WorkingDirectory = Split-Path $exe
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $base = @{ WOW_DATA = $Data; BENILLA_HOME = $home_; WOW_UNATTENDED = "1" }
    foreach ($k in $base.Keys) { $psi.EnvironmentVariables[$k] = $base[$k] }
    foreach ($k in $vars.Keys) { $psi.EnvironmentVariables[$k] = [string]$vars[$k] }
    $p = [Diagnostics.Process]::Start($psi)
    $stdout = $p.StandardOutput.ReadToEndAsync()
    $stderr = $p.StandardError.ReadToEndAsync()
    $done = $p.WaitForExit([int]($limit * 1000))
    if (-not $done) { $p.Kill(); $p.WaitForExit() }
    $text = $stdout.Result + $stderr.Result
    [IO.File]::WriteAllText((Join-Path $out "$name.log"), $text, (New-Object Text.UTF8Encoding $false))
    if (-not $done) { Write-Log "$name stopped after $([int]$limit) s" }
    return $done
}

# The Calling's spell ids the probe casts first, before its swing starts combat (#523): spells
# that need the player out of combat (Stealth), stealth openers on a target out of combat (Sap),
# the other stealth openers (Garrote), then Seals and the Judgements that need one. Read from the
# sheet's fields, never from spell names or ids.
function Get-OpeningOrder($calling) {
    $all = @($calling.skills) + @($calling.rows | ForEach-Object { $_.choices })
    $hasEffect = { param($s, $kind) @($s.effects | ForEach-Object { $_.kind }) -contains $kind }
    # A Seal is a `seal` effect, or a granted aura some Judgement of the Calling judges (#703).
    $sealAuras = @(Get-SealAuras $calling)
    $groups = @(
        { param($s) $s.requires_out_of_combat },
        { param($s) $s.requires_stealth -and $s.requires_target_out_of_combat },
        { param($s) $s.requires_stealth },
        { param($s) (& $hasEffect $s 'seal') -or (Test-Grants $s $sealAuras) },
        { param($s) & $hasEffect $s 'judgement' }
    )
    $ids = @()
    foreach ($g in $groups) {
        foreach ($s in $all) {
            if ($s -and (& $g $s) -and $ids -notcontains $s.spell_id) { $ids += $s.spell_id }
        }
    }
    return ($ids -join ',')
}

# The aura keys the Calling's Judgements judge.
function Get-SealAuras($calling) {
    $all = @($calling.skills) + @($calling.rows | ForEach-Object { $_.choices })
    @($all | ForEach-Object { $_.effects } | Where-Object { $_.kind -eq 'judgement' } |
        ForEach-Object { $_.params.seal } | Where-Object { $_ } | Select-Object -Unique)
}

# Whether the skill puts one of these aura keys on the caster or its target.
function Test-Grants($skill, $auras) {
    foreach ($e in @($skill.effects)) {
        foreach ($key in @($e.params.aura, $e.params.apply_aura)) {
            if ($key -and $e.kind -ne 'copy_aura' -and $e.kind -notlike 'consume*' -and $auras -contains $key) {
                return $true
            }
        }
    }
    return $false
}

# For each skill refused until another skill has been cast, that other skill: a combo point
# builder for a finisher (#706, #709, #710), the skill that puts the aura on the target for one
# that copies it (#711, #712), a Seal for a Judgement (#703). The probe casts it, unreported,
# when the skill is refused for want of it. Only skills: the probe's spellbook holds no Talent Row
# choice. Read from the sheet's fields, never from spell names or ids.
function Get-Setups($calling) {
    $skills = @($calling.skills | Where-Object { $_ })
    $isFinisher = {
        param($s)
        @($s.effects | Where-Object {
            $_.params.per_combo -or $_.params.duration_per_combo -or $_.params.ticks_per_combo -gt 0
        }).Count -gt 0
    }
    # A builder adds combo points with no condition: not a Kick's (on a stopped cast), and not a
    # stealth opener's (Stealth is gone by then).
    $builder = $skills | Where-Object {
        $kinds = @($_.effects | ForEach-Object { $_.kind })
        ($kinds -contains 'add_combo_points') -and ($kinds -notcontains 'interrupt') -and
            -not $_.requires_stealth -and -not $_.requires_out_of_combat -and
            -not $_.requires_target_out_of_combat
    } | Select-Object -First 1
    $sealAuras = @(Get-SealAuras $calling)
    $pairs = @()
    foreach ($s in $skills) {
        $by = $null
        if ($builder -and (& $isFinisher $s)) { $by = $builder }
        foreach ($e in @($s.effects)) {
            if ($e.kind -eq 'copy_aura') {
                $key = $e.params.aura
                $by = $skills | Where-Object { $_ -ne $s -and (Test-Grants $_ @($key)) } | Select-Object -First 1
            }
            if ($e.kind -eq 'judgement') {
                $by = $skills | Where-Object { Test-Grants $_ $sealAuras } | Select-Object -First 1
            }
        }
        if ($by -and $by.spell_id -ne $s.spell_id) { $pairs += "{0}:{1}" -f $s.spell_id, $by.spell_id }
    }
    return ($pairs -join ',')
}

# The reactive skills whose event the inert Training Dummy never makes, with the reason: one
# usable only after the caster blocks (#704), one only on a target that dodged, and one that spends
# the caster's own stacks of an aura no skill gives (Red Frenzy's Bloodlust comes from blood
# droplets walked over, #705). The probe reports one still refused at the end as skipped.
function Get-Skips($calling) {
    $skills = @($calling.skills | Where-Object { $_ })
    $auraName = @{}
    foreach ($a in @($calling.auras)) { if ($a) { $auraName[$a.key] = $a.name } }
    $pairs = @()
    foreach ($s in $skills) {
        $reason = $null
        $state = if ($s.requires) { $s.requires.caster_state } else { $null }
        if ($state) {
            $reason = "usable only {0}, which the inert dummy never causes" -f ($state -replace '^after_', 'after a ' -replace '_', ' ')
        } elseif ($s.requires_target_dodged) {
            $reason = "usable only after the target dodges, which the inert dummy never does"
        }
        foreach ($e in @($s.effects | Where-Object { $_.kind -eq 'consume_self_stacks' })) {
            $key = $e.params.aura
            $given = @($skills | Where-Object { Test-Grants $_ @($key) }).Count -gt 0
            if (-not $given) {
                $name = if ($auraName[$key]) { $auraName[$key] } else { $key }
                $from = @($calling.auras | Where-Object {
                    @($_.effects | Where-Object { $_.kind -eq 'pickup' -and $_.params.grants -eq $key }).Count
                } | ForEach-Object { $_.name }) | Select-Object -First 1
                $reason = if ($from) { "needs $name stacks, which come from picking up a $from" }
                          else { "needs $name stacks, which no skill gives" }
            }
        }
        if ($reason) { $pairs += "{0}={1}" -f $s.spell_id, ($reason -replace '[;=]', ' ') }
    }
    return ($pairs -join ';')
}

# ---- the batches: one launch per Calling of the sheet --------------------------------------------
$spells = Get-Content (Join-Path $Root 'data\twilights-eve\server_spells.json') -Raw | ConvertFrom-Json
$batches = @()
foreach ($c in $spells) {
    if ($Callings -and $Callings -notcontains $c.key) { continue }
    $batches += , @{ calling = $c.key; first = (Get-OpeningOrder $c); setup = (Get-Setups $c)
                     skip = (Get-Skips $c) }
}
Write-Log ("{0} batch(es) over {1} Calling(s), {2} min budget, {3}" -f $batches.Count,
    @($batches | ForEach-Object { $_.calling } | Select-Object -Unique).Count, $BudgetMin, $exe)

# ---- the character ---------------------------------------------------------------------------
Write-Log "creating $CharName"
Invoke-Game @{ WOW_CHARCREATE_SHOT = "1"; WOW_CHARCREATE_PICK = "1,0,1"
               WOW_CHARCREATE_NAME = $CharName; WOW_PROBE_EXIT_AT = "25" } "create" | Out-Null
$save = Get-ChildItem (Join-Path $home_ 'twilight') -Filter 'character-*.toml' -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $save) {
    Write-Log "no character save after the create launch; see create.log"
    exit 2
}
$pristine = Get-Content $save.FullName -Raw

# ---- one launch per batch ----------------------------------------------------------------------
$timedOut = $null
$n = 0
foreach ($b in $batches) {
    $n++
    if ((Get-Date) -ge $deadline) { $timedOut = $b.calling; break }
    $text = $pristine -replace 'calling = "[a-z_]+"', ('calling = "{0}"' -f $b.calling)
    $text = $text -replace '(?m)^level = \d+', 'level = 60'
    # New characters start in the Barrens; the goto probe teleports within the current map, so
    # the character waits in Goldshire. Only the first map/position pair: the second is the bind.
    $text = ([regex]'(?m)^map = \d+').Replace($text, 'map = 0', 1)
    $text = ([regex]'(?m)^zone = \d+').Replace($text, 'zone = 12', 1)
    $text = ([regex]'(?m)^position = \[.*\]').Replace($text, 'position = [-9464.0, 62.0, 56.0]', 1)
    [IO.File]::WriteAllText($save.FullName, $text, (New-Object Text.UTF8Encoding $false))

    $name = "{0:D2}-{1}" -f $n, $b.calling
    Write-Log ("batch {0}, first {1}, setups {2}, skips {3}" -f $name,
        $(if ($b.first) { $b.first } else { "none" }), $(if ($b.setup) { $b.setup } else { "none" }),
        $(if ($b.skip) { $b.skip } else { "none" }))
    $before = if (Test-Path $report) { @(Get-Content $report).Count } else { 0 }
    $ran = Invoke-Game @{
        WOW_CHAR = $CharName
        WOW_PROBE_TEST_GROUNDS = "1"
        WOW_PROBE_SPELLS = $report
        WOW_PROBE_SPELLS_CALLING = $b.calling
        WOW_PROBE_SPELLS_FIRST = $b.first
        WOW_PROBE_SPELLS_SETUP = $b.setup
        WOW_PROBE_SPELLS_SKIP = $b.skip
        WOW_LIVE_SHOT = (Join-Path $out "$name.png")
        WOW_LIVE_SHOT_AT = "30"
    } $name
    $after = if (Test-Path $report) { @(Get-Content $report).Count } else { 0 }
    Write-Log ("batch {0}: {1} line(s){2}" -f $name, ($after - $before), $(if ($ran) { "" } else { ", stopped" }))
    if (-not $ran -and (Get-Date) -ge $deadline) { $timedOut = $b.calling; break }
}

# ---- the verdict -------------------------------------------------------------------------------
$lines = if (Test-Path $report) { @(Get-Content $report | ForEach-Object { $_ | ConvertFrom-Json }) } else { @() }
$failed = @($lines | Where-Object { -not $_.ok })
# A reactive spell whose event the dummy never made is skipped, not failed.
foreach ($s in @($lines | Where-Object { $_.skipped })) {
    Write-Log ("skipped {0} {1} ({2}): {3}" -f $s.calling, $s.name, $s.spell_id, $s.skipped)
}
foreach ($f in $failed) {
    Write-Log ("FAILED {0} {1} ({2}): {3}" -f $f.calling, $f.name, $f.spell_id, $f.error)
}
if ($timedOut) {
    Add-Report ('{{"timed_out_at":"{0}"}}' -f $timedOut)
    Write-Log "timed out at Calling $timedOut"
}
Write-Log ("{0} spell(s), {1} failed; report {2}" -f $lines.Count, $failed.Count, $report)
if ($failed.Count -or $timedOut) { exit 1 }
exit 0
