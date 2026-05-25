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

Brief #1 stood up `argo-demo-seed` with 5 real workers (per canon — Mariah/Peta/Priya/Jamal/Nori, per `mission-demo-seed-worker-roster-canon-peta-included`). That gets the `/workers` card deck populated. But the runbook's scenes 2, 3, 4, 5, 6 all need audit-record rows the SPA `/audit` page can render — `worker_duty.fired`, `workflow.step.executed`, `SkillInvoked`, `ApprovalAsked`. None exist on the seeded tenant after brief #1 beyond the baseline `worker.hired` rows.

This brief adds the audit-emission half on top of brief #1's scaffold, in **two modes** per the operator-clarified function-vs-dressing rule (memory `feedback_no_theater_in_demo`, reframe doc `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`):

- **Pass A (function-side, real LLM)**: ~10-20 turns total spread across workers. Each is a real `AgentRunner::run_turn` call against a real LLM provider, producing real `WorkerAction` + `audit_record` rows + real `approval_request` rows. These are the rows the demo DRILLS INTO during recording (Scene 6 hash-chain hover, Scene 2 Slack approval chain).
- **Pass B (dressing-side, no LLM)**: ~100-200 audit-record rows total, inserted through the **same audit-write path** as Pass A (so hash-chain integrity is preserved, `argo audit verify` still OKs) but with fixture-templated payload instead of LLM output. The agent runner is NOT invoked for Pass B rows; the seeder emits a synthetic `WorkerAction` from a templated catalog and writes the audit row through the normal `AuditSink::emit` API. Camera doesn't drill into these rows — they exist only as visible filler in the `/audit` page list.

**Clarification 2026-05-25T08:10Z**: the original spec (~150 turns all Pass A) was over-engineered. The function-vs-dressing rule allows Pass B as long as it goes through the same audit-write API (chain integrity preserved). Pass B rows are NOT theater under the clarified rule because the camera doesn't drill into them.

## Acceptance criteria

1. New module `crates/argo-demo-seed/src/driver.rs` exports:
   - `pub async fn drive_agent_turns(deps: &AgentDeps, tenant_id:
     &TenantId, profile: SeedProfile) -> ArgoResult<SeedReport>`.
   - `SeedProfile` enum with at minimum `Default` (the canonical v1
     demo workload) — extensible for future per-vertical profiles.
   - `SeedReport` struct with counts: `turns_run`, `audit_rows_created`,
     `approvals_created`, `errors`.
2. The driver — **Pass A (function-side)**:
   - Reads the provider key from env var `ARGO_DEMO_SEED_PROVIDER_KEY` (NOT `ANTHROPIC_API_KEY` — avoids accidentally burning the operator's primary credits).
   - **Fails closed** if `ARGO_DEMO_SEED_PROVIDER_KEY` is unset AND a Pass A turn is requested. Returns `ArgoError::missing_provider_key`. No fallback to canned responses. No fallback to the operator's primary key. (Pass B-only runs can proceed without the env var — see acceptance #3.)
   - Drives **10-20 turns total** spread across workers (configurable via `SeedProfile`), targeting the specific drill-in beats the runbook scenes hit: ≥3 Slack-approval `ApprovalAsked` rows for Scene 2, ≥3 `workflow.step.executed` rows for Scene 5 (or 0 if Scene 5 runs the workflow live on camera), ≥3 `SkillInvoked` rows for Scene 4, ≥1 `worker_duty.fired` row for Scene 6 hash-chain hover.
   - For each Pass A turn, invokes the existing `WorkerSessionRunner` pipeline so audit rows are written through the same production path real customer traffic uses.
3. The driver — **Pass B (dressing-side)**:
   - Inserts **100-200 audit-record rows total** (configurable via `SeedProfile`) WITHOUT invoking the agent runner. Each Pass B row:
     - Has a real `worker_id` from Pass A's seeded workers + a real `tenant_id`.
     - Has a payload from a templated fixture catalog (~5-10 templates per event-kind, randomized).
     - Is written through `argo_audit::AuditSink::emit` (the same API Pass A uses) so the hash chain stays intact and `argo audit verify` OKs.
     - Has a backdated `created_at_micros` spread across the past 14 days.
   - Pass B has no LLM cost. It runs even when `ARGO_DEMO_SEED_PROVIDER_KEY` is unset (useful for SPA UX-loop dev where the operator isn't recording).
   - The function-vs-dressing rule's "drill-in test" applies: the demo never drills into a Pass B row. The runbook's scene scripts are the source of truth for which rows are Pass A vs Pass B.
4. Approval-pipeline criterion: at least 3 of the Pass A driven turns produce an `ApprovalAsked` audit row + a real `approval_request` DB row, linked to a real `WorkerAction` row. The driver achieves this by selecting actions whose vertical pack declares `auto_approve = false` (e.g. Mariah's `publish_listing` action).
5. Backdating: audit `created_at_micros` for Pass A AND Pass B rows spread across the past 14 days. If no backdating override exists in `argo-audit`'s public API, expose one via a feature flag scoped to the `argo-demo-seed` crate (do NOT add a runtime override to `argo-audit` itself).
6. Cross-tenant invisibility extends to audit rows: after driving on tenant A (both Passes), `audit/recent?session=...` for sessions in tenant B returns rows from tenant B only. New regression test `tests/seed_audit_cross_tenant_invisibility.rs`. Cross-tenant test must include both Pass A and Pass B rows.
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

Under the function-vs-dressing relaxation, default profile is **10-20 Pass A LLM turns** total (was 150 under the original spec). At ~$0.01-0.03 per turn that's **$0.10-0.60 per full seed** (was $1.50-4.50 — ~10× cheaper).

Pass B has no LLM cost — it's direct fixture-templated audit-row emission through `AuditSink::emit`.

The `ARGO_DEMO_SEED_PROVIDER_KEY` env-var requirement remains the load-bearing guardrail for Pass A — without it Pass A fails closed, no LLM turns run. Pass B runs independent of the env var.

CI must NOT set `ARGO_DEMO_SEED_PROVIDER_KEY`. The Pass A integration test SKIPS when unset; the Pass B integration test runs in CI (no LLM cost).

## Sizing

~400-600 LOC across the driver + profile + 2 tests. Single-worker
brief. The "fails-closed when env var unset" check + the gated
test SKIP-when-unset are the load-bearing acceptances.

## Branch + PR

- Branch: `claude-mission-demo-seed-real-llm-driver`
- PR title: `demo-seed: real-LLM agent-turn driver, fails-closed
  on ARGO_DEMO_SEED_PROVIDER_KEY (mission-demo-seed-real-llm-driver)`
