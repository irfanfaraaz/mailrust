# Mailrust

Self-hosted email platform with two doors: a sending API (like Resend) and a newsletter app (like beehiiv, coming next). One Rust engine sends everything through your own Amazon SES account.

- **Sending API** — `POST /emails`, batch, scheduled send, idempotency keys, attachments, webhooks, open/click tracking.
- **Safe for many accounts** — each account is its own SES tenant with its own block list, and Mailrust pauses an account itself when bounces go over 2% or complaints over 0.08%.
- **No extra services** — Postgres is the queue (`FOR UPDATE SKIP LOCKED`). No Redis.

## Quick start (no AWS needed)

```bash
docker compose up -d postgres
cp .env.example .env
cargo run -- migrate
cargo run -- account create --name "My app"              # prints the account id
cargo run -- key create --account <account-id> --name dev # prints the API key once
cargo run -- domain add --account <account-id> example.com
cargo run -- serve
```

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

## Deploy

One small server runs everything: `deploy/docker-compose.prod.yml` starts Postgres, Mailrust, Caddy (automatic HTTPS, including customers' tracking subdomains) and a nightly `pg_dump` backup.

```bash
cd deploy
docker compose -f docker-compose.prod.yml --env-file ../.env up -d
```

## Development

```bash
cargo test                       # needs the dev Postgres from docker compose
cargo sqlx prepare -- --all-targets   # after changing SQL; commit .sqlx/
```

## License

AGPL-3.0. See [LICENSE](LICENSE).
