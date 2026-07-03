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
gdclone-bot --config "$HOME/.config/gdclone-bot/config.toml" service-install
```

Use `loginctl enable-linger "$USER"` only if the user service must continue after logout.
