#Requires -Version 5.1
<#
.SYNOPSIS
    Install gdclone-bot as a Windows Task Scheduler task for the current user.

.DESCRIPTION
    Creates a Task Scheduler task that runs gdclone-bot on user logon.
    No admin rights required (task runs in user context).

.PARAMETER BinaryPath
    Full path to gdclone-bot.exe. Defaults to "$env:LOCALAPPDATA\gdclone-bot\gdclone-bot.exe".

.PARAMETER ConfigPath
    Full path to config.toml. Defaults to "$env:APPDATA\gdclone-bot\config.toml".

.PARAMETER Uninstall
    Remove the task instead of creating it.

.EXAMPLE
    .\packaging\windows-task.ps1
    .\packaging\windows-task.ps1 -BinaryPath "C:\tools\gdclone-bot.exe"
    .\packaging\windows-task.ps1 -Uninstall

.NOTES
    The task triggers on logon. To run immediately after installing, use:
        Start-ScheduledTask -TaskName "gdclone-bot"
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [string]$BinaryPath  = "$env:LOCALAPPDATA\gdclone-bot\gdclone-bot.exe",
    [string]$ConfigPath  = "$env:APPDATA\gdclone-bot\config.toml",
    [switch]$Uninstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$TaskName = "gdclone-bot"
$TaskDescription = "gdclone-bot — Telegram Google Drive clone bot"

# ── Uninstall ─────────────────────────────────────────────────────────────────
if ($Uninstall) {
    $existing = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    if ($null -eq $existing) {
        Write-Host "Task '$TaskName' not found — nothing to do."
    } else {
        if ($PSCmdlet.ShouldProcess($TaskName, "Remove-ScheduledTask")) {
            Stop-ScheduledTask  -TaskName $TaskName -ErrorAction SilentlyContinue
            Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
            Write-Host "Task '$TaskName' removed."
        }
    }
    return
}

# ── Pre-flight checks ─────────────────────────────────────────────────────────
if (-not (Test-Path $BinaryPath)) {
    Write-Error "Binary not found: $BinaryPath`nBuild with 'cargo build --release' and copy gdclone-bot.exe there, or pass -BinaryPath."
    exit 1
}
if (-not (Test-Path $ConfigPath)) {
    Write-Error "Config not found: $ConfigPath`nCreate config.toml there first, or pass -ConfigPath."
    exit 1
}

# ── Build task definition ─────────────────────────────────────────────────────
$action = New-ScheduledTaskAction `
    -Execute $BinaryPath `
    -Argument "--config `"$ConfigPath`" run" `
    -WorkingDirectory (Split-Path $BinaryPath)

# Trigger: on logon of the current user.
$trigger = New-ScheduledTaskTrigger -AtLogon -User $env:USERNAME

$settings = New-ScheduledTaskSettingsSet `
    -ExecutionTimeLimit (New-TimeSpan -Hours 0) `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1) `
    -MultipleInstances IgnoreNew `
    -StartWhenAvailable

$principal = New-ScheduledTaskPrincipal `
    -UserId $env:USERNAME `
    -RunLevel Limited `
    -LogonType Interactive

$task = New-ScheduledTask `
    -Action    $action `
    -Trigger   $trigger `
    -Settings  $settings `
    -Principal $principal `
    -Description $TaskDescription

# ── Register ──────────────────────────────────────────────────────────────────
if ($PSCmdlet.ShouldProcess($TaskName, "Register-ScheduledTask")) {
    # Remove stale entry if it exists.
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue

    Register-ScheduledTask -TaskName $TaskName -InputObject $task | Out-Null
    Write-Host "Task '$TaskName' registered successfully."
    Write-Host ""
    Write-Host "  Binary : $BinaryPath"
    Write-Host "  Config : $ConfigPath"
    Write-Host ""
    Write-Host "To start now:  Start-ScheduledTask -TaskName '$TaskName'"
    Write-Host "To view logs:  Get-Content `"$(Split-Path $BinaryPath)\logs\gdclone-bot.log`" -Tail 50"
    Write-Host "To uninstall:  .\packaging\windows-task.ps1 -Uninstall"
}
