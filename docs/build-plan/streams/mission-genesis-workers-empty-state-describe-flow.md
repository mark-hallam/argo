# mission-genesis-workers-empty-state-describe-flow

**Owner**: claude or codex.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-workers-empty-state-describe-flow`.
**Depends on**: `mission-genesis-gateway-route-streaming-api` MERGED + `mission-genesis-spa-page-describe-and-watch` MERGED.

🟡 **P1 MISSION CLAIM 3 — second-time-user discovery surface for Genesis.**

Once the workspace is set up, a returning user who wants to hire another teammate goes to `/workers`. The empty-state today shows the 5 starter templates. We need the same Genesis CTA visible here.

## Scope

Modify the `/workers` empty-state component (per Agent-2 audit, the `WORKER_TAB_IDS` empty state at `web/app/(app)/workers/page.tsx`):

1. Above the 5 starter templates, surface the same Genesis CTA card from the onboarding-pick-vertical: "Or describe your own role".
2. Clicking routes to `/genesis?return=/workers`.
3. After Hire, route to `/workers/{new_id}` (the just-hired worker's detail page).

This is the steady-state hire-another-worker path. Reuse the `<GenesisCta>` component from the onboarding brief.

## Acceptance criteria

- [ ] `/workers` empty state shows the Genesis CTA + 5 prefabs.
- [ ] `/workers` populated state (with seeded workers) shows the Genesis CTA in a less-prominent way (e.g. small "+ Hire another teammate" button at the top of the card grid that opens the same flow).
- [ ] After Hire, the new worker appears in the deck without a page reload (or with a single navigate-and-refresh).

## May-touch

- `web/app/(app)/workers/page.tsx`
- `web/components/skycot/workers/EmptyState.tsx` (or similar; check existing)
- `web/components/skycot/onboarding/pick-vertical/GenesisCta.tsx` (re-use; may need parameterization)

## Out of scope

- Genesis page / gateway / onboarding-side integration (separate briefs)

## Effort

~50-120 LOC. ~1-2 hours.
