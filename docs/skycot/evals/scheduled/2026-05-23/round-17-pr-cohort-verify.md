# Round 17 — PR cohort verification (round-13 SQL leak fixes + CHANGES-REQUESTED triage)

**Date**: 2026-05-23T15:00Z
**Round number**: 17
**Pillar/Claim**: claim 6 (multi-tenant by construction) + process drift
**Scope**: anti-theater verify the round-13 SQL leak PRs (#2094, #2095, #2093); status-check the 3 P0 CHANGES-REQUESTED PRs (#2087, #2088, #2090).

---

## Scenario 1 — #2094 `claude-skill-runs-worker-budget-tenant-scope` (P1)

**Verdict**: 🟢 PASS — real structural fix, not theater.

**Setup**:
- Read `gh pr diff 2094` end-to-end (145 add / 19 del across 4 files).
- Confirmed the leak still exists on `origin/main`:
  ```
  crates/argo-store/src/skill_runs_query.rs:
      "SELECT action_id, COALESCE(SUM(usd_micros), 0) AS spend_usd_micros \
       FROM worker_budget GROUP BY action_id"   ← no tenant predicate
  ```

**Observed**:
- `budget_by_action` signature now takes `tenant_id: Option<&str>`.
- SQL adds `WHERE tenant_id = ?` when scoped; admin/operator surfaces pass `None` for cross-tenant view.
- `tenant_id.as_bytes()` is bound — correct, since `worker_budget.tenant_id` is BLOB (the docstring explicitly notes TEXT/BLOB comparisons return false even on byte equality in SQLite — that's the trap the round-3 audit-export bug fell into).
- 5 caller sites updated (`runs_for_skill` external API + 3 internal + 1 in argo-forge, 1 in argo-skill, 1 in argo-cli).

**Anti-theater check**:
- Regression test `budget_aggregate_is_tenant_scoped` deliberately reuses the SAME `action_id` across tenants. Before the fix, `GROUP BY action_id` would have aggregated tenant_a's spend into tenant_b's row. After the fix, tenant_b only sees their own spend. The assertion is meaningful.
- Test does NOT just assert `result.is_empty()` (which would pass even on a broken fix that returns nothing) — it asserts the spend value matches the per-tenant input, distinguishing fix from over-correction.
- Docstring cites the round-13 evaluator finding by name — claim of intent is sound.

---

## Scenario 2 — #2095 `claude-spend-anomaly-tenant-scope-distinct` (P1)

**Verdict**: 🟢 PASS — real structural fix, not theater.

**Setup**:
- Read `gh pr diff 2095` (134 add / 17 del across 3 files).
- Confirmed leak still on `origin/main`:
  ```
  crates/argo-store/src/spend_anomaly.rs:
      "SELECT DISTINCT worker_id FROM worker_budget ORDER BY worker_id"   ← no tenant predicate
  ```

**Observed**:
- `list_active_worker_ids` signature now **requires** `tenant_id: &TenantId` (not `Option`). Stronger than #2094 because there is no legitimate "cross-tenant worker enumeration" use case.
- New helper `list_active_tenant_ids` for the admin cron driver to walk every tenant first.
- CLI handler `budget anomaly-tick` rewritten to iterate tenants → per-tenant workers, preserving the original "walk every active worker" semantic without leaking worker_ids across tenants at the query layer.
- Docstring calls the predicate "load-bearing" and explicitly cites this as the **SQL-layer twin of #2068** (the response-layer existence leak I surfaced as round-4 P2 → now closed). Good cross-reference.

**Anti-theater check**:
- Regression test `list_active_worker_ids_does_not_leak_across_tenants` seeds 4 workers (2 per tenant) with non-overlapping IDs (W1+W2 vs W3+W4), calls from tenant_b's context, asserts the result is **exactly** `[W3, W4]`. The exact-equality (not `contains_all`) catches both leak (would include W1/W2) and over-correction (would be empty).
- Existing `list_active_worker_ids_returns_distinct_rows` test updated to take the tenant parameter; not silently weakened.
- Type signature change (`Option` → required `&TenantId`) forces every future caller to explicitly think about tenant scope — defense in depth.

---

## Scenario 3 — #2093 `eval-subtask-ledger-caller-tenant-audit` (P1 investigation)

**Verdict**: 🟢 PASS — clean investigation-style brief, correct shape.

**Setup**:
- Read PR body + file list.
- Verified the brief is **investigation-only**: 0 production source changes under `crates/argo-store/` (just tests + report).

**Observed**:
- Per-caller walk of all 12 `subtask_ledger` SQL helpers. Verdict: **0 🔴 actual leaks**.
- 4 🟡 contract-pinning tests cover the CLI surface (no gateway exposure today, but the tests fail-closed if a future route exposes id lookup without tenant-scoping).
- 3 🟢 runtime callers verified: `turn_budget::check_pre_turn_budget`, `turn_budget::record_subtask_turn_spend`, `record_decomposition` — all resolve their id via session-scoped lookup or natural key.
- Differentiation table vs #1992 (audit_detail) makes the boundary explicit: subtask_ledger has no HTTP route + no tenant_id column by design; SAFETY block in `29__subtask_ledger.sql` documents this.

**Anti-theater check**:
- The "no fix needed" verdict is the correct outcome when the round-13 finding turns out to be a non-issue. Brief authoring discipline says **don't manufacture a fix to look productive**. This PR opted for contract pins instead of a non-fix, which is the right shape.
- Tests are non-trivial (167 LOC runtime + 233 LOC CLI); not single-line `assert!(true)` filler.

---

## Scenario 4 — 🔴 PROCESS FINDING: 3 P0 CHANGES-REQUESTED PRs stale

**Verdict**: 🔴 FAIL (P1 process)

**Setup**:
- `gh pr view 2087 / 2088 / 2090 --json updatedAt,mergeable,state`

**Observed**:
- All three CHANGES-REQUESTED P0 PRs from tick-85 last updated at **2026-05-23T02:10Z** — **~13 hours** stale.
- #2087 `mission-cred-get-tenant-parameter` — MERGEABLE but no new commit since CHANGES-REQUESTED. The detailed remediation comment ("91/91 callsites took get_unscoped — P0 leak still live") has not been actioned.
- #2088 `fix-duty-scheduler-invokes-agent-runner` — **CONFLICTING** with main.
- #2090 `mission-credentials-per-tenant-master-key` — **CONFLICTING** with main.

**Anti-theater check**:
- The conductor tick-85 entry says "fix-briefs authored" but they're not driving worker action. Either workers don't see CHANGES-REQUESTED as claimable, or the rebase pressure once #2086/#2089/#2091 merged pushed both #2088 and #2090 into conflict and nobody picked them back up.
- These are the **last three load-bearing P0 cross-tenant fixes** from the round-9/10 finding wave. Demo MP4 cannot ship cleanly while cred::get is still `get_unscoped`-ing across 91 callsites.

**Gap / follow-up brief candidates**:

| Brief shape | Priority | Why |
|---|---|---|
| `claude-rebase-changes-requested-cluster-tick85` | 🔴 **P0** | Same recipe that worked for the tick-38 5-DIRTY cluster (#2041). Worker rebases all three, addresses remediation comments inline, re-pushes. Without it the 3 P0 cross-tenant fixes will rot indefinitely. |

## Operational

- **Gateway**: DOWN. Verbose `curl -v http://127.0.0.1:7400/health` returns "Connection refused". (Initial `curl -sf` exit=0 was misleading because `-s` suppressed the connection-refused message — the empty body + exit 0 looked like an empty 200 but was actually no connection at all. Lesson: always probe with `-v` not `-sf` for binary up/down checks.)
- Browser scenarios skipped this round.
- Heartbeat written, round-17 report file complete, scenarios 1-3 PASS + scenario 4 surfaces a P0 process gap with a clear fix recipe.

## Gateway-up backlog (when it comes back)

- Round-5 P0 was the layout.ts rename; resolved per ledger. Worth re-probing `/workflows/sales/qualify_lead/build` to confirm the canvas renders.
- Round-7 P1 `web-workflow-build-fetch-from-gateway` (canvas reads SEED_WORKFLOWS not gateway) — verify the next-tick fix closes it.
- Round-16 P2 `mission-orchestrator-duty-store-tenant-args` — still open recommendation.
