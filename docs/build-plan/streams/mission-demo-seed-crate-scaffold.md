# mission-demo-seed-crate-scaffold — Stand up argo-demo-seed crate + 6 real workers + cross-tenant test

**Owner**: claude.
**Depends on**: none.
**Blocks**: `mission-demo-seed-real-llm-driver`, `mission-demo-seed-runbook-integration`.
**Authored against**: origin/main `ea0e00353` on 2026-05-25.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-crate-scaffold`.

🔴 **P0 DEMO-GATE — split-child #1 of 3.** Parent brief
`mission-demo-seed-data-bootstrap` was BLOCKED 2026-05-25 because a
single worker can't drive real-LLM turns + an approvals pipeline +
runbook integration in one PR without burning operator credits.
BUILD_STATUS.md line 204 recommends a 3-brief split. This is #1.

## Why this brief exists

The v1.0 demo MP4 is blocked because the demo workspace has zero
seeded data — no workers, no approvals, no audit rows the SPA
`/audit` page can render. Per the `🚨 NO THEATER IN THE DEMO`
operator amendment (BUILD_STATUS header), the seed CANNOT use
`web/lib/sample-*.ts` fixtures at runtime; it must produce real DB
rows via real production code paths.

This brief stands up the **crate scaffold + worker creation** half
only. It does NOT drive real LLM turns (that's brief #2) and does
NOT add a CLI subcommand or touch the runbook (that's brief #3).

By shipping this in isolation:
- A new `crates/argo-demo-seed` crate exists with a `seed_workers`
  public API.
- 6 named workers (Mariah, Priya, Jamal, Nori, Peta, Patrick) can
  be inserted into a real tenant via `IdentityStore::create_worker`.
- Cross-tenant invisibility is pinned by a regression test.
- No agent turns, no approvals, no audit_record rows beyond the
  baseline `worker.hired` rows the production `create_worker` path
  already emits.

This unblocks brief #2 (LLM driver) to add agent turns on top of
the seeded workers, and brief #3 (CLI + runbook) to wire the
operator-facing surface.

## Acceptance criteria

1. New crate `crates/argo-demo-seed/` exists with:
   - `Cargo.toml` declaring deps on `argo-identity`, `argo-store`,
     `argo-core`, `tokio`, `tracing`, `serde`.
   - `src/lib.rs` exporting `pub async fn seed_workers(deps:
     &AgentDeps, tenant_id: &TenantId) -> ArgoResult<Vec<WorkerId>>`.
   - `src/personas.rs` with the canonical 6-worker spine (name,
     role, vertical_id, charter, persona traits) hard-coded as
     Rust constants. The seed reads from `web/lib/sample-personas.ts`
     ONLY as the human-readable design source — the actual Rust
     constants are the runtime input. (This avoids a runtime
     dependency on the SPA fixture file.)
2. The `seed_workers` function:
   - Calls `IdentityStore::create_worker` for each of the 6
     personas. Uses real ULID worker IDs.
   - Binds each worker to its declared `vertical_id`.
   - Returns the 6 created `WorkerId`s in deterministic order.
   - Is idempotent: running twice on the same tenant returns the
     existing 6 IDs (not 12 duplicates). The check is by
     `(tenant_id, worker_name)` uniqueness via existing identity
     primitives.
3. Cross-tenant invisibility regression test
   `tests/seed_cross_tenant_invisibility.rs`:
   - Seeds tenant A → asserts `IdentityStore::list_workers(tenant_a)`
     returns exactly 6 rows with the expected names.
   - Asserts `IdentityStore::list_workers(tenant_b)` returns 0 rows
     (different tenant sees nothing).
   - Re-seeds tenant A → asserts still exactly 6 (idempotent).
4. `cargo clippy -p argo-demo-seed --all-targets -- -D warnings` clean.
5. `cargo test -p argo-demo-seed` passes.
6. Every modified file < 500 LOC.
7. PR body includes the test output for the cross-tenant
   invisibility test (positive + negative both visible).
8. BUILD_STATUS.md updated with PR number, this brief's row flipped
   to IN_REVIEW.

## Parallel-surface declarations

- Grep'd `crates/` for existing `pub fn seed_workers` / `pub fn
  seed_demo` / `DemoSeed` / `argo_demo` types — none exist on
  origin/main. This crate is a new green-field surface.
- Grep'd for `crates/argo-demo-*` directories — none exist. No
  parallel demo-related crate to fold into.
- `IdentityStore::create_worker` confirmed at
  `crates/argo-identity/src/store.rs:132` — used as the seed's only
  worker-creation primitive.

## May-touch

- `NEW: crates/argo-demo-seed/Cargo.toml`
- `NEW: crates/argo-demo-seed/src/lib.rs`
- `NEW: crates/argo-demo-seed/src/personas.rs`
- `NEW: crates/argo-demo-seed/tests/seed_cross_tenant_invisibility.rs`
- `Cargo.toml` (workspace root — add `crates/argo-demo-seed` to
  `members`)

## Files you must NOT touch

- `crates/argo-identity/**` — consume its existing public API only,
  no edits.
- `crates/argo-store/src/migrations/**` — no migration is required;
  seed uses existing tables.
- `crates/argo-cli/**` — CLI wiring is brief #3's scope.
- `web/lib/sample-personas.ts` — read by humans as design source,
  not by Rust at runtime; do not change.
- `scripts/demo-dry-run*` — brief #3's scope.
- `docs/skycot/demos/v1-mission-mvp-runbook.md` — brief #3's scope.

## Boundary against brief #2 + #3

- Do NOT add any `WorkerSessionRunner` invocation. The seed in this
  brief is purely identity/worker rows. Agent turns are brief #2.
- Do NOT add a `argo demo seed` CLI subcommand. That's brief #3.
- Do NOT touch the runbook or dry-run script. That's brief #3.

## Sizing

~250-400 LOC. Single-worker brief (no manager package). The
cross-tenant test is the load-bearing acceptance — without it the
seed is just code, not evidence of correctness.

## Branch + PR

- Branch: `claude-mission-demo-seed-crate-scaffold`
- PR title: `demo-seed: stand up argo-demo-seed crate + 6 real
  workers via IdentityStore (mission-demo-seed-crate-scaffold)`
