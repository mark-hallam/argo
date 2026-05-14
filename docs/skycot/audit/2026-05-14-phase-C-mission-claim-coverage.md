# SKYCOT Phase C Audit — Mission-Claim Coverage Proof

**Authored**: 2026-05-14 by conductor via Explore agent (a1d20fb73fe5a0c03).
**Scope**: For each of the 6 mission claims, demonstrate that an E2E test or smoke flow exercises the claim.
**HEAD at run time**: `66758890`.

## Per-claim coverage table

| Claim | Coverage | E2E tests | Smoke step? | Gap |
|---|---|---|---|---|
| 1. Teammate | ADEQUATE | `argo-identity/tests/create_show_list.rs` | none explicit | Per-worker memory scope queued |
| 2. Delegation | STRONG | `argo-orchestrator/tests/duty_scheduler_integration.rs` | steps 12-15 | M6+M7+M8 not shipped |
| 3. Genesis builds workers | ADEQUATE | `argo-genesis/tests/marketing_intern.rs` | step 14 (lint 3 verticals) | No eval-seed gen test |
| 4. Skills shared/learned | **WEAK** | `argo-store/tests/skill_share.rs` only; marketplace→worker propagation unimplemented | none | 🔴 SHIP-BLOCKER (Phase A.5 finding) |
| 5. PC execution | STRONG | `argo-cli/tests/browser_action_executor_smoke.rs`; `argo-executor-computer-use-macos/tests/notes_smoke.rs` | step 19 (Browser mock) | No real macOS computer-use full-workflow E2E |
| 6. Multi-channel approval | STRONG | 8 channel tests + Slack roundtrip + E1 SPA Playwright | step 20 (Slack HMAC) | M8 composition UI TODO |

## Success-scenario demo-readiness

| # | Scenario | Demoable today? | Missing |
|---|---|---|---|
| 1 | Generated worker runs autonomously over non-CLI | ⚠️ PARTIAL | Full duty→Slack→reply→CU loop untested as smoke |
| 2 | Learned skill propagates across workers | 🔴 **NO** | Brief `claude-skill-library-worker-load-wire` required |
| 3 | Multi-worker handoff with shared memory | ✅ YES | But memory-use by handoff worker B not proven |
| 4 | Auto-approval under budget | ✅ YES | Existing tests sufficient |
| 5 | Audit trail complete | ✅ PARTIAL | L11 IN-FLIGHT for CU→IR evidence |

## Smoke test coverage (20 steps)

- Steps 1-15: covers claims 1, 2, 3 well; mock-mode end-to-end
- Step 19: Browser mock only (not real CU)
- Step 20: Slack approval roundtrip (claim 6)
- **Not exercised**: claim 4 skill loop, real macOS CU execution, multi-channel orchestration

## Top 5 missing tests for v1.0

1. **`claude-skill-library-worker-load-wire`** (already authored) — 🔴 SHIP-BLOCKER, 150 LOC
2. **`claude-skill-marketplace-end-to-end-test`** (already authored) — 250 LOC
3. **`claude-computer-use-full-workflow-smoke`** (NEW) — 300 LOC. Real macOS CU end-to-end: worker queues action with `executor: { kind = "computer-use" }`; macOS executor spawns, executes N screenshot+click cycles, reports completion. macOS-gated. Claim 5 + scenario #1.
4. **`claude-multiworker-duty-handoff-e2e-smoke`** (NEW) — 200 LOC. Worker A fires duty → completes → hands off to Worker B with memory context; B fires its own duty → asserts shared memory used. Scenario #3.
5. **`claude-vertical-eval-seed-generation-test`** (NEW) — 120 LOC. Genesis emits eval seeds + fixtures for a generated vertical; verify ≥5 fixtures generated. Claim 3.

## Optional P1.5 briefs

- **`claude-auto-approval-multiwindow-e2e`** (NEW) — 180 LOC. Budget gate + channel fallback. Depends on M8. Scenario #4 polish.
- **`claude-cross-tenant-skill-discovery-test`** (NEW) — 100 LOC. Cross-tenant skill discovery (after Brief A). May overlap with `mission-cross-tenant-trajectory-aggregation` — implementer should consolidate.

## Verdict

Smoke test is NECESSARY but NOT SUFFICIENT for v1.0 ship gate without:
1. Brief A (skill-library-worker-load-wire) MERGED — fixes scenario #2
2. 3 new test briefs MERGED (computer-use-full-workflow, multiworker-handoff, vertical-eval-seed-gen)
3. Playwright CI wiring fixed (currently disabled per V1_DEFINITION.md §4)
4. Demo MP4 recorded (operator artifact)

**5/5 success scenarios become demoable** with Brief A merged + scope cuts (macOS-first) honored.
