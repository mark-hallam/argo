# rebase-fix-onboarding-two-roster-uuid-space-fixtures-vs-gateway

Re-implement the original brief
`streams/fix-onboarding-two-roster-uuid-space-fixtures-vs-gateway.md`
against current `origin/main` HEAD.

## Original PR

PR #2575 (closed for re-author) — `fix: SPA two-roster UUID drift —
align fixtures on gateway-seeded canon (F01 + F14)`, created 2026-05-27
against an older `main`. The PR touched 34 files (404 additions / 150
deletions, including stories + e2e + tests + 9 sample-* files +
sample-personas spine). Conflicted on:

- `web/lib/sample-personas.ts` — main has had multiple downstream
  edits since (`8ca479789` folded persona traits + level + personality
  onto sample-personas spine, `720a33bba` added pronouns field). The
  PR's new UUID literals on every persona need to land against the
  current spine shape.
- 33 other files referencing the old `018f67d0-0000-7000-8000-…`
  UUIDs (escalation page SEEDS map, CommandPalette palette hrefs, e2e
  specs, sample data files). Each needs a separate UUID swap. Some
  of these files have evolved (CommandPalette has had palette
  reshuffles; sample-cohorts has D3 magazine tile data updates).

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-onboarding-two-roster-uuid-space-fixtures-vs-gateway

## Bug is still load-bearing

Confirmed on `a5df90c41`:

- `web/lib/sample-personas.ts` still has the `018f67d0-0000-7000-8000-`
  UUID space (lines 92, 131, 170, 208, 228). The gateway-seeded canon
  uses `019e6785-928*-…` UUIDs.
- With `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, `fetchTeam()` returns
  fixture UUIDs; without bypass, the gateway returns its own UUIDs.
  Clicking any worker link in the wrong mode 404s.
- The demo-mode first-hire wizard cannot reach Peta because
  `SAMPLE_PETA` doesn't exist (only `SAMPLE_MARIAH/PRIYA/JAMAL/NORI`)
  and `project-coordination` isn't in `VERTICAL_TO_SAMPLE_WORKER`.

## Acceptance (same as original brief)

- All 5 `SAMPLE_*` constants use the gateway-seeded UUIDs:
  - Mariah → `019e6785-928a-7270-a766-f945b6cd7161`
  - Peta   → `019e6785-928c-72b3-85c6-436fe5004c50`
  - Priya  → `019e6785-928d-78b1-a6b2-136d9ed263ce`
  - Jamal  → `019e6785-928e-7f20-803b-55266e526011`
  - Nori   → `019e6785-928f-7281-bf8d-e062bc277478`
- `SAMPLE_PETA` exported from `sample-personas.ts` (single source of
  truth) and re-exported from `sample-workers-detail.ts`.
- `VERTICAL_TO_SAMPLE_WORKER` includes `project-coordination →
  SAMPLE_PETA`.
- Non-canon mock workers (Sam, Rio if present) — move to a
  `018f67d1-…` prefix so the canon grep stays clean.
- `grep -rn "018f67d0-0000-7000-8000" web/lib/` returns NO
  production-code matches (only test fixtures + in-doc grep-pattern
  references).
- New vitest `web/lib/onboarding/use-first-hire.test.ts` covers the
  `project-coordination → Peta` mapping (4 tests).

## Shape of main on touched files

`web/lib/sample-personas.ts` is the canonical spine. After
`8ca479789` it carries persona traits + level + personality fields,
and after `720a33bba` it carries pronouns. The rebase needs to
preserve those fields while swapping just the `id:` literal on each
persona block (lines 92, 131, 170, 208, 228).

The 9 other `sample-*.ts` files reference the old UUIDs in:

- `sample-team.ts` — workforce list keyed by persona UUID
- `sample-cohorts.ts` — D3 magazine tile data with worker UUID
- `sample-audit.ts` + `sample-audit-detail.ts` — audit rows
- `sample-worker-overview.ts` / `-trust.ts` / `-trust-history.ts` /
  `-worker-activity.ts` / `-worker-skills.ts` / `-worker-memory.ts` /
  `-workers-detail.ts` — per-worker fixtures

Each needs a `sed`-style swap from old → new UUID. The post-`8ca479789`
spine consolidation means some files may now re-export from
`sample-personas.ts` rather than carry their own literals — verify
before swapping.

Two production files outside `web/lib/` also need swaps:

- `web/app/(app)/workers/[workerId]/escalation/page.tsx` — `SEEDS` map
  keyed by worker UUID.
- `web/components/skycot/CommandPalette.tsx` — `/workers/<uuid>`
  palette hrefs.

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` uses
opaque `act_<UUID>_<seq>` composite action ids that don't resolve
through `/workers/<id>` — explicitly OUT OF SCOPE.

## Notes for the worker

- Harvest live gateway UUIDs to confirm the table above is still
  correct: `argo gateway start --bind 127.0.0.1 --port 7400` then
  `curl http://127.0.0.1:7400/api/v1/workers | jq '.workers[] | {id,name}'`.
  If the gateway-seeded UUIDs differ from the table above, the
  brief's UUID literals are wrong — use the live values.
- Pre-existing test flakes
  (`__tests__/approvals/page.test.tsx`,
  `tests/approvals-ops-dashboard-cards.test.tsx`) are not in scope —
  per `SWARM_PROTOCOL.md` HARD RULE 2.
- `tsc --noEmit` + `next lint` + `eslint .` + targeted vitest is the
  verification floor.
