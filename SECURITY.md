# Security

## Reporting a vulnerability

**Do not open a public issue for security problems.** Report privately via GitHub Security Advisories:

<https://github.com/GiaHung07/502Drive/security/advisories/new>

Please include a description, affected version/commit, reproduction steps, and impact. You will get a response and a fix or mitigation timeline; credit is optional and up to you.

## Supported versions

| Version | Supported |
| :--- | :--- |
| 0.2.x | yes |
| < 0.2 | no |

<!-- TODO(maintainers): Cargo.toml currently declares version 0.1.0; adjust this table when release versions are tagged. -->

## Do not commit

- `config.toml`
- `.env`
- `master.key`
- `*.db`
- `reports/`
- `logs/`

## Rotate credentials

1. Stop the bot.
2. Run `502drive auth revoke`.
3. Remove old local credential material only after backup.
4. Run `502drive auth login`.
5. Run `502drive doctor`.

Background: the Google refresh token is AES-256-GCM encrypted in the local SQLite DB with a per-device `master.key` file (mode `0600`) in the config directory. Deleting the key or the DB invalidates access locally; revoking at Google (via `auth revoke` or [Google account permissions](https://myaccount.google.com/permissions)) invalidates it server-side. See also [docs/threat-model.md](docs/threat-model.md) and [PRIVACY.md](PRIVACY.md).
