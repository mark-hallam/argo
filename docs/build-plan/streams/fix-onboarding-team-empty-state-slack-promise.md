# fix-onboarding-team-empty-state-slack-promise

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-team-empty-state-slack-promise`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F13 (P2).

🟡 **P2 — `/team` SSR empty-state literally renders "No teammates yet. Pick a vertical and they'll be in your Slack in four minutes." This is a Slack-shaped promise the v1.0 demo does not deliver (no Slack integration runs during the recorded demo). The first thing a fresh-onboarded operator sees is a promise the product won't keep.**

## Root cause

The `/team` empty-state copy at the SSR layer hard-codes Slack as the delivery channel, but the v1.0 demo path (and the MVP product) does not auto-wire Slack to a newly-hired worker. Channels are described in `/settings/notify-channels` as "11 approval channels ship with SKYCOT — email, chat, SMS, voice, push, and the web inbox" — not "your Slack." The empty-state copy was authored against an aspirational Slack-first onboarding that doesn't exist yet.

This is dressing-side (copy-only) but it's the first impression after the hire wizard completes if the post-hire readback fails (which is the honest-mode default — see `fix-hire-demo-mode-stub-uuid-worker-roster-invisibility` + `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`).

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F13 [P2] — `/team` empty-state copy "in your Slack in four minutes" is a promise the demo doesn't deliver

## Fix recipe

1. Locate the empty-state copy in `web/app/(app)/team/page.tsx` (search "No teammates yet" or "in your Slack").
2. Replace with channel-agnostic language that matches the v1.0 surface:
   > "No teammates yet. Pick a vertical to hire one — they'll be reachable via your web inbox, email, and the channels you connect in Settings."
3. Cross-reference `/settings/notify-channels` copy so the language matches.
4. If a Slack-connection state is detected (`/api/v1/integrations/slack` returns connected), branch the copy to "in your Slack within four minutes" — but only when the connection is real.

## Acceptance criteria

- [ ] `rg "in your Slack" web/app/(app)/team/` returns zero matches (or a single match conditional on a connected-slack check).
- [ ] `/team` empty-state copy matches what `/settings/notify-channels` advertises.
- [ ] A fresh operator with no Slack connection sees channel-agnostic copy.

## May-touch

- `web/app/(app)/team/page.tsx` (~5 LOC copy edit)

## Out of scope

- Building the Slack auto-wire flow (separate v1.x feature).
- The deeper post-hire 404 root cause (see `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`).

## Effort

XS — single-line copy edit, ~5 minutes. P2 because it's dressing the camera doesn't drill into during Scene 3, but a buyer trialing the product post-demo will see it and flag the promise as broken.
