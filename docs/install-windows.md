# Windows Installation

> **Status: community verification pending.** The daemon itself is cross-platform Rust and the watch/sync behavior is identical to Linux, but the Windows packaging steps below have not been verified end-to-end by CI or maintainers. If something does not match, please [open an issue](https://github.com/GiaHung07/502Drive/issues).

For Linux see [install-linux.md](install-linux.md), for Docker see [install-docker.md](install-docker.md), for Google OAuth see [oauth-setup.md](oauth-setup.md).

## 1. Prerequisites

- Rust 1.85+ (2024 edition) via [rustup](https://rustup.rs) — `rustup default stable`.
- Git (optional, or download a source zip / prebuilt `502drive-v*-windows-x86_64.zip` from [Releases](https://github.com/GiaHung07/502Drive/releases)).
- No OpenSSL or MSVC system libraries beyond the default VS Build Tools toolchain that rustup sets up.

## 2. Build

```powershell
git clone https://github.com/GiaHung07/502Drive.git
cd 502Drive
cargo build --release --bin 502drive
.\target\release\502drive.exe --help
```

## 3. Configure

The default config location on Windows is:

```
%APPDATA%\gdclone-bot\config\config.toml
```

Copy `config.sample.toml` there (or next to the exe and pass `--config .\config.toml`), then set `telegram.bot_token`, `telegram.owner_telegram_id`, `google_oauth.client_id`, and `google_oauth.client_secret`. Keep `watch.enabled = false` unless you want change polling (see [sync-semantics.md](sync-semantics.md)).

State (SQLite DB, logs, reports) lives under `%APPDATA%\gdclone-bot\data\` by default; `502drive doctor` prints the exact paths it uses.

## 4. First authentication

```powershell
.\target\release\502drive.exe auth login
.\target\release\502drive.exe doctor
```

`auth login` opens your browser and receives the OAuth redirect on loopback ports 51000–51100. Details in [oauth-setup.md](oauth-setup.md).

## 5. Run at logon (Task Scheduler)

The supported background mechanism on Windows is a **Task Scheduler logon task under the same user that ran OAuth** — not a LocalSystem service. Service-account contexts break credential/secret access (see [architecture.md](architecture.md), ADR 0006).

```powershell
.\target\release\502drive.exe service-install
schtasks /Run /TN 502Drive      # start it now without re-logging on
```

<!-- TODO(maintainers): older docs referenced the task name `gdclone-bot`; src/platform/windows.rs registers the task as `502Drive`. Verify on a Windows host which name the shipped 0.2.x binaries actually use. -->

Remove the task:

```powershell
.\target\release\502drive.exe service-uninstall
```

### Portable alternative: `windows-task.ps1`

`packaging/windows-task.ps1` (shipped in release bundles) creates the same kind of logon task without needing the exe's built-in command; no admin rights are required:

```powershell
powershell -ExecutionPolicy Bypass -File .\windows-task.ps1                       # install (auto-detects 502drive.exe + config.toml next to the script)
powershell -ExecutionPolicy Bypass -File .\windows-task.ps1 -BinaryPath "C:\Tools\502Drive\502drive.exe" -ConfigPath "C:\Tools\502Drive\config.toml"
powershell -ExecutionPolicy Bypass -File .\windows-task.ps1 -Uninstall            # remove
```

The release bundle also contains simple helpers in `packaging\windows\`: `Start-502Drive.bat`, `Stop-502Drive.bat`, `Register-Startup.bat`, `Unregister-Startup.bat`, and `Doctor.bat`.

## 6. Run in foreground

```powershell
.\target\release\502drive.exe run
```

## 7. Windows-specific notes

- Keep `auth login` and the scheduled task under the **same Windows user**. Do not run the bot as LocalSystem.
- Source deletes preserve the destination copy and destination deletes by the user are respected — the same one-way semantics as everywhere else ([sync-semantics.md](sync-semantics.md)).
- If the scheduled task starts but the bot cannot reach Google or Telegram, check `%APPDATA%\gdclone-bot\data\logs\` for the JSON log file.
