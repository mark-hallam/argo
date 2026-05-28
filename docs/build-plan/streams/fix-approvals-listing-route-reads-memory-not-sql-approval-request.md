# fix-approvals-listing-route-reads-memory-not-sql-approval-request

**Owner**: claude or codex.
**Authored against**: origin/main `a6c0be33e` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-listing-route-reads-memory-not-sql-approval-request`.
**Source**: UX-AUDIT-T0819MEL post-action diagnosis 2026-05-27 — `argo demo seed` wrote 11 `approval_request` SQL rows for the demo tenant but `/api/v1/approvals` returns `{"items":[]}` because the listing route reads from worker_actions memory, not the SQL table.

🔴 **P0 — Data-source mismatch between approval write paths and the listing API. SPA Scene 6 approvals drill-in renders empty even when the canonical demo seed has written rows.**

## Root cause

`crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs:59-130` (`list_response`) calls `collect_latest_actions(None)` → reads from worker_actions memory files (`open_workspace_memory`). The canonical demo seed (`crates/argo-demo-seed/src/`) writes approvals to the SQL `approval_request` table via the standard approval-request primitive. Neither side queries the other.

DB-level check confirmed 2026-05-27:

```
sqlite3 ~/.argo/sessions.db "SELECT lower(hex(tenant_id)), COUNT(*) FROM approval_request GROUP BY tenant_id;"
019e6785654d73029a29ff6035517c53|11
019e67803c3d7c318a218cbe1ef9fad9|3
```

But `curl -H "X-Argo-Tenant-Id: 019e6785-654d-7302-9a29-ff6035517c53" /api/v1/approvals` returns `{"items":[]}`.

The route's docstring says "Per-tenant filtering across a multi-tenant gateway is closed by `mission-enterprise-approvals-rbac`" — that confirms the SQL `approval_request` table is the future-canonical source, but the listing route never migrated.

## Findings this brief closes

- UX-AUDIT-T0819MEL S6 — approvals drill-in empty.
- The empty Scene 6 approvals tile in the v1.0 demo MP4 walkthrough.
- Any future audit pass that observes "approvals seeded but listing empty" should map to this brief.

## Fix recipe

Modify `list_response` in `routes_approvals_json.rs` to query the SQL `approval_request` table directly:

1. Add a `pool: &Pool<Sqlite>` parameter to `list_response` (it already has access to a `SessionStore` via the call site — surface it).
2. Replace the `collect_latest_actions(None).await` call with `sqlx::query_as` against `approval_request` filtered by tenant (matching the workspace tenant resolution used by `/team`, `/skills`, `/workflows`).
3. Decode rows into the existing `approval_row_json` shape — map SQL columns (id, tenant_id, worker_id, payload, state, requires_approval, created_at) onto the same JSON envelope `ApprovalRow` already produces from `ActionRecord`.
4. Keep the redaction + cursor pagination logic unchanged.
5. Optionally UNION with the memory-file path during a transition window if the swarm needs backward compatibility — but pure SQL-side is preferable.

Add a smoke test in `crates/argo-cli/src/handlers/gateway/routes_approvals_json_tests.rs` that seeds 3 `approval_request` rows via `seed_approval_for_test` and asserts the listing route returns 3 items.

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/api/v1/approvals` returns `>0` items against a seeded demo tenant (11 expected after `argo demo seed`).
- [ ] Cross-tenant negative: rows under a foreign tenant_id never surface (existing test pin holds).
- [ ] Pagination + redaction smoke tests still PASS.
- [ ] `cargo check -p argo-cli` PASSES.
- [ ] LOC budget for `routes_approvals_json.rs` stays under 500.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` (the listing route)
- `crates/argo-cli/src/handlers/gateway/routes_approvals_json_helpers.rs` (helper additions)
- `crates/argo-cli/src/handlers/gateway/routes_approvals_json_tests.rs` (smoke test)
- Possibly `crates/argo-store/src/approval_request.rs` if a list-by-tenant helper is missing

## Out of scope

- Backfilling the memory-file path to mirror SQL writes. The SQL table is canonical now.
- RBAC / cross-tenant filter changes — `mission-enterprise-approvals-rbac` covers those.
- Approval state transitions (decide / undo) — those already route through the right primitives.

## Effort

60-120 min. One route file + helpers + smoke test.

## Why this matters

Closes the last of the 4 endpoint data-source mismatches surfaced during the 2026-05-27 post-seed verification. With `fix-workflow-seed-promote-to-head-via-set-head` + the audit-scope fallback fix already in flight, Scene 6 approvals is the remaining drill-in lens for the v1.0 demo MP4. Without it, the recording opens to an empty approvals tile despite the SQL table holding 11 rows.
