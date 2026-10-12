# Changelog

All notable changes to this project are listed here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-13

First release: the sending API and the engine behind it.

### Added

- Sending API: `POST /emails`, `POST /emails/batch` (up to 100), get, list and cancel, with scheduled send (up to 30 days), idempotency keys and attachments.
- Postgres queue (`FOR UPDATE SKIP LOCKED`) with leases, a reaper, retries with backoff and a send limiter.
- Amazon SES sender: one SES tenant per account, automatic identities, DKIM and custom MAIL FROM records, and `mailrust account sync-ses`.
- SES event intake over SNS (signature check, auto-confirm), with delivery, bounce, complaint, reject and delay events.
- Per-account block list and auto-pause when bounces go over 2% or complaints over 0.08%.
- Customer webhooks with signing, retries and an SSRF guard.
- Open and click tracking, with custom tracking subdomains and automatic HTTPS through Caddy.
- Accounts, API keys and domains from the CLI, plus API key and suppression endpoints.
- OpenAPI reference at `GET /openapi.json`.
- A fake sender that writes `.eml` files and acts like the SES mailbox simulator, so nothing needs AWS to run.
- Docker image, `docker-compose.prod.yml` and nightly `pg_dump` backups.
- Priority lanes: `"priority": "urgent" | "normal" | "bulk"` on send and batch. Urgent mail is capped per account at max(100, 10% of its last 24 h); extra urgent mail is stored as `normal`.
- SES quota sync every 5 minutes (`GetAccount`). The send rate follows SES, capped by `SES_MAX_SEND_RATE`.
- Daily-cap handling: an SES "daily quota exceeded" answer pauses all sending for 15 minutes and uses no try; one probe email checks if sending works again.
- Urgent reserve: at 90% of the daily quota, only urgent mail goes out.

### Changed

- The worker claims only as many emails per round as it can send within half the lease.
- Dropped the unused `emails_account_urgent` index (migration 0004).

### Fixed

- No AWS SDK retries on the SES send call, so a timeout cannot send an email twice.
- The database clock stamps `send_at` for send-now emails.
- No repeat "suppressed" events for the same email.

[Unreleased]: https://github.com/irfanfaraaz/mailrust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/irfanfaraaz/mailrust/releases/tag/v0.1.0
