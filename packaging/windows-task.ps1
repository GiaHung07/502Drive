#Requires -Version 5.1
<#
.SYNOPSIS
    Install 502Drive as a Windows Task Scheduler task for the current user.

.DESCRIPTION
    Creates a Task Scheduler task that runs 502Drive on user logon in the background.
    No admin rights required (task runs in the current user context).

.PARAMETER BinaryPath
    Full path to 502drive.exe. Defaults to script directory's 502drive.exe or "$env:LOCALAPPDATA\502drive\502drive.exe".

.PARAMETER ConfigPath
    Full path to config.toml. Defaults to script directory's config.toml or "$env:APPDATA\502drive\config.toml".

.PARAMETER Uninstall
    Remove the task instead of creating it.

.EXAMPLE
    .\windows-task.ps1
    .\windows-task.ps1 -BinaryPath "C:\tools\502drive.exe"
    .\windows-task.ps1 -Uninstall
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [string]$BinaryPath  = "",
    [string]$ConfigPath  = "",
    [switch]$Uninstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$TaskName = "502Drive"
$TaskDescription = "502Drive — Local-first Telegram Google Drive Clone & Realtime Sync"

# Resolve default paths
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($BinaryPath)) {
    $localExe = Join-Path $scriptDir "502drive.exe"
    if (Test-Path -LiteralPath $localExe) {
        $BinaryPath = $localExe
    } else {
        $BinaryPath = "$env:LOCALAPPDATA\502drive\502drive.exe"
    }
}

if ([string]::IsNullOrWhiteSpace($ConfigPath)) {
    $localConfig = Join-Path $scriptDir "config.toml"
    if (Test-Path -LiteralPath $localConfig) {
        $ConfigPath = $localConfig
    } else {
        $ConfigPath = "$env:APPDATA\502drive\config.toml"
    }
}

# ── Uninstall ─────────────────────────────────────────────────────────────────
if ($Uninstall) {
    $existing = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    if ($null -eq $existing) {
        Write-Host "Task '$TaskName' not found — nothing to do."
    } else {
        if ($PSCmdlet.ShouldProcess($TaskName, "Remove-ScheduledTask")) {
            Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
            Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
            Write-Host "Task '$TaskName' removed successfully."
        }
    }
    return
}

# ── Pre-flight checks ─────────────────────────────────────────────────────────
if (-not (Test-Path $BinaryPath)) {
    Write-Error "Binary not found: $BinaryPath`nPlease place 502drive.exe in the same folder, or pass -BinaryPath."
    exit 1
}
if (-not (Test-Path $ConfigPath)) {
    Write-Error "Config not found: $ConfigPath`nPlease copy config.sample.toml to config.toml, or pass -ConfigPath."
    exit 1
}

# ── Build task definition ─────────────────────────────────────────────────────
$action = New-ScheduledTaskAction `
    -Execute $BinaryPath `
    -Argument "--config `"$ConfigPath`"" `
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
    -Action      $action `
    -Trigger     $trigger `
    -Settings    $settings `
    -Principal   $principal `
    -Description $TaskDescription

# ── Register ──────────────────────────────────────────────────────────────────
if ($PSCmdlet.ShouldProcess($TaskName, "Register-ScheduledTask")) {
    # Remove stale entry if it exists.
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue

    Register-ScheduledTask -TaskName $TaskName -InputObject $task | Out-Null
    Write-Host "[OK] Task '$TaskName' registered successfully."
    Write-Host ""
    Write-Host "  Binary : $BinaryPath"
    Write-Host "  Config : $ConfigPath"
    Write-Host ""
    Write-Host "To start now:     Start-ScheduledTask -TaskName '$TaskName'"
    Write-Host "To stop:          Stop-ScheduledTask -TaskName '$TaskName'"
    Write-Host "To uninstall:     .\windows-task.ps1 -Uninstall"
}
