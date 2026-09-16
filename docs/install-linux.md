# Linux Installation

Covers building/installing the `502drive` daemon (`gdclone-bot` is an alias for the same binary), the systemd user service, and the tray applet. For Windows see [install-windows.md](install-windows.md), for Docker see [install-docker.md](install-docker.md), for Google OAuth see [oauth-setup.md](oauth-setup.md).

## 1. Dependencies

Any recent Linux distribution works. On Arch/CachyOS:

```bash
sudo pacman -Syu
sudo pacman -S --needed rustup git
rustup default stable
```

On Debian/Ubuntu the equivalent is `sudo apt install build-essential git` plus rustup. Rust 1.85+ (2024 edition) is required. No OpenSSL or other C libraries are needed: the build uses pure-Rust TLS (`rustls`) and statically bundled SQLite.

The tray applet additionally needs Python 3 with GTK (usually preinstalled on desktop environments); it is optional.

## 2. Build

```bash
git clone https://github.com/GiaHung07/502Drive.git
cd 502Drive
cargo build --release --bin 502drive
./target/release/502drive --help
```

Alternatively download a prebuilt bundle (`502drive-v*-linux-x86_64.tar.gz` or aarch64) from [Releases](https://github.com/GiaHung07/502Drive/releases) and extract it.

## 3. Install (binary, tray, desktop entry, services)

Run the universal installer from the repo root (or from the extracted release bundle):

```bash
bash packaging/install.sh
```

It installs, per-user (no root required):

- `~/.local/bin/502drive` (plus a `gdclone-bot` symlink alias);
- `~/.local/bin/502drive-gui` if the Tauri desktop GUI binary was built;
- `~/.local/bin/502drive-tray` (GTK tray applet) and its icons/`.desktop` entry;
- systemd user units `~/.config/systemd/user/gdclone-bot.service` and `502drive-tray.service`;
- a default config at `~/.config/gdclone-bot/config.toml` if none exists (copied from `config.sample.toml`).

## 4. Configure

Edit the config:

```bash
$EDITOR ~/.config/gdclone-bot/config.toml
```

At minimum set `telegram.bot_token`, `telegram.owner_telegram_id`, `google_oauth.client_id`, and `google_oauth.client_secret`. Keep `watch.enabled = false` unless you want change polling. The config and data directory must be readable only by you (`0600`/`0700`); the installer and scripts enforce this. Every key can also be overridden via `GDCLONE__SECTION__KEY` environment variables — see [install-docker.md](install-docker.md) for the table (the same layering applies on Linux).

## 5. First authentication

```bash
502drive auth login    # opens a browser on the loopback redirect (ports 51000-51100)
502drive doctor        # verifies DB, Telegram getMe, Google token, destination
```

`auth login` binds `127.0.0.1` only and uses PKCE; details in [oauth-setup.md](oauth-setup.md).

## 6. Run as a systemd user service

`packaging/install.sh` already copied the unit file. To start it:

```bash
systemctl --user daemon-reload
systemctl --user enable --now gdclone-bot.service
systemctl --user enable --now 502drive-tray.service   # optional tray applet
journalctl --user -u gdclone-bot.service -f
```

### Alternative: hardened script installer

`scripts/install-systemd-user.sh` writes a hardened unit (system-call filter, `ProtectSystem=strict`, writable paths limited to config/data/log dirs), enables linger so the service survives logout, and starts the service:

```bash
bash scripts/install-systemd-user.sh
systemctl --user status gdclone-bot --no-pager
journalctl --user -u gdclone-bot -f
```

It installs `target/release/gdclone-bot` to `~/.local/bin/gdclone-bot` (same binary, alias name), writes `~/.config/systemd/user/gdclone-bot.service`, and enables linger via `loginctl enable-linger "$USER"` when permitted.

Manual controls:

```bash
systemctl --user restart gdclone-bot
systemctl --user stop gdclone-bot
systemctl --user disable --now gdclone-bot
```

## 7. Foreground run (no service)

```bash
502drive run    # or just: 502drive
```

Logs also go to `~/.local/share/gdclone-bot/logs/gdclone-bot.log` (JSON lines) in addition to stdout/journald.

## 8. Uninstall

```bash
systemctl --user disable --now gdclone-bot 502drive-tray
bash scripts/uninstall-systemd-user.sh   # removes unit and binary if installed by the script
rm -rf ~/.local/share/gdclone-bot ~/.config/gdclone-bot   # removes state, tokens, master.key
```
