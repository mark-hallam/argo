# fix-onboarding-two-roster-uuid-space-fixtures-vs-gateway

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-onboarding-two-roster-uuid-space-fixtures-vs-gateway`
**Source:** UX e2e audit 2026-05-28-0252 (onboarding F01 + F14)

🟠 **P1 — Two parallel worker rosters exist with non-overlapping UUIDs: gateway returns 5 workers with IDs `019e6785-928*-*-*-*` (Mariah/Peta/Priya/Jamal/Nori); SPA hardcodes `018f67d0-0000-7000-8000-00000000000{1..4,7}` for the same five names. With `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` the bypass path serves fixtures and the gateway-real workers are never shown; without bypass the gateway is shown and the SPA fixtures aren't. Anyone clicking a fixture-UUID link in the wrong mode 404s, and demo-mode first-hire cannot route to Peta because her sample UUID isn't defined.**

## Root cause

Three coupled files maintain independent worker-ID universes:

- `web/lib/api/team.ts:85-135` — when `isAuthBypassed()`, `fetchTeam()` returns `SAMPLE_TEAM` (fixture roster) instead of hitting the gateway.
- `web/lib/sample-workers-detail.ts:32-93` — defines `SAMPLE_MARIAH/PRIYA/JAMAL/NORI` with hardcoded UUIDs `018f67d0-0000-7000-8000-000000000001/2/3/4` and `…000000007` (Peta is missing from the constant exports — the file only carries 4 sample worker constants).
- `web/lib/onboarding/use-first-hire.ts:109-128` — `VERTICAL_TO_SAMPLE_WORKER` maps verticals to one of those 4 SAMPLE UUIDs; `project-coordination` is not in the map, so any operator picking that vertical falls through to `DEFAULT_FALLBACK = SAMPLE_MARIAH` — Peta is unreachable in demo mode.

Gateway-side, `curl -H 'Authorization: Bearer <demo-mint-jwt>' http://127.0.0.1:7400/api/v1/workers` returns the same 5 names under different UUIDs (`019e6785-928*-*-*-*`). Cross-surface, anyone clicking a fixture link expecting a gateway worker (or vice versa) gets 404 on the detail page — different namespaces don't intersect.

The onboarding audit flagged that the demo MP4 will run against ONE configuration so this is shippable for v1.0 (per `feedback_no_theater_in_demo` fixtures are acceptable for the dressing side), but:
- developer testing in bypass mode shows a different roster than a real signed-in user
- the `Peta` sample UUID is missing entirely (F14)
- demo-mint creates a real principal that COULD hit the gateway, but `fetchTeam()` still short-circuits to fixtures

## Findings this brief closes

- onboarding F01 [P1] — Two parallel worker rosters with non-overlapping UUIDs (`018f67d0-…` fixtures vs `019e6785-…` gateway)
- onboarding F14 [P3] — `VERTICAL_TO_SAMPLE_WORKER` map cannot route to Peta from any vertical (no `SAMPLE_PETA` constant)

## Fix recipe

Choose **Path A (smaller)** as the v1.0 fix; Path B is the post-MVP cleanup.

**Path A — Align the SPA fixtures on the gateway-seeded UUIDs.**

1. Run the demo seed CLI (`argo demo seed`) and read the actual gateway UUIDs for the 5 canon workers via `curl http://127.0.0.1:7400/api/v1/workers`. These IDs are deterministic per seed run (verify by reading the seed code in `crates/argo-cli/src/handlers/demo_seed.rs` or wherever the seed lives).
2. Update `web/lib/sample-workers-detail.ts:32-93` to use the gateway-seeded UUIDs for all 5 canon constants:
   ```ts
   export const SAMPLE_MARIAH = "019e6785-928f-7281-bf8d-…";   // Mariah from demo seed
   export const SAMPLE_PETA   = "019e6785-928*-…";              // NEW — was missing
   export const SAMPLE_PRIYA  = "019e6785-928*-…";
   export const SAMPLE_JAMAL  = "019e6785-928*-…";
   export const SAMPLE_NORI   = "019e6785-928*-…";
   ```
3. Update `web/lib/onboarding/use-first-hire.ts:121-128` to add `"project-coordination": { id: SAMPLE_PETA, name: "Peta" }` to the vertical map.
4. Cascade-update any test fixtures that hardcode the old `018f67d0-…` UUIDs (grep for the literal string across `web/`).

**Path B — Drop the bypass short-circuit and let the demo-mint principal hit the gateway.**

When the user came through `/api/auth/demo-mint`, they have a verified `human:<email>` JWT + a real gateway tenant. `fetchTeam()` could call the gateway with that JWT instead of returning fixtures. The downside: an offline preview (no gateway running) wouldn't work. Defer this until the operator decides whether offline previews matter.

**Tests:**

5. Extend `web/lib/onboarding/use-first-hire.test.ts` (NEW or existing) to assert that every vertical in the wizard's catalogue maps to a defined SAMPLE_* UUID and that `project-coordination` maps to Peta.
6. Add a grep test to CI: `! rg "018f67d0-0000-7000-8000" web/lib/` (fails if the old fixture UUIDs reappear).

## Acceptance criteria

- [ ] All 5 `SAMPLE_*` constants in `web/lib/sample-workers-detail.ts` use the gateway-seeded UUIDs.
- [ ] `SAMPLE_PETA` exists and is exported.
- [ ] `VERTICAL_TO_SAMPLE_WORKER` includes `project-coordination → SAMPLE_PETA`.
- [ ] Clicking any fixture-UUID link on the SPA also resolves on the gateway (cross-mode coherence).
- [ ] `grep -rn "018f67d0-0000-7000-8000" web/lib/` returns no production-code matches (test fixtures may keep them if needed).
- [ ] vitest covers the vertical-to-Peta mapping.
- [ ] PR body cites onboarding F01 + F14.

## May-touch

- `web/lib/sample-workers-detail.ts` (constants update, ~10 LOC)
- `web/lib/onboarding/use-first-hire.ts` (map update, ~3 LOC)
- `web/lib/api/team.ts` (only if Path B taken)
- `web/lib/sample-team.ts` (cascade)
- Any test file that hardcodes the old UUID literal

## Out of scope

- Demo seed CLI itself (covered by `fix-demo-seed-cli-for-real-roster` MERGED-STALE-FIXED + `fix-demo-seed-cli-operator-action-runbook` MERGED)
- Path B (bypass-removal) — operator decision, defer
- Personality card fallback for non-canon names (covered by `fix-worker-personality-and-level-derive-from-real-data` BLOCKED + sibling worker-personality briefs)
- First-hire-demo prompt discard (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker` IN_REVIEW #2463)

## Effort

S — ~15 LOC production code + ~50 LOC test + a one-time seed-UUID lookup. ~2-3 hours. Depends on the demo seed CLI being run cleanly first to harvest the UUIDs.
