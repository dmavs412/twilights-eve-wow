<#
.SYNOPSIS
Start overnight.ps1 (the two builders) so it outlives the shell or chat that started it.

A process started from a Claude Code chat lives in that chat's Windows job object, and a restart
of the chat or of VS Code kills it (the 2026-11-09 run died at 11:04 that way, leaving two
claimed tickets). This starts the fleet through WMI, whose processes belong to no such job.
Arguments go straight to overnight.ps1. The fleet opens its own PowerShell window.

.EXAMPLE
.\scripts\start-builders.ps1 -LingerMinutes 10 -StopAt 07:00 -BudgetUsd 25 -Max 60
#>
$root = Split-Path $PSScriptRoot -Parent
$quoted = @($args | ForEach-Object { if ("$_" -match '\s') { "`"$_`"" } else { "$_" } }) -join ' '
$cmd = "powershell.exe -NoProfile -ExecutionPolicy Bypass -NoExit -File `"$root\scripts\overnight.ps1`" $quoted"
$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = $cmd; CurrentDirectory = $root }
if ($r.ReturnValue -ne 0) { throw "Win32_Process.Create returned $($r.ReturnValue)" }
"builders started: overnight.ps1 pid $($r.ProcessId); logs in $root\.claude\overnight\"
