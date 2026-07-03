# CachyOS Setup

```bash
sudo pacman -Syu
sudo pacman -S --needed rustup git
rustup default stable
cargo build --release
install -Dm755 target/release/gdclone-bot "$HOME/.local/bin/gdclone-bot"
install -Dm600 config.sample.toml "$HOME/.config/gdclone-bot/config.toml"
$EDITOR "$HOME/.config/gdclone-bot/config.toml"
gdclone-bot --config "$HOME/.config/gdclone-bot/config.toml" auth login
gdclone-bot --config "$HOME/.config/gdclone-bot/config.toml" doctor
gdclone-bot --config "$HOME/.config/gdclone-bot/config.toml" run
```

## Run In Background

Install and start the systemd user service:

```bash
bash scripts/install-systemd-user.sh
systemctl --user status gdclone-bot --no-pager
journalctl --user -u gdclone-bot -f
```

The installer:

- installs `target/release/gdclone-bot` to `~/.local/bin/gdclone-bot`;
- writes `~/.config/systemd/user/gdclone-bot.service`;
- enables and starts the user service;
- enables linger with `loginctl enable-linger "$USER"` when allowed.

Manual controls:

```bash
systemctl --user restart gdclone-bot
systemctl --user stop gdclone-bot
systemctl --user disable --now gdclone-bot
```
