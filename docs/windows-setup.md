# Windows Setup

Default startup is Task Scheduler at user logon, not LocalSystem service.

```powershell
cargo build --release
.\target\release\gdclone-bot.exe auth login
.\target\release\gdclone-bot.exe doctor
.\target\release\gdclone-bot.exe run
```

Install startup task for the current Windows user:

```powershell
.\target\release\gdclone-bot.exe service-install
schtasks /Run /TN gdclone-bot
```

Remove it:

```powershell
.\target\release\gdclone-bot.exe service-uninstall
```

Keep OAuth login and the scheduled task under the same Windows user. Do not run
the bot as LocalSystem.

Watch/sync uses the same Rust code on Windows and Linux. Build the Windows
binary after Watch changes to get the same behavior: source deletes preserve the
destination copy, destination deletes by the user are respected, and fallback
scan only copies source items the watch has never mapped before.
