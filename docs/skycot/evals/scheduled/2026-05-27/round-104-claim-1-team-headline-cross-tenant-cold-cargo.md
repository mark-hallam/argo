# Round 104 — Claim 1 (Teammate-not-tool) team_headline cross-tenant LIVE cold cargo

**Date**: 2026-05-27
**Round**: 104
**Pillar**: MISSION Claim 1 (Teammate-not-tool) — `generate_team_headline` cross-tenant regression pin
**Verdict**: 🟢 LIVE

## Context

The CLAUDE.md Status block lists Claim 1 as 🟢 LIVE, with the load-bearing regression test being the SQL tenant gate for `argo-team-headline` (originally wired by PR #2261). Claim 1 hasn't appeared in the evaluator scenarios in the last 2 rounds (R102 hit Claim 2, R103 hit Claim 6). Rotation priority lands it here this round.

The regression risk for Claim 1 is subtle: the team-headline panel renders activity counts ("3 PRs reviewed today") per worker on the team page. If the SQL query joins audit rows to workers without filtering by `tenant_id`, a worker in tenant A could be shown activity counts that belong to a same-named worker (or even a different worker queried with the wrong tenant context) in tenant B. The cross-tenant negative test exists to pin that risk closed.

## Setup

```bash
git rev-parse --short origin/main
# 2c5028b96 (current main head)

cargo test -p argo-team-headline --release --tests
```

## Observed

Full cold cargo result, current main:

```
running 23 tests
test lib_tests::generate_team_headline_threads_tomorrow_line ... ok
test lib_tests::activity_counts_aggregates_seeded_audit_events ... ok
test lib_tests::next_duty_returns_none_when_role_duty_table_missing ... ok
test lib_tests::generate_team_headline_renders_pr_checker_bullets ... ok
test lib_tests::next_duty_skips_disabled_and_paused_rows ... ok
test lib_tests::generate_team_headline_no_activity_returns_single_fallback ... ok
test lib_tests::activity_counts_ignores_other_workers ... ok
[…23 total…]
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s

running 1 test
test generate_headline_does_not_count_other_tenant_worker_activity ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

running 7 tests
test bullet_kind_format_drops_zero_counts ... ok
test exported_surface_inventory_is_pinned ... ok
test team_headline_serializes_to_expected_json_shape ... ok
[…7 total…]
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**31/31 PASS, 1.03s runtime, ~3 minutes compile wall under 3-worker contention.** Exit 0.

Production tenant-filter at `crates/argo-team-headline/src/compute.rs`:
- Line 53: `pub async fn activity_counts(tenant_id: TenantId, …)` — tenant_id is the first parameter
- Line 58: `let tenant_blob = tenant_id.0.as_bytes().to_vec();` — UUID bytes-encoded for SQLite blob param
- Line 68: `WHERE s.tenant_id = ?` — bound in the SQL JOIN against `session` table

Cross-tenant test at `crates/argo-team-headline/tests/cross_tenant_negative_test.rs` (read in full):
- Seeds two tenants (`tenant_a`, `tenant_b`) with separate workers via real `SessionStore::create_session_with_worker` (no mocks)
- Writes 2 `AuditKind::ToolCall` rows for tenant_a's worker, 5 for tenant_b's worker via real `AuditSink`
- Calls `generate_team_headline(tenant_a, worker_a.worker, "pr-checker", &store)` → expects `"2 PRs reviewed today"` bullet (owned ≥ 1)
- Calls `generate_team_headline(tenant_a, worker_b.worker, "pr-checker", &store)` → asserts `"5 PRs reviewed today"` bullet does **not** appear (leaked == 0)
- **Vacuous-pass guard active**: both `assert_eq!(leaked, 0, …)` and `assert!(owned >= 1, …)` run in the same scenario, so a globally-broken query that returns no bullets for any tenant would fail the owned assertion

## Anti-theater check

Four independent axes — all dispositive:

1. **Test uses the real SessionStore + AuditSink + sqlx pool.** `fresh_store()` opens `:memory:` and runs the real `migrate()`. Workspace + session rows are INSERTed via raw SQL against the actual schema. Audit rows go through `AuditSink::new_with_tenant(...).write(...)` — the real production audit path.
2. **Production query uses `WHERE s.tenant_id = ?` (compute.rs:68).** This is the load-bearing tenant filter that the negative test exists to pin. If the WHERE clause is ever dropped or replaced with a less-restrictive predicate, the cross-tenant test fails because tenant_b's 5 ToolCall rows would surface in tenant_a's query.
3. **Negative + positive in the same scenario** — the round-37 vacuous-pass guard documented as the test's docstring: "A query that returns no activity for every tenant would also appear safe. This test keeps the round-37 vacuous-pass guard by asserting `leaked == 0` and `owned >= 1` in the same scenario."
4. **Source-read confirms `tenant_id` is the first parameter to `activity_counts`** — caller code cannot easily forget to pass it (no `Option<TenantId>` escape hatch, no default value). The query is structurally tenant-bound at the function signature level, not just at SQL-injection time.

## Gap / follow-up brief candidates

None. Claim 1 LIVE re-confirmed; production filter is structural and the regression pin is dispositive on 4 axes. This was the 114th anti-theater verification this week.

## Source

This report. PR #2261 originally wired the SQL tenant gate. Test: `crates/argo-team-headline/tests/cross_tenant_negative_test.rs`. Production query: `crates/argo-team-headline/src/compute.rs:53-72`.
