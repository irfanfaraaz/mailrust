# Mailrust

[![ci](https://github.com/irfanfaraaz/mailrust/actions/workflows/ci.yml/badge.svg)](https://github.com/irfanfaraaz/mailrust/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/irfanfaraaz/mailrust)](https://github.com/irfanfaraaz/mailrust/releases)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

Self-hosted email platform with two doors: a sending API (like Resend) and a newsletter app (like beehiiv). One Rust engine sends everything through your own Amazon SES account.

- **Sending API** — `POST /emails`, batch, scheduled send, idempotency keys, attachments, webhooks, open/click tracking.
- **Safe for many accounts** — each account is its own SES tenant with its own block list, and Mailrust pauses an account itself when bounces go over 2% or complaints over 0.08%.
- **Smart sending** — `"priority": "urgent" | "normal" | "bulk"` puts login codes ahead of newsletters. The send rate follows your SES limits, a hit on the SES daily cap pauses sending instead of failing emails, and the last 10% of the daily quota is kept for urgent mail.
- **Newsletter** — subscribers with double opt-in, tags, CSV import, a block editor, sending in growing waves, one-click unsubscribe, per-newsletter reports and a public archive.
- **Dashboard** — magic-link login, teams and invites (owner/member), emails log, API keys, webhooks, domains and suppressions in a Next.js app.
- **No extra services** — Postgres is the queue (`FOR UPDATE SKIP LOCKED`). No Redis.

## Quick start (no AWS needed)

```bash
docker compose up -d postgres
cp .env.example .env                                   # dev: also uncomment DEV_LOG_LINKS=true
cargo run -- migrate
cargo run -- account create --name "My app"              # prints the account id
cargo run -- key create --account <account-id> --name dev # prints the API key once
cargo run -- domain add --account <account-id> example.com
cargo run -- serve
```

```bash
# Dashboard
cargo run -- user invite you@example.com --account <account-id> --role owner
# in .env: SYSTEM_ACCOUNT_ID=<account-id>  SYSTEM_FROM=login@example.com  (a verified domain of that account)
pnpm --dir web install && pnpm --dir web dev   # http://localhost:3000
```

With `MAILRUST_SENDER=fake` the login link lands in `./outbox`. Uncomment `DEV_LOG_LINKS=true` in `.env` (dev only, never in production) to also print it to the server log.

```bash
curl -s localhost:8080/emails \
  -H "Authorization: Bearer <key>" -H "Content-Type: application/json" \
  -d '{"from":"Hi <hi@example.com>","to":"bounce@simulator.amazonses.com","subject":"Test","text":"Hello"}'
```

With `MAILRUST_SENDER=fake`, emails are written to `./outbox/*.eml`, and the SES simulator addresses (`success@`, `bounce@`, `complaint@simulator.amazonses.com`) produce the same events SES would.

API reference: `GET /openapi.json`.

## Using Amazon SES

1. In SES, verify nothing by hand: Mailrust creates identities and tenants for you.
2. Create a configuration set (e.g. `mailrust`).
3. Create an SNS topic, add an HTTPS subscription to `https://<PUBLIC_URL host>/ses/events` (Mailrust confirms it automatically).
4. Add an event destination to the configuration set: SNS, that topic, event types Send, Delivery, Delivery delay, Bounce, Complaint, Reject.
5. Give the server's AWS identity these permissions: `ses:SendEmail`, `ses:CreateTenant`, `ses:CreateTenantResourceAssociation`, `ses:CreateEmailIdentity`, `ses:GetEmailIdentity`, `ses:PutEmailIdentityMailFromAttributes`.
6. Set `MAILRUST_SENDER=ses`, `AWS_REGION`, `SES_CONFIGURATION_SET`, `SNS_TOPIC_ARN`, then run `mailrust account sync-ses --all`.
7. Pricing: new SES accounts start on the Essentials plan ($0.16 / 1,000). À la carte is $0.10 / 1,000 plus $0.005 / tenant / month and $0.005 / 1,000 emails for tenants. Pick the plan in the SES console.
8. Ask AWS for production access before sending to real people.

## Use it from your AI

Your engine has an MCP endpoint at `/mcp`. Claude Code, Codex and Cursor can use it to run everything the dashboard does, with one of your API keys. Setup: [docs/mcp.md](docs/mcp.md).

## Deploy

One small server runs everything: `deploy/docker-compose.prod.yml` starts Postgres, Mailrust, the `web` dashboard (served at `app.<domain>`, set `MAILRUST_DOMAIN`, `WEB_URL` and `WEB_SECRET` in `.env`), Caddy (automatic HTTPS, including customers' tracking subdomains) and a nightly `pg_dump` backup.

```bash
cd deploy
docker compose -f docker-compose.prod.yml --env-file ../.env up -d
```

Set `MAILRUST_LANDING=true` for `web` to show a public landing page to signed-out visitors at `/`. `MAILRUST_LANDING=only` is for a showcase site with no engine (for example on Vercel): `/` is the landing page and every other path is a 404.

The reverse proxy in front of `web` must set `X-Forwarded-For` (Caddy does by default). Without it every reader shares one IP, so the signup form's per-IP limit (5 joins an hour) stops all signups. The engine logs a warning once when this happens.

## Development

```bash
cargo test                       # needs the dev Postgres from docker compose
cargo sqlx prepare -- --all-targets   # after changing SQL; commit .sqlx/
pnpm --dir web test              # dashboard unit tests
pnpm --dir web e2e               # dashboard end-to-end tests
```

## Contributing

Mailrust does not take pull requests yet. Bug reports and ideas are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md), and report security problems as [SECURITY.md](SECURITY.md) says.

Changes per version are in [CHANGELOG.md](CHANGELOG.md).

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
