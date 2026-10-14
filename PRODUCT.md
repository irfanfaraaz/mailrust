# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

- People who run their own newsletter and their own app email from one place: solo creators, indie developers and small teams.
- Two jobs, one person often doing both: writing and sending a newsletter (beehiiv side), and sending transactional email from code (Resend side).
- Not a developer tool first. The writer must feel at home; the developer pages stay precise.

## Product Purpose

Mailrust is a self-hosted email platform with two doors on one Rust engine: a newsletter app (like beehiiv) and a sending API (like Resend). All mail goes through the owner's own Amazon SES account, so sending costs about $0.10 per 1,000 emails. Success: a writer drafts, sends and reads the report of an issue with confidence, and a developer wires up the API without leaving the same app.

## Positioning

- Self-hosted first (main goal), and also offered as a cloud service like beehiiv.
- One engine, two doors: newsletter and transactional API share domains, suppressions, team and sending health.
- Safe sending built in: waves with the most engaged readers first, auto-pause over 2% bounces or 0.08% complaints, one-click unsubscribe, priority lanes so login codes beat newsletters.
- Postgres is the queue. No Redis, no extra services.

## Operating Context

- Inspiration and rivals: Mailcheap (Marc Lou), beehiiv, Resend, listmonk, Plunk, useSend.
- Dashboard: Next.js + shadcn (Base UI) + Tailwind v4, Tiptap block editor, Sonner toasts, lucide icons.
- Magic-link login, accounts with owner/member teams, account switcher.

## Capabilities and Constraints

- Information architecture (confirmed 2026-10-14): one dashboard, one sidebar. Newsletter group on top (campaigns/posts, contacts/audience, tags, signup form, archive), Sending API group below (emails log, API keys, webhooks), Settings at the bottom (domains, suppressions, team). Overview shows both sides.
- Public pages: issue archive `/p/<slug>`, unsubscribe `/u/<token>`, signup confirm, login, invite.
- Campaign states: draft, scheduled, sending (waves), paused, sent, cancelled. Account can be paused by the engine.
- The core features exist; design work treats this as a finished app, every screen in scope.

## Brand Commitments

- Original look, never a copy (user, 2026-10-14). Other products (Resend, beehiiv, Linear, Stripe and the rest) are inspiration only. Mailrust gets its own palette values, type pairing, layouts and signature details; no brand's exact colour set, component styling or page layout is reproduced.

## Evidence on Hand

- No real customers, testimonials or metrics yet. Do not invent them.

## Product Principles

1. The writer comes first; the developer is never second-class.
2. Sending health is always visible: a paused account or a sending wave is never hidden.
3. One app, shared truth: domains, suppressions and team live once.
4. Self-hosted must not look self-hosted: production-grade polish on every screen.
