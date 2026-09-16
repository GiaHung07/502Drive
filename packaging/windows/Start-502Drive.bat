@echo off
title 502Drive
cd /d "%~dp0"

if not exist config.toml (
    echo [!] File config.toml not found!
    if exist config.sample.toml (
        echo [*] Creating config.toml from config.sample.toml...
        copy config.sample.toml config.toml >nul
        echo [OK] Please edit config.toml with your Bot Token and Google Client ID.
        notepad config.toml
        pause
        exit /b 0
    ) else (
        echo [ERROR] config.sample.toml missing. Please download complete package.
        pause
        exit /b 1
    )
)

echo [*] Starting 502Drive...
502drive.exe
pause
