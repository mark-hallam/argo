# Stream mission-team-headline-tenant-bind — bind tenant_id through generate_team_headline + activity_counts_for_worker

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `2963e5426` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-team-headline-tenant-bind`.
🟡 **P1 CLAIM-1 SUBSTRATE TENANT-BIND.** Explicit follow-up named in the BUILD_STATUS row for `eval-team-headline-crate-surface` (#2211 IN_REVIEW). The audit found `generate_team_headline` API and `compute::activity_counts_for_worker` SQL are `WorkerId`-scoped only — production callers `routes_team_list` and `routes_worker_detail` tenant-gate at the route layer but the underlying SQL doesn't bind tenant. Cross-tenant leakage is possible if a future caller forgets the route-layer gate.

## Scope

Push `tenant_id: TenantId` through the two team-headline surfaces so the SQL JOIN binds it explicitly. Match the pattern from #2146 (`mission-approval-decide-request-tenant-pinned`) which added `tenant_id: TenantId` + `WHERE id = ? AND tenant_id = ?` to `approvals::decide_request`.

Current shape:

- `pub async fn generate_team_headline(worker_id: WorkerId, vertical_id: &VerticalId, store: &Arc<SessionStore>) -> ArgoResult<TeamHeadline>` (`crates/argo-team-headline/src/lib.rs:73`)
- `pub async fn activity_counts_for_worker(store: &Arc<SessionStore>, worker_id: WorkerId, now: DateTime<Utc>) -> ArgoResult<ActivityCounts>` (`crates/argo-team-headline/src/compute.rs:51`)
- SQL: `JOIN session s ON s.id = a.session_id WHERE s.worker_id = ? AND a.created_at >= ?` — no tenant bind.

Target shape:

- `pub async fn generate_team_headline(tenant_id: TenantId, worker_id: WorkerId, vertical_id: &VerticalId, store: &Arc<SessionStore>) -> ArgoResult<TeamHeadline>`
- `pub async fn activity_counts_for_worker(store: &Arc<SessionStore>, tenant_id: TenantId, worker_id: WorkerId, now: DateTime<Utc>) -> ArgoResult<ActivityCounts>`
- SQL: `JOIN session s ON s.id = a.session_id WHERE s.tenant_id = ? AND s.worker_id = ? AND a.created_at >= ?` (verify `session` has a `tenant_id` column at claim time; if not, the JOIN needs to flow through `worker.tenant_id` instead — implementer's call, document in PR).

This is a **contract change** — the public API of `argo-team-headline` gains a required argument. Downstream callers (`routes_team_list` + `routes_worker_detail`) update in lockstep.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-team-headline/src/lib.rs` — add `tenant_id: TenantId` to `generate_team_headline` signature; pass it through to `activity_counts_for_worker`.
- `crates/argo-team-headline/src/compute.rs` — add `tenant_id: TenantId` to `activity_counts_for_worker` signature; update the SQL bind to include the tenant filter; add `.bind(tenant_blob)`.
- `crates/argo-team-headline/src/lib_tests.rs` — update existing tests to pass tenant_id (use a fixture `TenantId` per test).
- `NEW: crates/argo-team-headline/tests/cross_tenant_negative_test.rs` — vacuous-pass-guarded negative test: seed activity for two workers under two tenants, query under tenant A → assert tenant B's worker activity is not visible AND tenant A's own worker activity IS visible (the `leaked == 0 AND owned >= 1` pattern from #2146 round-37 evaluator note). Without the `owned >= 1` half, a broken query that returns nothing for both tenants would still pass.
- `crates/argo-cli/src/handlers/gateway/routes_team_list.rs` — update `generate_team_headline` call to pass the tenant_id already resolved at the route layer. Should be a one-line change at the existing callsite.
- `crates/argo-cli/src/handlers/gateway/routes_worker_detail.rs` — same one-line callsite update in `build_headline`.
- `crates/argo-team-headline/tests/surface_pin_test.rs` — if #2211 merged before this lands, update the surface pin to reflect the new signature (otherwise leave; the pin is the active contract).

## Files you must NOT touch

- `crates/argo-team-headline/src/headline.rs` or any KPI formatter logic — this brief is tenant-binding only, not algorithm work.
- Any callsite outside the 2 named routes (no `generate_team_headline` callers exist elsewhere in production code per `rg -l 'generate_team_headline' crates/`).
- `crates/argo-store/src/migrations/**` — no schema change is in scope; if you find `session.tenant_id` is missing, mark BLOCKED with reason "session table needs tenant_id column".
- `Cargo.toml` (no new deps; `TenantId` is already re-exported from `argo-types` and used at the callsites).

## Acceptance criteria

- [ ] `generate_team_headline` signature begins with `tenant_id: TenantId`
- [ ] `activity_counts_for_worker` signature includes `tenant_id: TenantId`
- [ ] SQL `WHERE` clause binds `s.tenant_id = ?` (or equivalent if joining via `worker.tenant_id`)
- [ ] Cross-tenant negative test exists at `crates/argo-team-headline/tests/cross_tenant_negative_test.rs` and uses the vacuous-pass guard pattern (`leaked == 0 AND owned >= 1`)
- [ ] Both callsites (`routes_team_list.rs`, `routes_worker_detail.rs`) pass the route-layer tenant_id explicitly
- [ ] `cargo clippy -p argo-team-headline -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-team-headline` passes (including the new negative test)
- [ ] Every modified file < 500 LOC
- [ ] PR body cites the round-37 evaluator note's vacuous-pass guard requirement
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-team-headline-tenant-bind`
- PR title: `team-headline: bind tenant_id through generate + activity_counts (mission-team-headline-tenant-bind)`

## Notes for the implementer

- Template: read PR #2146 (`mission-approval-decide-request-tenant-pinned`, merged commit `d3e626698`) end-to-end before starting. The pattern (signature change + WHERE-clause bind + vacuous-pass-guarded negative test) is the locked template.
- The vacuous-pass guard is load-bearing. A negative test that only asserts `leaked == 0` passes vacuously when the SQL returns nothing for any input. Always pair with `owned >= 1` on the legitimate side. Round 37 of the scheduled-evaluator caught this anti-pattern.
- This audit fed the `argo-team-headline` 🟡 PARTIAL verdict in #2211; once this lands, the verdict flips 🟢 and the gap-manifest row for claim 1 / argo-team-headline closes.
- If `session.tenant_id` doesn't exist, the JOIN can flow through `worker.tenant_id` instead (worker has tenant binding via #1356-era work). Implementer documents the join shape choice in the PR body.
- Pre-existing test flakes in `argo-cli` (per `KNOWN_FLAKES.md`) reproduce on clean `origin/main`; HARD RULE 2 applies.
