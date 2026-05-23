# round 45 — process drift + P0 ageing watch + overlay 5th-round recheck

- **Date**: 2026-05-23T22:27Z (Melbourne local Sun 2026-05-24 08:27 AEST)
- **Round**: 45
- **Pillar / Claim**: Process gates + Claim 5 (Skills are shared and learned) + Claim 6 (Multi-tenant-by-construction)
- **Verdict**:
  - 🟡 **P0 `mission-skill-share-tenant-pinned` STILL UNCLAIMED at 2h+** — third consecutive evaluator round flagging this; tick-131 ran ledger-maint but did not claim. Recommend conductor claim next tick.
  - 🔴 **`store_worker_overlay_tests` 2/5 RED on origin/main — 5TH consecutive evaluator round** — both fix-briefs unclaimed across all 5 rounds.
  - 🟢 Process gates clean: findings-only 4/4 within budget, gateway DOWN (informational).

## Scenario 1 — P0 ageing watch (`mission-skill-share-tenant-pinned`)

### Setup

```bash
grep -n "mission-skill-share-tenant-pinned" docs/build-plan/BUILD_STATUS.md
ls docs/build-plan/streams/mission-skill-share-tenant-pinned.md
git log --oneline origin/main -5  # confirm tick-131 ran without claim
```

### Observed

- **Row state**: BUILD_STATUS.md L1207 — `claude | TODO | — | 🔴 P0 LATENT-BUT-LIVE CROSS-TENANT MUTATION`. No claim, no PR.
- **Brief on disk**: `docs/build-plan/streams/mission-skill-share-tenant-pinned.md` exists, valid header, freshness-validated.
- **Ageing timeline**:
  - 2026-05-23T20:27Z — round 43 elevated POSSIBLE-LATENT → LIVE-EXPLOIT via byte-for-byte production verify; recommended P0 promote to conductor.
  - 2026-05-23T20:42Z — tick-127 conductor inserted P0 ledger row.
  - 2026-05-23T21:27Z — round 44 flagged UNCLAIMED 60min, recommended conductor claim next tick.
  - 2026-05-23T22:13Z — tick-130 actioned PR #2131 fix-request + queue ops; did NOT claim P0.
  - 2026-05-23T22:30Z — tick-131 actioned 2 MERGES (#2169 + #2170) + DEMOTE (#2131 4th cycle) + 4 codex briefs authored for queue replenishment; did NOT claim P0.
  - 2026-05-23T22:27Z — round 45 (this report) — STILL UNCLAIMED 2h+ after elevation.
- **Production code on main unchanged**: `crates/argo-store/src/skill_share.rs:78-86` `set_shared` UPDATE WHERE is still `name = ? AND version = ?` (no tenant_id bind). The LIVE exploit path at `crates/argo-cli/src/handlers/gateway/marketplace.rs:252` `POST /marketplace/<id>/install` is still wired.

### Anti-theater check

Read the production code again to verify the leak is still real:

```
crates/argo-store/src/skill_share.rs:78-86  -> WHERE name = ? AND version = ? (NO tenant_id)
crates/argo-cli/src/handlers/gateway/marketplace.rs:252  -> set_shared(...) callsite with NO tenant filter
crates/argo-store/migrations/v80.rs  -> adds tenant_id column to skill table (post-#2092)
```

The leak is exploitable today, not just latent. **Pre-v1.0-demo blocker** if marketplace install is in the recorded surface. Round-43 P0 elevation verdict still holds.

### Recommendation for tick-132

**Claim `mission-skill-share-tenant-pinned` next tick.** The recipe is canonical and matches the PR #2168 / #2167 family that has been successfully shipped 3 times this week (additive-constructor `set_shared_for_tenant(store, tenant_id, name, version, shared, cluster)` + thread `auth.tenant_id` from marketplace handler + cross-tenant pin test + same-tenant baseline). ~200-350 LOC. Worker time, ~1h cold.

## Scenario 2 — `store_worker_overlay_tests` 5th-round RED recheck

### Setup

```bash
cargo test -p argo-skill --lib store_worker_overlay_tests::
```

### Observed

```
test store_worker_overlay_tests::list_for_worker_filters_other_workers ... FAILED
test store_worker_overlay_tests::list_for_worker_returns_attached_active_overlays ... FAILED
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 351 filtered out; finished in 0.63s
```

Failure shape identical to rounds 41/42/43/44:
```
assertion `left == right` failed
  left: 0
 right: 1
```

at `crates/argo-skill/src/store_worker_overlay_tests.rs:45:5` — the BLOB-vs-TEXT join mismatch surfaced by `fix-skill-worker-overlay-blob-text-join-mismatch` (P1, TODO, unclaimed since tick-121).

### Ageing timeline

- Round 41 (~5h ago) — first surfaced, brief authored, P1
- Round 42 — RED, brief still TODO
- Round 43 — RED, brief still TODO
- Round 44 — recheck deferred (cold build still in flight at submit)
- Round 45 (this) — RED, brief STILL TODO 5h+ after authoring

### Anti-theater check

Cold cargo invocation reads the actual test binary on disk (no test rewriting), produces deterministic 2/5 failure across 5 consecutive evaluator rounds, error message identical. The failure is real and reproducible.

### Recommendation for tick-132

**Bundle `fix-skill-worker-overlay-blob-text-join-mismatch` + `ops-pre-merge-gate-fix-test-failures-not-just-clippy` into a single conductor-pre-merge-gate-hardening-wave PR** — same recommendation as round 43, escalated. Both are fast-mode candidates (~5-50 LOC bash each). The longer this 2/5 RED stays unfixed, the higher the risk a new PR ships that masks an unrelated regression by virtue of these tests being already RED.

## Scenario 3 — Process drift sweep

```bash
bash scripts/check-findings-only-fix-by.sh    # 4/4 within budget (next deadline 2026-05-30)
bash scripts/check-gateway-freshness.sh       # UNREACHABLE (informational, swarm-compile load)
curl -sf http://127.0.0.1:7400/health         # exit 7 (DOWN)
```

- **findings-only fix-by tracker**: 4 within-budget · 0 due-today · 0 overdue. All four are 7 days remaining (2026-05-30 deadline). Same as round-43 after tick-127 flipped `mission-cross-tenant-test-audit-export` to MERGED-FINDINGS-CLOSED.
- **gateway-freshness**: UNREACHABLE — gateway DOWN at :7400. This is the informational flag; per round 30 the gateway being down during swarm-compile load is the expected steady state.
- **SHARED.md timestamp-drift**: 11th-consecutive-round of operator-pending action on `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` (still not repo-tracked; race observed in this very session — round-45 heartbeat got overwritten back to round-44 stamp by worker `git reset --hard` mid-run; re-wrote at end of run).

### Recommendation for tick-132

- **Re-surface `ops-evaluator-skill-md-update-from-runbook`** (P2, operator-actionable) — 11th round in a row. The runbook from #2155 superseded base evaluator.json semantics but SKILL.md still references the base; operator must hand-edit since SKILL.md isn't repo-tracked.

## Summary of round-45 brief candidates for conductor tick-132

| Brief | Severity | Status | Recommendation |
|---|---|---|---|
| `mission-skill-share-tenant-pinned` (existing row, L1207) | P0 | UNCLAIMED 2h+ | **CLAIM** — recipe is canonical; lift #2168 pattern |
| `fix-skill-worker-overlay-blob-text-join-mismatch` (existing row, P1) | P1 | UNCLAIMED 5h+ | Bundle into pre-merge-gate-hardening wave |
| `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (existing row, P1) | P1 | UNCLAIMED | Bundle into pre-merge-gate-hardening wave |
| `ops-evaluator-skill-md-update-from-runbook` (existing row, P2) | P2 | Operator-actionable | Re-surface for operator summary |
| **NEW** `ops-conductor-fix-request-comment-needs-stale-doc-tree-cleanup-recipe` | P3 | TODO-author | 5-line recipe in fix-request comment template |

## Evaluator stamp

23rd anti-theater-honest substrate contract on main this week (counting pending #2131 merge as #23).
