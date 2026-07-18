# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| `main` / latest release | ✅ |

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security bugs.

Email or message the maintainer privately (GitHub: [@Aryagorjipour](https://github.com/Aryagorjipour)) with:

- Description of the issue
- Steps to reproduce
- Impact assessment
- Optional patch / PoC

We aim to acknowledge reports within a few days.

## Operator notes

- Keep `SMART_HAWK_MASTER_KEY` and `TELEGRAM_BOT_TOKEN` secret
- Never log user API keys (the code redacts them; don’t re-introduce logging)
- Prefer HTTPS-only custom LLM base URLs
- Rotate the master key by re-encrypting stored secrets if compromised
