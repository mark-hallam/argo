# mission-genesis-workers-empty-state-describe-flow

**Owner**: claude.
**Authored against**: origin/main `5b20282f7` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-workers-empty-state-describe-flow`.

## Why (verified gap, 2026-06-09)
DEC-034 WS3: Genesis IS the hire path. When the workspace has no teammates, the `/team` + `/workers` empty state should invite the first hire via the **Genesis describe-and-watch flow** (`/genesis`). It currently invites a describe-flow, but routes to the WRONG surface.

VERIFIED on current `origin/main` (the empty state IS built — this is a re-route, not a new surface):
- `web/app/(app)/workers/page.tsx` renders `<HireStarterPrompts />` for the empty roster (lines 102, 250) and `web/app/(app)/team/page.tsx` uses the same.
- `web/components/empty-state/HireStarterPrompts.tsx` is a thin wrapper around `<TeamEmptyState>`.
- `web/components/skycot/TeamEmptyState.tsx` IS the empty state. Its primary CTA — "Describe a role in plain English →" (line 62) — `<Link href="/onboarding/first-hire">` (line 54). **It does NOT route to `/genesis`.** The doc-comment even says "Or describe your own link that routes to `/hire`" — stale; the live href is `/onboarding/first-hire`.
- So the empty-state's "describe your own" CTA bypasses the Genesis describe-and-watch surface entirely. The watch-it-get-built moment is one re-route away.

## Grep-before-write
- `rg -n 'href|onboarding/first-hire|/genesis|/hire' web/components/skycot/TeamEmptyState.tsx` — confirm the primary CTA still points at `/onboarding/first-hire` (re-verify; do not double-edit if a prior worker already re-pointed it).
- `rg -l 'TeamEmptyState' web/` — find ALL consumers (`HireStarterPrompts` → `/workers` + `/team`); a single edit to `TeamEmptyState.tsx` fixes every consumer (do NOT fork a parallel component).
- `rg -n 'data-testid="team-empty-state-describe-own"' web/` — the existing testid the tests assert on; keep it.
- `rg -n 'useSearchParams|return=' 'web/app/(app)/genesis/page.tsx'` — check whether `/genesis` reads a return param before sending `?return=/team`.
- `rg 'pub (struct|enum|fn)' crates/` — N/A: web-only (TSX). No new Rust public surface.

## Fix (additive / minimal; consumer = both `/workers` and `/team` empty states via the shared component)
1. **Re-point the primary CTA** in `web/components/skycot/TeamEmptyState.tsx` from `href="/onboarding/first-hire"` to `href="/genesis"` so the headline "describe a role" CTA lands the stranger on the Genesis describe-and-watch surface. Carry `?return=/team` ONLY if `/genesis` consumes it (see grep); otherwise plain `/genesis`.
2. **Refresh the stale doc-comment** (lines ~8-19) to say it routes to `/genesis` (the Genesis describe flow), not `/hire`.
3. **Keep the template tiles** (`EXAMPLE_ROLES` grid) as the fast-lane fallback — unchanged. Only the headline "describe your own" CTA moves to Genesis.
4. **Optional, same component:** the secondary disclosure copy is unchanged. Do NOT add a new component or new testid; reuse `data-testid="team-empty-state-describe-own"`.

## Acceptance (truth-bounded)
1. `web/components/skycot/TeamEmptyState.tsx`: the element with `data-testid="team-empty-state-describe-own"` now has `href` starting with `/genesis`. The template tiles (`team-empty-state-tiles`) still render.
2. Updated **vitest**: the existing TeamEmptyState test that asserts the describe-CTA href is updated to expect `/genesis` (re-run green). If no such test exists, add one rendering `<TeamEmptyState />` and asserting the CTA href.
3. Both `/workers` and `/team` empty states inherit the new route (verified by the shared-component test; no per-page edit needed).
4. Web test runner green for the touched files; web lint clean. File stays < 500 LOC (currently ~117).

## On ship (same commit)
Update the FEATURE_MATRIX.md row for the workers/team empty state (or the Genesis row) to note the first-hire empty-state CTA now routes into the Genesis describe-and-watch flow. 🚨 DEC-043: do NOT regenerate or commit `feature-matrix.html` (gitignored) — `.md` row only.

## Effort
XS — a one-line href change + doc-comment + a test assertion update. ~30-60 min.
