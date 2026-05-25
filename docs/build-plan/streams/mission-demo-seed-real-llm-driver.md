# mission-demo-seed-real-llm-driver — Drive real-LLM agent turns over seeded workers, produce real audit + approval rows

**Owner**: claude.
**Depends on**: `mission-demo-seed-crate-scaffold` MERGED.
**Blocks**: `mission-demo-seed-runbook-integration`.
**Authored against**: origin/main `ea0e00353` on 2026-05-25.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-real-llm-driver`.

🔴 **P0 DEMO-GATE — split-child #2 of 3.** Parent context in
`mission-demo-seed-data-bootstrap` (BLOCKED) +
`mission-demo-seed-crate-scaffold` (the prereq scaffold brief).

## Why this brief exists

Brief #1 stood up `argo-demo-seed` with 6 real workers. That gets
the `/workers` card deck populated. But the runbook's scenes 2, 3,
4, 5, 6 all need audit-record rows the SPA `/audit` page can render
— `worker_duty.fired`, `workflow.step.executed`, `SkillInvoked`,
`ApprovalAsked`. None exist on the seeded tenant after brief #1
beyond the baseline `worker.hired` rows.

Per the `🚨 NO THEATER IN THE DEMO` operator amendment, these rows
must be produced by **real agent turns** through the existing
`WorkerSessionRunner` against a **real LLM provider**, not
hand-inserted via `INSERT INTO audit_record VALUES (...)`. That
would be theater.

This brief adds the LLM-driving half on top of brief #1's scaffold.

## Acceptance criteria

1. New module `crates/argo-demo-seed/src/driver.rs` exports:
   - `pub async fn drive_agent_turns(deps: &AgentDeps, tenant_id:
     &TenantId, profile: SeedProfile) -> ArgoResult<SeedReport>`.
   - `SeedProfile` enum with at minimum `Default` (the canonical v1
     demo workload) — extensible for future per-vertical profiles.
   - `SeedReport` struct with counts: `turns_run`, `audit_rows_created`,
     `approvals_created`, `errors`.
2. The driver:
   - Reads the provider key from env var `ARGO_DEMO_SEED_PROVIDER_KEY`
     (NOT `ANTHROPIC_API_KEY` — avoids accidentally burning the
     operator's primary credits).
   - **Fails closed** if `ARGO_DEMO_SEED_PROVIDER_KEY` is unset.
     Returns `ArgoError::missing_provider_key` with a remediation
     message. No fallback to canned responses. No fallback to the
     operator's primary key.
   - Drives ~25 turns per worker (configurable via `SeedProfile`),
     spread across a backdated 14-day window.
   - For each turn, invokes the existing `WorkerSessionRunner`
     pipeline so audit rows are written through the same production
     path real customer traffic uses.
3. Approval-pipeline criterion: at least 7 of the driven turns must
   produce a `ApprovalAsked` audit row + a real `approval_request`
   DB row, linked to a real `WorkerAction` row. The driver achieves
   this by selecting actions whose vertical pack declares
   `auto_approve = false` (e.g. Mariah's `publish_listing` action).
4. Backdating: audit `created_at_micros` for seeded rows is spread
   across the past 14 days (calculated from `now()` at seed-time).
   Use the existing audit-record write path; if no backdating
   override exists, expose one via a feature flag scoped to the
   `argo-demo-seed` crate (do NOT add a runtime override to
   `argo-audit` itself).
5. Cross-tenant invisibility extends to audit rows: after driving
   on tenant A, `audit/recent?session=...` for sessions in tenant B
   returns rows from tenant B only. New regression test
   `tests/seed_audit_cross_tenant_invisibility.rs`.
6. `cargo clippy -p argo-demo-seed --all-targets -- -D warnings` clean.
7. `cargo test -p argo-demo-seed` passes (NEW tests: the driver
   integration test gated on `ARGO_DEMO_SEED_PROVIDER_KEY` being set
   in CI — if unset, the test SKIPS with a printed reason, not
   FAILs; this lets the test live in main without burning credits
   on every PR build).
8. Every modified file < 500 LOC.
9. PR body includes:
   - A 15-line transcript of running `cargo test -p argo-demo-seed
     --test seed_driver_integration` with `ARGO_DEMO_SEED_PROVIDER_KEY`
     set (locally, not in CI), showing real turns running and audit
     rows being created.
   - The `SeedReport` returned by a successful run: turn counts,
     audit row counts, approval counts.
10. BUILD_STATUS.md updated with PR number, this brief's row
    flipped to IN_REVIEW.

## Parallel-surface declarations

- Grep'd `crates/` for `drive_agent_turns` / `SeedProfile` /
  `SeedReport` types — none exist. New surfaces.
- `WorkerSessionRunner` confirmed at `crates/argo-agent-runner/src/`
  — consume its existing public API only.
- `approvals::create_request` is test-only on origin/main (per the
  BLOCKED note on the parent brief). This brief either:
  - (a) routes through the production `decide_and_request_approval`
    path that already exists in the runner (preferred), OR
  - (b) flags this as a follow-up fix-brief if the runner doesn't
    expose that path. Do NOT introduce a parallel `create_request`
    public surface.

## May-touch

- `crates/argo-demo-seed/Cargo.toml` (add `argo-agent-runner`,
  `argo-audit`, `argo-approvals` deps)
- `NEW: crates/argo-demo-seed/src/driver.rs`
- `NEW: crates/argo-demo-seed/src/seed_profile.rs`
- `crates/argo-demo-seed/src/lib.rs` (re-export new public types
  only — no behavioural change to brief #1's `seed_workers`)
- `NEW: crates/argo-demo-seed/tests/seed_driver_integration.rs`
  (gated test)
- `NEW: crates/argo-demo-seed/tests/seed_audit_cross_tenant_invisibility.rs`

## Files you must NOT touch

- `crates/argo-identity/**` — brief #1's territory; no edits.
- `crates/argo-audit/**` — do not introduce a backdating override
  in this crate; scope any seed-only backdating to `argo-demo-seed`.
- `crates/argo-agent-runner/**` — consume existing public API only.
- `crates/argo-cli/**` — brief #3's scope.
- `crates/argo-store/src/migrations/**` — no migration in scope.
- `web/**` — no SPA work.
- `scripts/demo-dry-run*` — brief #3's scope.
- `docs/skycot/demos/v1-mission-mvp-runbook.md` — brief #3's scope.

## Boundary against brief #3

- Do NOT add a `argo demo seed` CLI subcommand. That's brief #3.
- Do NOT touch the runbook or dry-run script. That's brief #3.
- Do NOT add SPA proxy routes for `/api/v1/audit/events`. That's a
  separate fix-brief (`fix-spa-audit-api-v1-proxy-routes` —
  recommend authoring as a peer to brief #3).

## Cost guardrails

The driver runs ~150 LLM turns per `drive_agent_turns` call (25 ×
6 workers). At ~$0.01-0.03 per turn that's $1.50-4.50 per full
seed. The `ARGO_DEMO_SEED_PROVIDER_KEY` env-var requirement is the
load-bearing guardrail — without it the driver fails closed, no
turns run, no spend.

CI must NOT set `ARGO_DEMO_SEED_PROVIDER_KEY`. The integration
test SKIPS when unset.

## Sizing

~400-600 LOC across the driver + profile + 2 tests. Single-worker
brief. The "fails-closed when env var unset" check + the gated
test SKIP-when-unset are the load-bearing acceptances.

## Branch + PR

- Branch: `claude-mission-demo-seed-real-llm-driver`
- PR title: `demo-seed: real-LLM agent-turn driver, fails-closed
  on ARGO_DEMO_SEED_PROVIDER_KEY (mission-demo-seed-real-llm-driver)`
