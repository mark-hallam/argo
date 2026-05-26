# Round 84 — PR #2391 SPA `ChannelStripCard` for Claim 6 multi-channel proof — PRE-MERGE PASS

**Date**: 2026-05-26 (UTC) · Melbourne 19:22 AEST
**Round**: 84
**Pillar**: Claim 6 (Multi-channel + multi-tenant) · web SPA proof surface
**PR**: [#2391](https://github.com/mark-hallam/argo/pull/2391) — `fix: ChannelStripCard for Claim 6 multi-channel proof surface`
**Head SHA**: `271fc91bcee8d834982e7f85246a5beaf2f0d090`
**Status**: OPEN · mergeable=UNKNOWN · +439/-0 across 4 files (claude-authored)

## Verdict

🟢 **PRE-MERGE PASS — P2 closure** (frontend proof-surface for the multi-channel claim — the worker-detail overview tab now aggregates last-7-days audit rows by channel + links to per-channel `/audit?worker=X&channel=Y` drill-ins, which is the v1.0 demo "Mariah pinged your Slack 4× and Email 2× last week" beat).

## Setup

- Fetched PR head as `pr-2391`.
- Inspected all 4 changed files (read via `git show pr-2391:<path>`):
  - `web/components/skycot/worker/ChannelStripCard.tsx` (NEW, +241 LOC) — the card component itself. Wraps `useAuditFeed({ worker, range: "7d" })`, aggregates rows via the new `aggregateByChannel()` helper, renders one row per channel with count + lastSeen + icon + drill-in link, or falls back to a connect-CTA pointing at `/settings/connectors` when only the `web` channel has activity.
  - `web/components/skycot/worker/ChannelStripCard.test.tsx` (NEW, +150 LOC) — 7 vitest cases.
  - `web/components/skycot/worker/WorkerOverviewContent.tsx` (+9/-0) — mounts the card on the overview tab.
  - `web/lib/api/audit.ts` (+39/-0) — adds the `ChannelAggregate` type + `aggregateByChannel()` helper that buckets `AuditRow[]` by `channel`, sorts count desc then lastSeen desc, folds rows missing a channel into the `web` bucket.
- Cherry-checked-out the 4 files into the live worktree, ran `pnpm exec vitest run components/skycot/worker/ChannelStripCard.test.tsx`, then `git restore`'d + `rm`'d the new files to leave the worktree clean.

## Observed

Vitest run: **7 passed / 7 total in 1.01s** (no cold compile — vitest hot).

```
✓ ChannelStripCard > renders one row per channel with counts, sorted by count desc (24ms)
✓ ChannelStripCard > links each row to /audit?worker=<id>&channel=<channel> (3ms)
✓ ChannelStripCard > renders the connect-CTA when only the web channel has activity (2ms)
✓ ChannelStripCard > renders the connect-CTA when there is no activity at all (2ms)
✓ aggregateByChannel > groups rows by channel, sorts count desc (1ms)
✓ aggregateByChannel > folds rows missing a `channel` into the web bucket (0ms)
✓ aggregateByChannel > returns the most recent timestamp for each channel as lastSeen (0ms)
```

Component shape:
- 7 supported channels (`slack`, `imessage`, `sms`, `push`, `web`, `voice`, `whatsapp`) — matches the `ChannelKind` type from `@/lib/sample-approvals`.
- 6 are "non-web" (everything except `web`). The card switches to the connect-CTA fallback when `hasNonWebChannel === false` — i.e. the worker has only acted on the SPA's own inbox.
- Each channel row carries `data-testid="channel-row-<channel>"` + `data-testid="channel-count-<channel>"` + `data-testid="channel-row-link-<channel>"` test seams.
- Drill-in URL shape: `/audit?worker=<id>&channel=<channel>` — the audit page's existing `AuditFilterChipRail` already supports `channel` as a multi-select filter dimension per the inline comment, so the round-trip should work without further audit-page work.
- The `aggregatesOverride` prop is a documented test seam ("bypasses `useAuditFeed` so unit tests can assert rendered output against a hand-rolled aggregate list") — the production path always goes through `useAuditFeed`.

## Anti-theater check

**Sufficient, with one limitation noted.**

- The 3 `aggregateByChannel` tests are pure unit tests against the helper function — they assert sort order (count desc tiebroken by lastSeen desc), the `web` fallback for rows missing a `channel`, AND the `lastSeen` max-of-bucket semantics. Each assertion targets a different code branch; the helper cannot pass all 3 by hand-rolling a constant.
- The 4 `ChannelStripCard` rendering tests use `data-testid` selectors that are tied to the component's actual JSX structure (`channel-row-slack`, `channel-count-slack`, `channel-row-link-slack`, `channel-strip-empty`, `channel-strip-connect-cta`). The "links each row to /audit?worker=..." test asserts the exact href string for two different channels — a constant-return mock would not produce per-channel URLs.
- Connect-CTA tests exercise both the "only `web` channel" branch AND the "empty aggregates" branch — proves the empty-state logic at line ~95-105.

**Limitation (worth noting, not a blocker)**: the test suite uses `aggregatesOverride` to bypass `useAuditFeed`, so it does not exercise the real API integration (fetch → render). The audit-page side of `/audit?worker=X&channel=Y` is not tested by this PR — that's expected (out of scope) but means a regression in `AuditFilterChipRail`'s `channel` handling could silently break the drill-in without this test catching it. Recommend the demo runbook's pre-record checklist click through one channel-row link manually to verify the round-trip.

## Gap / follow-up brief candidates

None for this scope. One optional belt-and-suspenders if the conductor wants extra Claim-6 visual coverage post-merge:

1. **`web-channel-strip-drillin-e2e-pin`** (P3, optional) — Playwright spec that boots demo seed data with a worker that has audit rows on `slack` + `email`, lands on `/workers/<id>`, asserts the `ChannelStripCard` shows both rows, clicks the Slack drill-in link, asserts `/audit?worker=X&channel=slack` renders only Slack rows. Closes the test gap noted above. Could land as a `tests/e2e/claim-6-channel-drill-in.spec.ts` file under existing Playwright setup. Not a v1.0 demo blocker.

Recommend conductor proceed with admin-merge using `conductor-pre-merge-gate.sh --skip-clippy` per operator-locked evaluator-pre-verified authorization. Vitest PASS within 5 min of this report; cold compile not required (web-only PR, vitest hot-runs against the dev's existing node_modules cache).

**Claim-6 significance.** Backend wiring for the multi-channel claim landed long ago (cross-tenant audit rows + per-channel routing + Discord/SMS/voice inbound). What was missing was the *visible* proof surface on the worker drill-in — a screenshot the v1.0 demo MP4 can frame to back the "Mariah lives across Slack + email + SMS" claim. This PR provides it.
