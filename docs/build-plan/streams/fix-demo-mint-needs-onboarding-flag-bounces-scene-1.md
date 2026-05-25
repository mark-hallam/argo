# fix-demo-mint-needs-onboarding-flag-bounces-scene-1

**Owner**: claude.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-mint-needs-onboarding-flag-bounces-scene-1`.
**Source**: e2e-ux-audit Agent (scene-1-and-2-claim-1-and-2) finding F7 (`docs/skycot/ux-audits/2026-05-25-2309/scene-1-and-2-claim-1-and-2.md`).

🔴 **P0 — `POST /api/auth/demo-mint` hardcodes `needs_onboarding: true`. Scene 1's entry navigation is bounced to `/onboarding/pick-vertical` before `/workers` is reachable. Demo-runbook entry path broken.**

## Root cause

`web/app/api/auth/demo-mint/route.ts:155-158` returns:

```json
{ "ok": true, "needs_onboarding": true }
```

…on every demo-mint call. The SPA reads `needs_onboarding` and routes to `/onboarding/pick-vertical` instead of `/workers`. Reproduction:

```
$ curl -s -X POST http://127.0.0.1:60894/api/auth/demo-mint \
    -H 'content-type: application/json' \
    -d '{"email":"demo@skycot.com"}' | jq '.needs_onboarding'
true
```

The runbook (Scene 1) opens with `/workers` already populated with the 5-persona deck. The current demo-mint code path forces a 4-step onboarding wizard before that surface is reachable. Combined with the data path (F1 in the same audit: gateway has only 1 worker `Maya`, not the 5 personas), the operator has to either:
- a) skip onboarding manually (breaks the natural funnel narrative on camera), or
- b) record a separate scene of the onboarding wizard before scene 1 (different scope).

When the upstream demo-seed brief (`mission-demo-seed-data-bootstrap` / `fix-demo-seed-cli-for-real-roster`) lands the 5 canonical workers as real gateway rows, the tenant SHOULD no longer need onboarding — but the flag is currently hardcoded `true`, not derived from "has the tenant completed onboarding".

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F7 [P0] — `POST /api/auth/demo-mint` returns `needs_onboarding: true` unconditionally, bouncing Scene 1's entry navigation to `/onboarding/pick-vertical`

## Fix recipe

1. **Derive `needs_onboarding` from real tenant state**, not a hardcoded literal. Touch site: `web/app/api/auth/demo-mint/route.ts:155-158`.

2. **Derivation rule**: a demo-mint tenant has completed onboarding when it has at least 1 worker AND at least 1 vertical installed. Query the gateway:
   - `GET /api/v1/team` → if `total_count > 0`, tenant has workers
   - `GET /api/v1/verticals` → if `verticals.length > 0`, tenant has a vertical installed
   - Set `needs_onboarding = !(hasWorkers && hasVertical)`.

3. **OR** — accept the operator's earlier decision (see audit "Suggested fix" option a/b): have `mission-demo-seed-data-bootstrap` explicitly flip the tenant's `needs_onboarding` flag to false as the last step of seeding. Either approach works; option (3) is simpler and ties the flag to the seed CLI's success, not a runtime check.

4. **For the recording lane specifically**: when the demo-seed CLI has already run for a tenant, `needs_onboarding` MUST be false so Scene 1's entry navigation lands on `/workers`.

5. **Test**: `web/app/api/auth/demo-mint/route.test.ts` — mock the gateway to return 5 workers + 1 vertical; assert response has `needs_onboarding: false`. Mock empty state; assert `needs_onboarding: true`.

## Acceptance criteria

- [ ] `curl POST /api/auth/demo-mint` after the demo-seed CLI has run for the tenant returns `{ok: true, needs_onboarding: false}`.
- [ ] `curl POST /api/auth/demo-mint` against a never-seeded tenant returns `{ok: true, needs_onboarding: true}` (preserves the wizard for first-time tenants).
- [ ] Navigating `/auth/signup` → submit → lands on `/workers` (not `/onboarding/pick-vertical`) when seeded.
- [ ] `route.test.ts` covers both branches.

## May-touch

- `web/app/api/auth/demo-mint/route.ts`
- `web/app/api/auth/demo-mint/route.test.ts` (NEW or extend)
- `crates/argo-cli/src/bin/demo-seed.rs` (if option 3: flip flag as last seed step)

## Out of scope

- The 5-worker seed itself (separate brief `fix-demo-seed-cli-for-real-roster` already covers this)
- The `/workers` empty-state vs `/team` redirect ambiguity (separate brief `fix-workers-team-route-ambiguity-and-breadcrumb-drift`)
- Magic-link signup flow (separate brief `mission-signup-magic-link-spam-protection`)

## Effort

~40-80 LOC + 1 test. ~1 hour. Demo-recording-blocker.
