@echo off
title Remove 502Drive Startup Task
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0windows-task.ps1" -Uninstall
echo.
pause
