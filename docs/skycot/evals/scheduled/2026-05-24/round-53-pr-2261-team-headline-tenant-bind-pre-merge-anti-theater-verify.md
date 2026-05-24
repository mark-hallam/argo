# Round 53 — PR #2261 mission-team-headline-tenant-bind — PRE-MERGE anti-theater verify

**Date**: 2026-05-24 (UTC) / 2026-05-25 09:10 AEST
**Round**: 53
**Pillar / Claim**: Claim 6 (Multi-tenant by construction)
**Verdict**: 🟢 **PRE-MERGE PASS** (P1 — claim-6 substrate)

## Setup

- `gh pr view 2261 --json title,headRefName,mergeable,mergeStateStatus,baseRefName,body,files`
- `gh pr diff 2261` — full diff read end-to-end (6 files, +143 / −16, including a NEW 112-LOC `tests/cross_tenant_negative_test.rs`)
- `grep -rn "generate_team_headline\|activity_counts_for_worker" crates --include="*.rs"` — orphan-caller search
- `grep -n "tenant_id" crates/argo-store/src/migrations/v1.rs` — schema-bind type check

Branch: `codex-mission-team-headline-tenant-bind` → `main`. Status: CLEAN MERGEABLE per gh-view.

## Observed

### Diff shape (6 files)

| File | LOC | Role |
|---|---|---|
| `crates/argo-team-headline/src/compute.rs` | +6/−2 | adds mandatory `tenant_id: TenantId` param to `activity_counts_for_worker`; SQL grows `WHERE s.tenant_id = ? AND s.worker_id = ?` with `.bind(tenant_blob)` *before* `.bind(worker_blob)` |
| `crates/argo-team-headline/src/lib.rs` | +6/−3 | adds mandatory `tenant_id: TenantId` param to `generate_team_headline`; threads into compute call |
| `crates/argo-team-headline/src/lib_tests.rs` | +9/−7 | adds `tenant: TenantId` to `SeededWorker` struct + updates all 6 callsites |
| `crates/argo-team-headline/tests/cross_tenant_negative_test.rs` | +112/−0 | NEW. Round-37 vacuous-pass guard with `leaked == 0` AND `owned >= 1` |
| `crates/argo-cli/src/handlers/gateway/routes_team_list.rs` | +1/−1 | calls `generate_team_headline(workspace_tenant, w.id, &w.vertical_id, store)` |
| `crates/argo-cli/src/handlers/gateway/routes_worker_detail.rs` | +9/−3 | adds `tenant: TenantId` param to `build_headline` helper, threads through |

### Production SQL — post-merge contents of `compute.rs`

```sql
SELECT
  SUM(CASE WHEN a.kind = 'assistant_message' THEN 1 ELSE 0 END) AS assistants,
  SUM(CASE WHEN a.kind = 'tool_call'         THEN 1 ELSE 0 END) AS tools,
  SUM(CASE WHEN a.kind = 'escalation.required'                       THEN 1 ELSE 0 END) AS escalations,
  SUM(CASE WHEN a.kind IN ('approval.granted', 'approval.consumed')  THEN 1 ELSE 0 END) AS approvals
FROM audit_record a
JOIN session s ON s.id = a.session_id
WHERE s.tenant_id = ?
  AND s.worker_id = ?
  AND a.created_at >= ?
```

Bind order in the diff: `.bind(tenant_blob).bind(worker_blob).bind(since_micros)` — predicate order matches bind order.

### Schema-bind match

- `session.tenant_id` is `BLOB NOT NULL` per `crates/argo-store/src/migrations/v1.rs:38`. Index exists at `idx_session_tenant_started ON session(tenant_id, started_at DESC)` (line 53).
- Diff binds `tenant_blob = tenant_id.0.as_bytes().to_vec()` — `Vec<u8>` is sqlx's canonical BLOB bind. ✅ matches schema column type. Mirrors `#2260` r52 verification's pattern for `session.tenant_id` BLOB.

### Cross-tenant negative test — anti-theater dissection

The new `tests/cross_tenant_negative_test.rs` seeds:

- `tenant_a` with `worker_a`, session attached to `vertical_id = "pr-checker"`, **2** `tool_call` audit rows
- `tenant_b` with `worker_b`, session attached to `vertical_id = "pr-checker"`, **5** `tool_call` audit rows

Then queries `generate_team_headline(tenant_a, worker_a, "pr-checker", &store)` (the **owned** call) and `generate_team_headline(tenant_a, worker_b, "pr-checker", &store)` (the **cross-tenant** call — same tenant_a credential, asking about a worker that belongs to tenant_b).

Assertions:

```rust
let owned   = owned_headline   .bullets.iter().filter(|b| b.as_str() == "2 PRs reviewed today").count();
let leaked  = cross_tenant_headline.bullets.iter().filter(|b| b.as_str() == "5 PRs reviewed today").count();

assert_eq!(leaked, 0, "tenant A query must not count tenant B worker activity; got {:?}", ...);
assert!(owned >= 1,  "vacuous-pass guard: tenant A's own activity must still be visible; got {:?}", ...);
```

**Why this is non-vacuous**:

- Without the `owned >= 1` clause, a regression that broke the `tenant_id = ?` predicate but ALSO returned 0 rows for everything (e.g. `WHERE 1=0`) would silently pass. The `owned >= 1` is the round-37 guard explicitly cited in the test doc-comment.
- Without `s.tenant_id = ?` in the SQL, the cross-tenant query would find `worker_b`'s 5 audit rows via the `s.worker_id = ?` clause alone (workers are globally-unique UUIDs, so `worker_b` is reachable from any session). The bullet would render as `"5 PRs reviewed today"` and `leaked` would be `1` → test FAILS loudly.
- The `pr-checker` vertical formatter (per `lib_tests.rs` precedent) renders `tool_calls` count as the "PRs reviewed today" bullet. So the count-to-bullet mapping is meaningful, not arbitrary string-match.

### Production-callsite orphan check

`generate_team_headline` and `activity_counts_for_worker` production callsites (filtered for `src/`, excluding intra-crate `lib.rs` / `compute.rs` / `lib_tests.rs` / new test):

```
crates/argo-cli/src/handlers/gateway/routes_team_list.rs:86:   generate_team_headline(w.id, &w.vertical_id, store)            ← OLD signature
crates/argo-cli/src/handlers/gateway/routes_worker_detail.rs:345: argo_team_headline::generate_team_headline(worker.id, ...)  ← OLD signature
```

Both grep hits are on `origin/main` HEAD `8b454f505` (pre-merge). **Both are updated in the same PR** (lines 86 and 345 of the post-merge file per the diff). Zero orphan callers — no other crate consumes either signature. ✅

### Anti-theater check

| Risk | Verified |
|---|---|
| Vacuous-pass (query returns 0 for everything) | `owned >= 1` guard explicitly asserted with `pr-checker` bullet match against `2 tool_calls` seed |
| Predicate revert (drop `AND s.tenant_id = ?`) | `leaked == 0` against `worker_b`'s 5 tool_calls seeded → would fail loudly |
| Wrong bind type (TEXT vs BLOB mismatch) | `tenant_id.0.as_bytes().to_vec()` matches `session.tenant_id BLOB` column |
| Orphan callers calling old 3-arg signature | grep returns only the 2 callsites both updated in the same PR |
| Wrong tenant stamp (sink-default leak) | new test is read-only on schema (uses the seeded session's tenant), but the sibling `claim_2_live_duty_runs.rs:340-415` already pins `audit_record.tenant_id` against `sink.tenant`-default; same pattern not needed here because team-headline doesn't write audit rows |

## Comparison to #2260 (r52)

Same canonical pattern: mandatory `tenant_id: TenantId` param + bind to `s.tenant_id = ?` BLOB + cross-tenant negative pin test with non-vacuous guard. PR #2260 covered `argo-store::skill_query` (claim-4 / claim-6); PR #2261 covers `argo-team-headline::generate_team_headline + activity_counts_for_worker` (claim-6 team headline surface). Both shipped within 24h of each other by the same author (codex), both follow the same shape. **Strong signal that the tenant-pinning playbook is now reproducible across the runtime surface area** — every primitive that touches `audit_record JOIN session` is being audited and pinned in turn.

## Verdict

🟢 **PRE-MERGE PASS** (P1). Recommend admin-merge in next claude-conductor tick.

35th anti-theater-honest verification this week.

## Gap / follow-up brief candidates

None new. PR #2261 is a clean self-contained tenant-bind. The PR body notes `crates/argo-team-headline/tests/surface_pin_test.rs` is absent on main — this is a P3 follow-up candidate (already in the carry-forward backlog as **(j) eval-team-headline-surface-pin**), no new authoring needed.

## Carry-forward brief candidates (12 total — unchanged from r52 + r53 has no new)

(a)–(j) carried verbatim from r41-r52 LATEST.md. (k) `fix-cli-skill-search-indirect-placeholder-fallback-error-message` (P3 ~30 LOC, r52). (l) `eval-skill-query-tenant-pin-extend-to-distinct-skill-verticals-isolation` (P3 ~40 LOC, r52).
