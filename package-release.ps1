# ==============================================================================
# 502Drive Windows Release Packager
# Generates Windows Portable Zip bundle with checksums & launcher scripts
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

# Copy binary & core assets
Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $packageDir "502drive.exe") -Force
Copy-Item -LiteralPath (Join-Path $repo "packaging\502drive.ico") -Destination (Join-Path $packageDir "502drive.ico") -Force
Copy-Item -LiteralPath (Join-Path $repo "config.sample.toml") -Destination (Join-Path $packageDir "config.sample.toml") -Force
Copy-Item -LiteralPath (Join-Path $repo "README.md") -Destination (Join-Path $packageDir "README.md") -Force
Copy-Item -LiteralPath (Join-Path $repo "packaging\windows-task.ps1") -Destination (Join-Path $packageDir "windows-task.ps1") -Force

# Copy Windows 1-click batch and runner scripts
$winDir = Join-Path $repo "packaging\windows"
if (Test-Path -LiteralPath $winDir) {
    Get-ChildItem -LiteralPath $winDir -File | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $packageDir $_.Name) -Force
    }
}

$readmeText = @"
================================================================================
502Drive (v$version) — Windows Portable Release
Local-first Google Drive Clone & Realtime Sync Bot
================================================================================

QUICK START:
1. Copy `config.sample.toml` to `config.toml` (or just double-click `Start-502Drive.bat`).
2. Edit `config.toml` with your Telegram Bot Token and Google Client credentials.
   (Note: `owner_telegram_id` in sample is a placeholder - replace with your real ID).
3. Double-click `Start-502Drive.bat` (foreground console) or `Start-Hidden.vbs` (silent background).

FEATURES & UTILITIES:
- Start-502Drive.bat        : Launch bot in a command prompt window
- Start-Hidden.vbs          : Launch bot silently in background (no black window)
- Stop-502Drive.bat         : Stop running 502Drive background process
- Doctor.bat                : Health check Google Drive and Telegram connection
- Register-Startup.bat      : Auto-start 502Drive invisibly on Windows logon (Task Scheduler)
- Unregister-Startup.bat    : Remove Windows auto-start task

CLI USAGE:
  502drive.exe               (starts the bot directly)
  502drive.exe doctor        (diagnostics)
  502drive.exe status        (database status)
  502drive.exe auth login    (OAuth browser login)

Repository & Issues: https://github.com/GiaHung07/502Drive
"@
$readmeText | Set-Content -LiteralPath (Join-Path $packageDir "README-WINDOWS.txt") -Encoding UTF8

$exeHash = (Get-FileHash -LiteralPath (Join-Path $packageDir "502drive.exe") -Algorithm SHA256).Hash
$manifest = [ordered]@{
    package = $packageName
    version = $version
    built_at = (Get-Date).ToString("s")
    files = @(
        [ordered]@{
            file = "502drive.exe"
            sha256 = $exeHash
            bytes = (Get-Item -LiteralPath (Join-Path $packageDir "502drive.exe")).Length
        }
    )
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $packageDir "release-manifest.json") -Encoding UTF8

if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -Path $packageDir -DestinationPath $zipPath -Force

$hash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
"$hash  $packageName.zip" | Set-Content -LiteralPath (Join-Path $OutputRoot "$packageName.zip.sha256") -Encoding ASCII

Write-Host "[OK] Windows package created: $zipPath"
Write-Host "[OK] SHA256: $hash"
