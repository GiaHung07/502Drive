@echo off
title 502Drive Doctor (Health Check)
cd /d "%~dp0"
502drive.exe doctor
echo.
pause
