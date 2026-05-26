# fix-demo-recording-disable-auth-bypass-prep

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-recording-disable-auth-bypass-prep`.
**Source**: ux-e2e-audit-T2309MEL `onboarding-hire-an-ai-teammate.md` F13 [P2] + `scene-4-and-5-claim-4-and-5.md` F6 [P1].

🟠 **P1 — `.claude/launch.json` sets `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, which makes the SPA short-circuit to local seed fixtures in `web/lib/api/{team,skills,workflows,...}.ts` BEFORE the gateway is even called. The dev environment is structurally incapable of seeing what the live gateway returns. The operator could record an 8-minute demo with the gateway returning empty `{}` envelopes and never know — the SPA renders the same 6 skills + 5 workers + 7 workflows regardless of whether the gateway is running, populated, empty, or down. This is appropriate for offline development but DANGEROUS for demo-recording preparation.**

## Root cause

Multiple SPA modules ship a permanent `isAuthBypassed()` short-circuit:

- `web/lib/api/skills.ts:152-157`
- `web/lib/api/team.ts:50-56`
- `web/lib/api/workflows.ts` (same pattern — verify line range)
- Possibly: `web/lib/api/cohorts.ts`, `web/lib/api/connectors.ts`, `web/lib/api/audit.ts`

When `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` is set (the `.claude/launch.json` default), each of these fetchers returns SEED fixtures WITHOUT making the network call.

This is correct behavior for offline development (where the gateway may not be running). It is the wrong behavior for the demo-recording profile, where:
- The runbook narrates "Skills marketplace returns 6 cards from the gateway" — but the network panel shows zero gateway calls
- F1 from the audit (`customer_research_followup` workflow only in `argo-demo-seed`, not in the SPA fallback) doesn't matter in the demo because the SPA never calls the gateway during recording
- The operator cannot pre-verify that the gateway state matches the SPA rendering before take-1

The fix is NOT to delete the bypass (offline development still needs it); it's to add a SEPARATE "recording profile" that exercises the live-gateway path.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F13 [P2] — Auth signup is bypassed in the recording configuration; brief's "(or demo-bypass)" was always-bypass
- scene-4-and-5-claim-4-and-5 F6 [P1] — Auth-bypass short-circuit hides ALL gateway problems in dev

## Fix recipe

1. **Add a recording profile env block** to `.claude/launch.json` (or as a sibling profile in a new `.claude/profiles/recording.json`):
   ```json
   {
     "name": "demo-recording",
     "env": {
       "NEXT_PUBLIC_ARGO_DEMO_MODE": "1",
       "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS": "0",
       "NEXT_PUBLIC_SKYCOT_DEMO_MODE": "1",
       "NEXT_PUBLIC_HIDE_DEMO_BANNER": "1"
     }
   }
   ```
   Keep the existing dev profile with `AUTH_BYPASS=1` untouched.

2. **Document the toggle** in `docs/skycot/demos/v1-mission-mvp-runbook.md`:
   - Pre-record step: switch SPA to the `demo-recording` profile (or `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0`)
   - Verify: open dev tools → Network → reload `/skills` → confirm a real GET to `/api/v1/skills` happens (not a short-circuited seed)
   - Run `scripts/demo-dry-run.sh` (existing or new — see below) to assert the live gateway returns canonical data before recording

3. **Add a pre-record smoke check.** `scripts/demo-dry-run.sh`:
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   # Gateway up?
   curl -fsS http://127.0.0.1:7400/health | jq '.schema_version' > /dev/null
   # Workers present?
   workers=$(curl -fsS -H 'X-Argo-Principal: human:demo@skycot.com' http://127.0.0.1:7400/api/v1/team | jq '.total_count')
   [[ "$workers" -ge 5 ]] || { echo "demo-dry-run FAIL: expected >=5 workers, got $workers"; exit 1; }
   # Skills marketplace populated?
   skills=$(curl -fsS http://127.0.0.1:7400/api/v1/skills | jq '.total_count')
   [[ "$skills" -ge 6 ]] || { echo "demo-dry-run FAIL: expected >=6 skills, got $skills"; exit 1; }
   # Workflow customer_research_followup present?
   curl -fsS http://127.0.0.1:7400/api/v1/verticals/marketplace/workflows/customer_research_followup > /dev/null || \
     { echo "demo-dry-run FAIL: customer_research_followup not seeded"; exit 1; }
   # Approvals seeded?
   apps=$(curl -fsS http://127.0.0.1:7400/api/v1/approvals | jq '.items | length')
   [[ "$apps" -ge 1 ]] || { echo "demo-dry-run FAIL: expected >=1 approval, got $apps"; exit 1; }
   echo "demo-dry-run PASS"
   ```

4. **Optional: alias the env var name.** Per `fix-claude-launch-json-env-var-naming-mismatch`, there's already drift between `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` vs `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. Verify which name `isAuthBypassed()` reads. Coordinate with that brief.

5. **Reading-time warning.** Add a CI-visible (or just log-line) warning when the SPA boots with both `DEMO_MODE=1` AND `GATEWAY_AUTH_BYPASS=1`: "⚠ DEMO-RECORDING WARNING: gateway bypass is on; SPA will NOT hit the live gateway. Switch to NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0 to exercise the real backend."

## Acceptance criteria

- [ ] A `demo-recording` env profile exists (in `.claude/launch.json` or `.claude/profiles/recording.json`) with `AUTH_BYPASS=0`
- [ ] `docs/skycot/demos/v1-mission-mvp-runbook.md` includes a pre-record step that flips to the recording profile
- [ ] `scripts/demo-dry-run.sh` exists and exits 0 against a properly-seeded gateway
- [ ] `scripts/demo-dry-run.sh` exits 1 with a clear error when ANY check fails (workers / skills / workflow / approvals)
- [ ] SPA logs a console warning (or banner) when both DEMO_MODE + AUTH_BYPASS are on simultaneously
- [ ] `pnpm --dir web exec next build` green
- [ ] Manual verification: with the recording profile, opening `/skills` triggers a real GET to `/api/v1/skills` in the Network panel

## May-touch

- `.claude/launch.json` (~10 LOC additions) OR new `.claude/profiles/recording.json` (~15 LOC)
- `scripts/demo-dry-run.sh` (~50 LOC new file)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (~15 LOC pre-record section)
- `web/lib/api/auth-bypass.ts` or wherever `isAuthBypassed()` lives (~10 LOC for boot warning)

## Out of scope

- Removing the `isAuthBypassed()` short-circuit (still needed for offline dev)
- Refactoring SPA fetchers to a single auth pattern (broader follow-up)
- Env-var-name drift cleanup (`fix-claude-launch-json-env-var-naming-mismatch`)
- Banner copy / signed-in-state copy (`fix-demo-mode-banner-investor-safe-copy`)

## Effort

S. ~50 LOC script + ~30 LOC profile + ~15 LOC doc + ~10 LOC warning. ~1 hour. P1 because the demo recording cannot be empirically pre-flighted against the live gateway in the current config — every Scene 4/5/6 gateway-dependent beat is invisible-on-rehearsal.
