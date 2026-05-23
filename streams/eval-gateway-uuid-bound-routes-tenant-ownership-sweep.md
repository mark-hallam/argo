# eval-gateway-uuid-bound-routes-tenant-ownership-sweep

**Priority**: P1 (claim-1 + claim-6 cross-tenant ownership hardening — third tick re-surfaced by evaluator round-37/38)
**Owner**: claude
**Type**: Audit + regression-test sweep (zero production code changes)
**Size**: ~150-300 LOC across one new test file + one Markdown audit report
**Authored against**: origin/main `01bbe9b58` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-gateway-uuid-bound-routes-tenant-ownership-sweep`

## Why

Two MERGED contracts on this codebase have proven that gateway routes which bind on UUID alone (without an `AND tenant_id = ?` ownership clause) are a recurring cross-tenant data-leak vector:

- **#2080 §5** (round-9, token-cache cwd-keyed) → fixed via #2103 (`mission-mcp-connector-token-cache-tenant-keyed`).
- **#2146** (`mission-approval-decide-request-tenant-pinned`) → `decide_request` `UPDATE WHERE id = ?` re-bound to `WHERE id = ? AND tenant_id = ? AND status = 'pending'` after evaluator round-33 + round-37 anti-theater verifies.

Round-38 explicitly calls out a third instance still in-flight (`mission-gateway-session-export-tenant-bind`), and notes the same SQL-bind-on-UUID-alone pattern is structurally present across **many** gateway handlers — but no canonical inventory exists. The evaluator has re-surfaced this exact brief **three consecutive rounds** (rounds 36/37/38). The pattern fix is one-line per route, but the audit-grep is wide enough that a single sweep is the only way to catch every instance before v1.0 ship.

This brief authors the sweep: enumerate every gateway route that binds on a UUID-shaped identifier (approval_id, session_id, worker_id, skill_id, audit_record id, run_id, message_id, etc.) and verify the underlying store method takes a `tenant_id` argument AND binds it in the SQL. The deliverable is a per-route verdict matrix + one new FINDINGS-ONLY regression test per uncovered route, pre-baked `#[ignore]` for the fix-brief that lands the parameter.

## What

### Step 1 — enumerate

`rg -n 'fn.+_id.+Path<' crates/argo-cli/src/handlers/gateway/` to enumerate routes binding a path-param UUID. Build a table at `docs/skycot/audit/2026-05-24-gateway-uuid-bound-routes/sweep.md`:

```
| Route | Handler | Store method | Binds tenant_id? | Verdict |
|---|---|---|---|---|
| GET /approvals/:id | routes_approvals.rs:42 | ApprovalStore::get_request | NO  | 🔴 LEAK |
| POST /approvals/:id/decide | routes_approvals.rs:84 | ApprovalStore::decide_request | YES (post #2146) | 🟢 OK |
| GET /sessions/:id/export | routes_sessions.rs:103 | SessionStore::export | NO  | 🔴 LEAK (fix in-flight: mission-gateway-session-export-tenant-bind) |
| ... |
```

### Step 2 — pin

For each 🔴 LEAK row, add a `#[ignore]` FINDINGS-ONLY test at `crates/argo-cli/tests/gateway_uuid_bound_routes_ownership_pin_test.rs` with the shape:

```rust
#[tokio::test]
#[ignore = "FINDINGS-ONLY — un-ignore + invert when <fix-brief-id> lands"]
async fn finding_<route_slug>_cross_tenant_currently_leaks() {
    // tenant_a writes row → tenant_b principal queries via route → asserts route currently returns row.
    // When fix brief lands the AND tenant_id = ? clause, un-ignore + invert (assert NotFound).
}
```

### Step 3 — index

Add the row count + LEAK count to the sweep report. Suggested follow-up briefs (named in the report, NOT authored here) close each LEAK individually — one per route — using the #2146 recipe verbatim.

## Acceptance

- [ ] `docs/skycot/audit/2026-05-24-gateway-uuid-bound-routes/sweep.md` exists and matrices every gateway route binding a path-param UUID.
- [ ] `crates/argo-cli/tests/gateway_uuid_bound_routes_ownership_pin_test.rs` adds one `#[ignore]` test per 🔴 LEAK route. Tests cite the sweep doc verbatim.
- [ ] `cargo test -p argo-cli --test gateway_uuid_bound_routes_ownership_pin_test --no-run` builds clean (so the `#[ignore]`'d tests are real, not dead code).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Zero production code touched (no `.rs` file outside the new test).
- [ ] Sweep report names a follow-up fix-brief per 🔴 LEAK, with a recommended ledger row shape.

## May-touch

- `docs/skycot/audit/2026-05-24-gateway-uuid-bound-routes/sweep.md` (NEW)
- `crates/argo-cli/tests/gateway_uuid_bound_routes_ownership_pin_test.rs` (NEW)

**Out of may-touch**: every production `.rs` file in `crates/argo-cli/src/`, every store crate. This is read-only.

## Parallel surfaces

- `mission-approval-decide-request-tenant-pinned` (MERGED #2146) — the exemplar this brief generalises. Reuse its test idiom for pin tests.
- `mission-gateway-session-export-tenant-bind` (in flight) — one of the routes the sweep will name. If MERGED before this brief claims, simply mark that row 🟢 OK in the sweep.
- `crates/argo-store/tests/connector_inventory_pin_test.rs` — the **structural** pin pattern. This brief's test file is the **behavioural** pin (different concern).
- `rg 'tenant_ownership' crates/` returns no hits — net-new test file.

## Sensible deviations

- If a route already correctly binds `tenant_id` via a wrapper (e.g. an Axum extractor that filters before reaching the handler), mark the row 🟢 OK and skip the pin test. The pin tests exist for **un-fixed** routes only.
- If `rg` enumeration surfaces >20 LEAK routes, group them by store crate (one section per crate) and recommend a per-crate fix-brief rather than per-route — keeps the brief queue clean.
- If a route binds a non-UUID identifier (e.g. user-chosen slug like `skill_name`), include it only if the underlying store table has a `tenant_id` column. Slugs without per-tenant uniqueness constraints are out of scope (different vulnerability class).
