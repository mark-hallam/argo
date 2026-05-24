# Stream mission-demo-seed-data-bootstrap — populate the workspace with the voiceover-script personas

**Owner**: claude.
**Depends on**: `fix-web-mock-bypass-env-var-rename` MERGED, `fix-web-workers-api-envelope-and-id-format` MERGED.
**Authored against**: origin/main `f3d1c66a6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-data-bootstrap`.
🔴 **P0 DEMO-BLOCKER.** The v1.0 demo voiceover (`docs/skycot/operator/demo-runbook.md`) references specific data that doesn't exist in a fresh workspace:
- 6 named teammates (Mariah, Priya, Jamal, Nori + 2 more)
- 7 pending approvals on `/`
- 1,122 weekly actions / 72 hours saved KPIs on `/team`
- Mariah's specific personality card + responsibilities + 3 today-KPIs on `/workers/{id}/overview`
- Populated audit chain on `/audit`

Currently the workspace returns `{workers: [], total_count: 0}` for /team, empty approvals, no audit rows. The demo voiceover doesn't match the on-screen reality.

## Scope

Two paths; implementer picks based on which makes the demo more honest:

**Option A (recommended) — Seed via mock-bypass mode.** Make the existing `web/lib/sample-*` fixtures (sample-personas, sample-approvals, sample-audit, sample-workflows) richer + voiceover-aligned. The operator runs the demo with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` set, the mock fallback fires, the demo shows the SAMPLE_TEAM data. Quick + visible-to-audience-as-demo. No real backend seeding.

**Option B — Real seed via Rust binary + gateway.** Add a `argo demo seed` CLI subcommand that inserts the 6 workers + 7 approvals + 1000 audit rows into the live workspace. Honest end-to-end demo. More work.

Pick A. The demo is a marketing artifact; mock-bypass is correct here.

For Option A:
1. Audit existing `web/lib/sample-personas.ts` — what's already there?
2. Expand to include 6 personas matching the voiceover (Mariah + Priya + Jamal + Nori + 2 more — use the voiceover line "These are real AI teammates — Mariah, Priya, Jamal, Nori." as the canonical 4, then 2 more for the "6 teammates" count)
3. Each persona needs: ULID worker ID (post-`fix-web-workers-api-envelope-and-id-format`), display_name, role, vertical, headline, traits, status, lastActiveAt, pendingApprovalCount
4. `web/lib/sample-approvals.ts` — 7 pending approval rows, each pointing at one of the personas, with realistic action shapes
5. `web/lib/sample-worker-overview.ts` — Mariah's full detail card: 4-trait personality (Patient, Detail-oriented, Picture-perfect listings, Polite), "great at" list (spotting price gaps, cropping photos, writing tight 60-word copy), "when she escalates" (price drops below floor, edited-looking photos, multiple competing offers), today's KPIs (3 listings live, 11 buyer replies, 2 items flagged below floor)
6. `web/lib/sample-audit.ts` — 1000+ audit rows (synthetic, realistic kinds + timestamps + workers + statuses)
7. `web/lib/sample-team.ts` aggregate stats: `weeklyActions=1122` and `hoursSaved=72` for the hero KPIs

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/lib/sample-personas.ts` — expand to 6 voiceover-aligned personas
- `web/lib/sample-team.ts` — aggregate stats
- `web/lib/sample-approvals.ts` — 7 pending approval rows
- `web/lib/sample-worker-overview.ts` — Mariah's full detail card
- `web/lib/sample-audit.ts` (or wherever audit mocks live) — 1000+ rows
- `web/lib/sample-workflows.ts` — verify `qualify_lead` exists with the React Flow nodes the voiceover demos
- `docs/skycot/operator/demo-runbook.md` — confirm the voiceover script's data references match the seeded data (one-line edits if a name needs harmonization)
- `web/lib/sample-*.test.ts` — update tests for the new persona inventory

## Files you must NOT touch

- Any `crates/argo-*/**` file — option A is web-side seed data, not real backend seeding
- Any `web/app/(app)/` page file — the pages already render whatever the mock returns
- `web/lib/api/*.ts` — the fetchers are correct; this brief is about the data they return

## Acceptance criteria

- [ ] With `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` set, `/team` renders 6 teammate cards with the voiceover-canonical names (Mariah, Priya, Jamal, Nori + 2 more)
- [ ] `/` (approvals) shows 7 pending rows
- [ ] `/team` hero shows "6 teammates", "1,122 actions this week", "72 hours saved"
- [ ] `/workers/{Mariah-ID}/overview` shows Mariah's full personality card matching the voiceover script line-by-line (traits, great-at, escalate-when, today's KPIs)
- [ ] `/audit` shows >100 audit rows (not empty)
- [ ] `/workflows` shows the `qualify_lead` workflow as a clickable tile
- [ ] `/workflows/sales/qualify_lead/build` renders the React Flow canvas with nodes the voiceover walks through
- [ ] `pnpm typecheck` clean
- [ ] `pnpm test` — no regressions; sample-*.test.ts updated
- [ ] PR body includes a screenshot of `/team` after seeding (operator-readable)
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-demo-seed-data-bootstrap`
- PR title: `web(demo): seed sample fixtures with voiceover-aligned 6 personas + 7 approvals + 1k audit rows (mission-demo-seed-data-bootstrap)`

## Notes for the implementer

- This brief is the BIG one (~600-1200 LOC, multi-file). It depends on the env var rename + worker ID format briefs landing first; otherwise the mock-bypass won't fire and the new fixtures won't be visible.
- Take the demo voiceover script (`docs/skycot/operator/demo-runbook.md`, lines 44-117) as the source of truth for every name + number + quote. The demo is recorded against the seeded data, so the data must literally match the voiceover.
- If you encounter ambiguity (e.g. voiceover names 4 teammates but says "6 teammates" — what are the other 2?), pick names + roles consistent with the existing SAMPLE_PERSONAS aesthetic and document in the PR.
- The 1000+ audit rows can be synthetic — repeated patterns with varied timestamps + workers + kinds. Doesn't need to be perfectly realistic; just needs to look populated and varied on screen.
- Once this lands + the prerequisites land + the operator records the demo MP4, the v1.0 ship gate closes.
