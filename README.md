# Smart Hawk 🦅

[![CI](https://github.com/Aryagorjipour/hawk/actions/workflows/ci.yml/badge.svg)](https://github.com/Aryagorjipour/hawk/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

**AI-powered website crawler Telegram bot** — [@SmartHawk_bot](https://t.me/SmartHawk_bot)

Repository: [github.com/Aryagorjipour/hawk](https://github.com/Aryagorjipour/hawk)

Users bring their own LLM API key (OpenAI, Anthropic, Gemini, Grok, OpenRouter, or any OpenAI-compatible endpoint). Smart Hawk fetches pages (HTTP first, headless Chromium fallback), extracts what they asked for, schedules recurring hunts, and keeps a timeline history.

Built as a **production-oriented Rust monolith** with:

- **Hexagonal architecture** (ports & adapters)
- **DDD** aggregates, value objects, domain services & events
- **SQLite** persistence (WAL)
- **AES-256-GCM** encryption for API keys at rest
- **tracing** + in-process domain event bus
- **en / fa** UI strings

## Features

| Area | What you get |
|------|----------------|
| Onboarding | Username → provider → (custom base URL) → API key → model list → connection check |
| Crawl | URL validation + SSRF guards, prompt, agent loop (up to 4 pages), structured JSON → pretty Markdown |
| Schedule | Interval / daily / weekly, **Trigger now**, chat & email flags, activate/deactivate/delete |
| Usage | Free daily + bonus credits + schedule slots at a glance |
| Settings | Name, email, timezone, AI, language, hard-delete all data |
| History | Last 100 crawls |
| About / Stars | Credit packs (25 / 100 / 250 ★) |
| Inline | `@SmartHawk_bot <url> [ask…]` and history picker (`h`) |
| Email | Resend primary, SMTP fallback |

### Free tier

- 10 crawls / day
- 3 active schedules

### Stars packs

| Stars | Crawl credits | Bonus schedule slots |
|------:|--------------:|---------------------:|
| 25 | 25 | +1 |
| 100 | 120 | +5 |
| 250 | 350 | +12 |

## Architecture

```
src/
  domain/          # entities, VOs, pure domain services
  ports/           # traits (repositories, LLM, fetcher, mailer, crypto)
  application/     # use cases + conversation FSM
  adapters/        # Telegram, SQLite, HTTP/browser crawler, LLM client, SMTP, i18n
  infrastructure/  # worker pool, event bus
  bootstrap/       # config, tracing, DI wiring
```

Dependency rule: **domain ← application ← adapters**; `bootstrap` wires concrete adapters.

## Requirements

- Rust 1.75+ (tested on 1.97)
- SQLite (bundled via sqlx)
- **No system OpenSSL** — the bot links **rustls** only (works on minimal servers)
- Optional: Chromium for JS-heavy pages
- Optional: Resend and/or SMTP for email delivery

## Quick start

```bash
cp .env.example .env
# fill TELEGRAM_BOT_TOKEN and SMART_HAWK_MASTER_KEY

cargo run --release
```

If you previously hit `openssl-sys` / `pkg-config` errors, pull latest and rebuild:

```bash
git pull
cargo clean
cargo build --release
```

Generate a master key (any CSPRNG is fine; `openssl` CLI is optional):

```bash
openssl rand -hex 32
# or: python3 -c 'import secrets; print(secrets.token_hex(32))'
```

Enable inline mode and payments (Stars) in [@BotFather](https://t.me/BotFather) for `@SmartHawk_bot`.

## Configuration

| Variable | Required | Description |
|----------|----------|-------------|
| `TELEGRAM_BOT_TOKEN` | yes | Bot token |
| `SMART_HAWK_MASTER_KEY` | yes | 32-byte key (hex/base64) or ≥16 char passphrase |
| `DATABASE_URL` | no | default `sqlite:data/smart-hawk.db?mode=rwc` |
| `ABOUT_LANDING_URL` | no | About screen link |
| `ABOUT_GITHUB_URL` | no | About screen link |
| `RESEND_API_KEY` / `RESEND_FROM` | no | Primary email (Resend API). **Quote** `RESEND_FROM` if it has spaces/`<>` |
| `SMTP_URL` / `SMTP_FROM` | no | SMTP fallback if Resend fails or unset |
| `EMAIL_FROM` | no | Shared from-address for Resend/SMTP |

### Email `.env` example (works with Docker)

```env
RESEND_API_KEY=re_your_key
RESEND_FROM="Smart Hawk <hawk@yourdomain.com>"
```

Or bare address (no quotes needed):

```env
RESEND_FROM=hawk@yourdomain.com
```

Do **not** leave `RESEND_FROM=` empty in the shell/environment — empty vars block `.env` values.
| `CHROMIUM_PATH` | no | Browser fallback binary |
| `WORKER_POOL_SIZE` | no | default `4` |
| `SCHEDULE_POLL_SECS` | no | default `30` |

## Docker

```bash
cp .env.example .env
# fill TELEGRAM_BOT_TOKEN + SMART_HAWK_MASTER_KEY

docker compose up -d --build
docker compose logs -f smart-hawk
```

SQLite lives on the `smart-hawk-data` volume (`/data` in the container).  
Browser fallback is off in the default image (no Chromium); HTTP fetch still works.

Stop:

```bash
docker compose down
```

## Security

- API keys are **encrypted at rest** (AES-256-GCM); plaintext never logged
- After key input the bot tries to **delete** the user's message
- Crawl URLs pass **SSRF** checks (no private/link-local/metadata IPs)
- Hard delete cascades all user rows

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## License

MIT — see [LICENSE](./LICENSE).
