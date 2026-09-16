@echo off
title Stop 502Drive
echo [*] Stopping 502Drive background process...
taskkill /F /IM 502drive.exe >nul 2>&1
if %ERRORLEVEL% equ 0 (
    echo [OK] 502Drive has been stopped.
) else (
    echo [i] 502Drive was not running.
)
timeout /t 2 >nul
