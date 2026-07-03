# Windows Setup

Default startup is Task Scheduler at user logon, not LocalSystem service.

```powershell
cargo build --release
.\target\release\gdclone-bot.exe auth login
.\target\release\gdclone-bot.exe doctor
.\target\release\gdclone-bot.exe run
```

