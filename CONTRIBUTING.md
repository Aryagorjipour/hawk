# Contributing to Smart Hawk

Thanks for helping the hawk fly better.

## Dev setup

```bash
git clone https://github.com/Aryagorjipour/hawk.git
cd hawk
cp .env.example .env
# set TELEGRAM_BOT_TOKEN + SMART_HAWK_MASTER_KEY (openssl rand -hex 32)
cargo run
```

## Checks before a PR

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## Architecture

Hexagonal layers: `domain` → `ports` ← `application` ← `adapters` / `bootstrap`.  
Prefer domain/application changes over Telegram-specific logic when possible.

## Pull requests

1. Fork / branch from `main`
2. Keep PRs focused
3. Describe *why* and how you tested
4. Do not commit `.env`, API keys, or local DBs

## Security

See [SECURITY.md](./SECURITY.md). Never open an issue with real user secrets.
