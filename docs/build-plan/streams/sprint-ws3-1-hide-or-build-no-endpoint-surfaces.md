# sprint-ws3-1-hide-or-build-no-endpoint-surfaces

**Owner**: claude or codex. **Frontend — no dead-end surfaces for a public user.**
**Depends on**: none.
**Authored against**: origin/main `5c6a82366` on `2026-06-04` (verified per gap-audit WS3: `retros.ts` / `retrain*.ts` / `duties.ts` reference sample data with NO mounted live endpoint).
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws3-1-hide-or-build-no-endpoint-surfaces`.

## Why

The gap-audit (WS3) found surfaces whose `web/lib/api` modules carry sample data but have NO
mounted backend endpoint: `retros.ts`, `retrain.ts`/`retrain-mocks.ts`/`retrain-types.ts`,
`duties.ts`. In a real public session these render fixture data or dead-end — exactly the
"mock dressed as real" a paying user must never hit. Building full retrain/retro/duties
backends is post-sprint scope, so the pragmatic paying-ready move is to **HIDE these
not-yet-real surfaces from the public nav** behind a feature flag, leaving them for a later
build — no dead-ends, no fixtures shown.

## What

For each surface with sample data + no live endpoint (`retros`, `retrain`, `duties` — confirm
the set at claim time with `rg "sample" web/lib/api/{retros,retrain,duties}*`):
- **HIDE it from the public/app nav** behind a feature flag (e.g. an env-gated `NEXT_PUBLIC_*`
  or a central `featureEnabled('retros')` check) that is OFF by default for the public build,
  so a real user never reaches a fixture/dead-end surface. (Building the real endpoint is a
  separate post-sprint brief — note it.)
- If a surface is trivially real-able (a list endpoint already exists), prefer wiring it; but
  default to HIDE for anything needing a new backend — this brief is about no-dead-ends, fast.
- Ensure no nav entry, command-palette action, or deep-link routes a public user to a hidden surface.

NO THEATER: the goal is that a public user only sees surfaces backed by real data; the
not-real ones are cleanly absent, not faked.

## May-touch
- `web/components/skycot/nav-*` (nav entries) + any command-palette / link source for the hidden surfaces
- a central feature-flag helper (reuse if one exists — `rg 'featureEnabled|NEXT_PUBLIC_.*ENABLE' web/lib`)
- the page routes (optional: render a "coming soon" gated state instead of a dead-end)
- vitest pinning the surfaces are hidden under the public/default flag
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

## Acceptance criteria
- [ ] `retros` / `retrain` / `duties` (and any other no-endpoint surface found) are NOT reachable by a public user under the default flag (nav + palette + deep-link all gated).
- [ ] No fixture data renders for a real session on these surfaces.
- [ ] PR body lists each surface + its disposition (hidden vs wired-to-real-endpoint) + names the post-sprint build follow-up for the hidden ones.
- [ ] `pnpm typecheck` + `pnpm lint` clean; `pnpm vitest run` GREEN (report counts).

## Out of scope
- Building the retrain/retro/duties backends (post-sprint). Workflow save/publish endpoints = WS3-2. Sample-fixture tree-shake = WS3-4.

## Parallel-surface check
- `rg 'featureEnabled|nav-items|NEXT_PUBLIC_.*ENABLE' web/` — reuse the established flag/nav pattern; no parallel gating system.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS3, "no live endpoint at all").
