# ==============================================================================
# 502Drive Windows Release Packager
# Generates Windows Portable Zip bundle with checksums
# ==============================================================================
param(
    [switch]$SkipBuild,
    [string]$OutputRoot = ""
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $repo "dist\windows"
}

if (-not (Test-Path -LiteralPath $OutputRoot)) {
    New-Item -ItemType Directory -Path $OutputRoot -Force | Out-Null
}

$releaseExe = Join-Path $repo "target\release\502drive.exe"
if (-not (Test-Path -LiteralPath $releaseExe)) {
    $releaseExe = Join-Path $repo "target\x86_64-pc-windows-msvc\release\502drive.exe"
}

$version = $env:GITHUB_REF_NAME
if ([string]::IsNullOrWhiteSpace($version)) {
    $version = "v0.1.0"
}

$packageName = "502drive-$version-windows-x86_64"
$packageDir = Join-Path $OutputRoot $packageName
$zipPath = Join-Path $OutputRoot "$packageName.zip"

if (-not $SkipBuild) {
    Push-Location $repo
    try {
        cargo build --release --bin 502drive
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo release build failed"
        }
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $releaseExe)) {
    throw "Missing release executable: $releaseExe"
}

if (Test-Path -LiteralPath $packageDir) {
    Remove-Item -LiteralPath $packageDir -Recurse -Force
}
New-Item -ItemType Directory -Path $packageDir | Out-Null

Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $packageDir "502drive.exe") -Force
Copy-Item -LiteralPath (Join-Path $repo "config.sample.toml") -Destination (Join-Path $packageDir "config.sample.toml") -Force
Copy-Item -LiteralPath (Join-Path $repo "README.md") -Destination (Join-Path $packageDir "README.md") -Force
Copy-Item -LiteralPath (Join-Path $repo "packaging\windows-task.ps1") -Destination (Join-Path $packageDir "windows-task.ps1") -Force

$readmeText = @"
================================================================================
502Drive — Local-first Google Drive Clone & Realtime Sync Bot
================================================================================

1. Quick Start:
   - Copy `config.sample.toml` to `config.toml`
   - Edit `config.toml` with your Telegram Bot Token and Google Client ID/Secret
   - Run: `502drive.exe run`

2. Run as Windows Scheduled Task (starts on boot / login):
   - Open PowerShell as Administrator
   - Run: `.\windows-task.ps1`

Documentation: https://github.com/GiaHung07/502Drive
"@
$readmeText | Set-Content -LiteralPath (Join-Path $packageDir "README-WINDOWS.txt") -Encoding UTF8

if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -Path $packageDir -DestinationPath $zipPath -Force

$hash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
"$hash  $packageName.zip" | Set-Content -LiteralPath (Join-Path $OutputRoot "$packageName.zip.sha256") -Encoding ASCII

Write-Host "[OK] Windows package created: $zipPath"
Write-Host "[OK] SHA256: $hash"
