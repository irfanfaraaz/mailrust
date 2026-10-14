# Session refresh — continue the dashboard redesign and motion work

Continuing work in /Users/irfanfaraaz/Developer/Projects/web2/mailrust (Mailrust: Rust engine + Next.js dashboard in `web/`), branch `ui/field-notes-look`. Goal of this session: pick up right where it stopped. Wait for the user's pick of which new motion ideas to build, keep PR #19 green, and keep the motion preview artifact current.

## Where things stand

- **Redesign done and in a PR:** https://github.com/irfanfaraaz/mailrust/pull/19 (ui/field-notes-look → main). Commits: c7d416c, 008b5f3, bd0f125, ad22c27, 2e98de4, 0d1ff59, f9dab7b (finish-review fixes), bbc2df1 (DESIGN.md + PRODUCT.md), dc4a89d (motion + delight), 0833aae (lint fix: milestone note uses useSyncExternalStore).
- **Auto-fix is ON for PR #19** in the desktop app. CI failures arrive as `<ci-monitor-event>`; fix, verify, commit, push without asking. CI web job = gen:types diff, typecheck, lint, test, build. Last local run: all clean (lint has 4 old warnings, 0 errors).
- **12 motion/delight ideas are built** (dc4a89d): stat cards rise + count up (`web/components/count-up.tsx`), meters fill (`.meter-fill`), stamp lands on status change (`web/components/stamp.tsx`, re-exported from `status-badge.tsx`), sliding sidebar marker (`web/components/sidebar.tsx`), autosave tick ("Saving…" → tick "Saved" → "All changes saved"), dialogs rise in (`web/components/ui/dialog.tsx`), Sent stamp after Send now (`web/app/a/[account]/campaigns/[id]/sent-celebration.tsx`, handed over via sessionStorage + `mr:sent` event), drawn empty-state envelope (`web/components/empty-state.tsx`), card lift (`.lift`), "Copied" label (`web/components/copied-hint.tsx` in 3 copy buttons), pen underline under page titles in group colour (`web/components/title-scribble.tsx`, first visit per page load), milestone notes on Overview (`web/components/milestone-note.tsx`; first issue sent, 100 / 1,000 readers, 1,000 sent this week; close stored in localStorage). All motion CSS is at the end of `web/app/globals.css`, moving parts inside `prefers-reduced-motion: no-preference`.
- **Motion preview artifact v2:** https://claude.ai/artifact/H5yjTtw4tx9uL2Xq4HeP3q (source `.scratch/look-lab/mailrust-motion-preview.html`, gitignored; republish the same path to keep the URL). It marks the 12 built ideas "In the app" and shows 14 new ideas from /impeccable layout, bolder, animate, delight, overdrive (user said "show all" three directions), polish: wave bar, log rows arrive, odometer numbers, paper-slip toasts, kind unsubscribe page with undo, test email drops in slot, pause banner with red pen circle, one-time "/" hint, issue masthead, ledger tables, health seal, morph-from-button (View Transitions), live wave send map (canvas), ink and paper physics (SVG filters + spring drag). Not verified live in a browser (pane could not load claude.ai); script passed `node --check`.
- **Editor "bug" was not a bug:** the hidden in-app browser pane never fires requestAnimationFrame, so React never revealed the page. With the pane visible the editor renders, types and autosaves. The cloud task task_02b4997d for it is not needed (user can stop it). A test line was typed into the seed draft "Notes on one-click unsubscribe".
- **Picks locked earlier:** Kami base, light, Parchment, Carrier navy, Source Serif + Geist, colour tab per group, Sketch cards; wave=one bar, charts=area, numbers=mono, nav icons on, comfortable, soft corners, stamp status, UPPERCASE labels.

## Next steps
1. Wait for the user to say which of the 14 new ideas to build; then build them into `web/`, typecheck + lint, verify in a VISIBLE browser tab (preview_start with the URL; hidden pane breaks hydration), commit by file name, push to PR #19.
2. Handle any `<ci-monitor-event>` for PR #19 (Auto-fix authorization).
3. If the user reports a broken demo in the artifact, fix that one and republish the same file path.
4. Optional, only if asked: stop dev servers, update DESIGN.md with the motion rules, merge the PR.

## Files to open (read these, don't re-derive)
- `DESIGN.md` — the shipped design system (tokens, Sketch cards, Stamp, named rules).
- `PRODUCT.md` — users, purpose, information architecture.
- `.scratch/look-lab/mailrust-motion-preview.html` — the motion preview artifact source (all 26 ideas, demo code to port).
- `web/app/globals.css` — tokens and all motion classes (end of file).
- `web/app/a/[account]/overview/page.tsx` — stats, health, latest issue, milestones wiring.
- `web/components/sidebar.tsx` — sliding marker logic.
- `.impeccable/surfaces/web-app-a-account.md` — direction contract for the dashboard surface.

## Local dev
- Env: `set -a; . ./.env; set +a` (MAILRUST_SENDER=fake, OUTBOX_DIR=./outbox). Engine: `./target/debug/mailrust serve` on :8080. Web: `npx next dev -p 3000` in `web/` (stale CSS → `rm -rf web/.next`, restart).
- Test account: http://localhost:3000, user maya@fieldnotes.test; magic link lands in `outbox/*.eml` (quoted-printable). Account id 01a13a56-38fd-7224-b6aa-f60d2f676cd8.

## Standing rules
- Never commit `docs/superpowers/`, `docs/learning/`, `.impeccable/`, `.scratch/`, or this file. Stage files by name. Only the main agent runs git. No worktrees. Agents in background. No Playwright unless asked. TDD only for Rust.
- Inspiration, never copy. ELI5 / STE reply style with `##` headings + `---`, `•` bullets. Say every planned write before writing.
