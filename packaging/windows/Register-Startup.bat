@echo off
title Register 502Drive Startup Task
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0windows-task.ps1"
echo.
pause
