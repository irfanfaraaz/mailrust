# Changelog

All notable changes to this project are listed here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] - 2026-10-14

### Added

- Delete a draft from its page.
- Issue report: Delivered and Marked as spam totals, with a complaint rate.
- Every time in the dashboard and the public archive shows in the viewer's own time zone, labelled (for example 14:03 GMT+5:30).
- Overview: below the auto-pause minimums (100 emails for bounces, 1,250 for complaints) the bars stay grey and say "Not checked yet". `GET /stats/overview` returns `bounce_min_sent` and `complaint_min_sent`.

### Fixed

- Autosave failed with "missing field `attrs`" as soon as an issue had a button, image, heading or merge tag.
- Images did not show in the issue preview.
- Overview showed the two rates with different decimals.

## [0.3.0] - 2026-10-14

### Added

- Campaigns: write an issue in a block editor (headings, lists, quotes, images, buttons, dividers, merge tags with fallbacks) with a `/` menu, autosave, preview and test sends to team members.
- Sending in growing waves (100, 200, 400, … up to 10,000, 10 minutes apart), most engaged readers first, to everyone subscribed or to chosen tags; send now or schedule. One issue sends at a time per account.
- Hard checks before a send: verified from address, subject, postal address, size under Gmail's 100 KB clip, active account, at least one reader, valid blocks.
- Pause, resume and cancel; an auto-paused account stops its issue and nobody gets it twice, also across a restart.
- `List-Unsubscribe` and one-click `List-Unsubscribe-Post` headers on every issue; unsubscribes count per issue.
- Per-issue report: totals, unique open and click rates (mail scanners left out), clicks per link, a split per mailbox provider and wave progress.
- Public archive at `/p/<slug>` with a strict CSP, and image uploads (PNG, JPEG, GIF, WebP up to 5 MB) served from `/img/<id>`.
- Postal address and an "Ask for first name" switch in the newsletter settings; the confirm page names the newsletter; the contacts list shows the total.
- Endpoints for `/campaigns`, `/images` and `/public/archive`, all in OpenAPI.

## [0.2.0] - 2026-10-13

The dashboard and the subscriber side of the newsletter.

### Added

- Contacts with status `pending`, `subscribed` or `unsubscribed`, first and last name, and free JSON fields. One audience per account, plus tags.
- Public signup form at `/join/<slug>` with double opt-in, plus an embed snippet. The form gives the same answer for every address and has per-IP and per-address limits.
- One-link unsubscribe at `/u/<token>`, including RFC 8058 one-click POST. Unsubscribe changes the contact status only, not the suppression list.
- Background CSV import (up to 100,000 rows or 20 MB) that resumes after a crash, and CSV export that defuses spreadsheet formulas.
- Endpoints for contacts, tags, imports, newsletter settings and `/public/*`, all in OpenAPI.
- Dashboard pages for contacts, import, tags and the signup form.
- Dashboard (`web/`) with magic-link login, teams and invites (owner/member), and the door 1 pages.
- `GET /stats/overview`, `/emails` filters (`priority`, `since`, `until`, `q`) and `/suppressions?q=`.
- CLI `user invite` and `openapi`.
- `mailrust-web` Docker image, a `web` service in `docker-compose.prod.yml` and `app.<domain>` in the Caddyfile.

### Changed

- Each CI check runs once per PR.

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

[Unreleased]: https://github.com/irfanfaraaz/mailrust/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/irfanfaraaz/mailrust/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/irfanfaraaz/mailrust/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/irfanfaraaz/mailrust/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/irfanfaraaz/mailrust/releases/tag/v0.1.0
