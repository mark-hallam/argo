# mission-genesis-investor-demo-script-and-eval

**Owner**: claude.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-investor-demo-script-and-eval`.
**Depends on**: `mission-genesis-gateway-route-streaming-api` + `mission-genesis-spa-page-describe-and-watch` + `mission-genesis-onboarding-pick-vertical-add-describe-own` ALL MERGED.

🟡 **P1 MISSION CLAIM 3 — the investor-demo narrative + acceptance test.**

Now that Genesis is wired through the SPA, codify the live-demo script for investors + an eval test that proves the journey works end-to-end.

The investor narrative the operator stated: *"hire an AI teammate as you would a human teammate"* + the Genesis claim 3: *"the platform builds workers"*. This brief turns those two phrases into a recordable, testable moment.

## Scope

### Part 1 — Demo script (`docs/skycot/demos/genesis-investor-demo-script.md`, NEW)

90-second script for live investor demos. Five beats:

1. **(0:00-0:15)** Operator on `/landing` reads the tagline "Describe the job in plain English and Skycot builds the AI teammate."
2. **(0:15-0:25)** Click "Hire your first teammate" → arrive on `/onboarding/pick-vertical` → click "Or describe your own role".
3. **(0:25-0:50)** Operator types a real job description: *"A sales SDR for a B2B SaaS company. Qualifies inbound demo requests, books meetings on my calendar, escalates pricing questions over $50k ACV. Should know about our 3 competitors and the SOC2 compliance angle."* Operator clicks Generate.
4. **(0:50-1:15)** Live SSE progress strip ticks through 5 phases. Camera lingers on the visible "Compiling charter…" + "Generating skill bundle…" labels.
5. **(1:15-1:30)** Preview card renders Sasha (or whatever name Genesis picks) with persona + 3 sample actions + 2 duties. Operator clicks Hire. Page navigates to `/workers/{sasha-id}` — C2-personality detail with trait chips + great-at + escalate-when. **"Sasha is hired."**

Script doc includes:
- Exact operator typing (for talking-head recordings)
- Camera framing notes per beat (per memory `project_v1_demo_mp4_ship_gate` pre-record checklist)
- Failure-mode contingencies (what to say if Genesis takes >60s, if a phase errors, etc.)
- Cross-tenant proof beat (10-second add-on: "And in tenant B, this worker doesn't exist." — curl-overlay)

### Part 2 — Acceptance eval (`crates/argo-genesis/tests/investor_demo_journey_smoke.rs`, NEW)

End-to-end test:
1. `POST /api/v1/genesis` with a real role description (use the same one as the demo script).
2. Consume SSE stream + assert all 5 progress events arrive within 90s.
3. Assert `vertical_ready` event payload validates + `argo_vertical::load_dir` round-trips clean.
4. POST /api/v1/workers/hire with the manifest → assert worker row exists + has real charter / persona / kpis.
5. Cross-tenant: foreign tenant's /workers list returns 0 rows (negative control).
6. Cleanup: archive the generated vertical at test end.

Test is GATED on `ARGO_GENESIS_PROVIDER_KEY` env var (skip if unset, per `mission-demo-seed-real-llm-driver` precedent — don't burn provider credits in CI).

## Acceptance criteria

- [ ] `docs/skycot/demos/genesis-investor-demo-script.md` exists with 5 beat-by-beat sections + failure contingencies.
- [ ] `crates/argo-genesis/tests/investor_demo_journey_smoke.rs` exists + passes when `ARGO_GENESIS_PROVIDER_KEY` is set.
- [ ] Test skips cleanly when env unset (CI-safe).
- [ ] Worker who claims this brief records a real test run output in the PR body (genesis tracing log + the 5 SSE events + worker_id).

## May-touch

- `docs/skycot/demos/genesis-investor-demo-script.md` (NEW)
- `crates/argo-genesis/tests/investor_demo_journey_smoke.rs` (NEW)

## Out of scope

- Genesis runtime itself (already exists in argo-genesis crate)
- Modifying the marketing copy on /landing (separate brief if needed)

## Effort

~150-300 LOC (script doc + e2e test). ~2-3 hours.
