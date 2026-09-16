# Contributing to Drive502

Thanks for helping make Drive502 safer and easier to run.

## Ground Rules

- Keep Drive502 local-first and Google Drive focused.
- Keep contributions compatible with `GPL-3.0-only`.
- Do not add quota bypasses, service-account rotation, scraping, torrent/leech scope, or download-disabled bypasses.
- Prefer small, reviewable changes.
- Keep Telegram UX usable without memorizing long commands.
- User-facing Telegram text must support both Vietnamese and English.
- Never commit secrets, tokens, DB files, logs, reports, or `master.key`.

## Before Opening A PR

Run:

```bash
cargo fmt --check
cargo test
cargo build --release
```

For Telegram UX changes, also check:

- `/menu` still opens the control center.
- Navigation uses buttons and edits existing bot messages where possible.
- Destructive actions have confirmation or a clear recovery path.
- New user-facing text is covered by `telegram.language = "vi"` and `"en"`.

## Good First Areas

- Documentation fixes.
- Setup troubleshooting.
- Full i18n catalog work.
- GitHub Actions and release packaging.
- Windows setup verification.

## Out Of Scope

- Hosted SaaS.
- Public webhook requirement.
- Service-account rotation.
- Torrent/direct-link leeching.
- Quota bypass.
- Permission cloning or ownership transfer in v1.
