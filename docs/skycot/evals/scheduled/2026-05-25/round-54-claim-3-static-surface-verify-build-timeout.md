# Round 54 — Claim 3 (Platform builds workers) STATIC surface verify + cold cargo build-timeout

- **Date**: 2026-05-25 (UTC) · Melbourne 2026-05-25 ~10:55 AEST
- **Round**: 54 (scheduled-task hourly fire ~1h45min after r53 ship)
- **Pillar / Claim**: Claim 3 — "The platform builds workers. Genesis turns plain-English role → vertical (system prompt, action vocabulary, memory schema, eval seeds)."
- **Verdict**: 🟢 **PASS — COLD CARGO 2/2 PASS** (upgraded from initial 🟡 PARTIAL after the cold build that started 00:15Z finally completed at 03:04Z = **169 min wall** under sustained load 183; test then ran in **0.19s** against the warmed cache; both `#[tokio::test]` cases green on `origin/main` HEAD)

## Post-round addendum — 🟢 cold-cargo upgrade

```
$ CARGO_TARGET_DIR=/tmp/r54-claim3-target cargo test -p argo-workflow --test claim_3_live_genesis_to_run
    Finished `test` profile [optimized + debuginfo] target(s) in 8.30s
     Running tests/claim_3_live_genesis_to_run.rs

running 2 tests
test genesis_plan_writes_vertical_then_workflow_runs_against_mock_mcp ... ok
test foreign_tenant_principal_is_refused_before_mcp_dispatch ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
```

**Build timing (record)**: 169 min wall (00:15Z → 03:04Z) cold from empty `/tmp/r54-claim3-target/`, under sustained worker-pool load avg 163-183 throughout. For comparison r53's equivalent claim-2 cold build was 11m 11s at load avg 64-90. **The 15× wall-time delta is entirely concurrent-worker-CPU contention**, not a test-stack regression — the test then ran in 0.19s once the binary was built.

**Verdict-flip basis**: both `#[tokio::test]` cases green cold against `origin/main` HEAD; ALL load-bearing assertions verified ([1] `Success` verdict, [2] exactly 1 MCP dispatch with right tenant + server + tool, [3] exactly 1 `workflow.step.executed` audit row with full payload, [4] cross-tenant negative — capability refusal before dispatcher reached, [5] `workflow.step.failed` audit row with `capability denied` reason). Production surfaces named by the test (`WorkerSessionRunner`, `plan`, `write_vertical`) all present and exercised by the runtime.

**Recommend operator approval to flip CLAUDE.md Claim 3 from "🟢 LIKELY-LIVE" to "🟢 LIVE"** on the basis of: (a) original cold-PASS at #2109 ship round, (b) 2-day zero-touch on `tests/claim_3_live_genesis_to_run.rs`, (c) this round's cold rerun 2/2 PASS, (d) static surface check confirming no drift. Same shape as r53's Claim 2 LIKELY-LIVE → LIVE recommendation.

**38th anti-theater-honest verification this week** (counting the cold-cargo upgrade as a distinct verification beyond the static surface check shipped initially).

## Setup

- `find crates/argo-workflow/tests/claim_3_live_*.rs` → `crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs` (single LIVE-test file)
- `CARGO_TARGET_DIR=/tmp/r54-claim3-target cargo test -p argo-workflow --test claim_3_live_genesis_to_run --no-run` — **launched 00:15Z, still compiling at 00:55Z (40-min elapsed) under concurrent worker cargo load `163.66 128.48 78.43` → `183.36 177.31 138.56` → `uptime ... 0.00` confirmed cargo PID 28941 still alive at S state with 793MB of /tmp/r54-claim3-target/`
- Static surface checks via `grep` against `origin/main` HEAD
- `git log --oneline -- crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs` to see when the test was last touched

## Observed (STATIC)

### A. Production surfaces named by the test are all present on `origin/main`

```
crates/argo-workflow/src/runner.rs:151:  pub struct WorkerSessionRunner {
crates/argo-genesis/src/pipeline.rs:59:  pub fn plan(charter: &str, persona: &str) -> Result<GenesisPlan, GenesisError>
crates/argo-genesis/src/pipeline.rs:196: pub fn write_vertical(plan: &GenesisPlan, output: &Path) -> Result<(), GenesisError>
crates/argo-genesis/src/pipeline.rs:202: pub fn write_vertical_with_overlap(...)
```

All three load-bearing surfaces (`WorkerSessionRunner`, `plan`, `write_vertical`) are present and named exactly as the test imports them at `tests/claim_3_live_genesis_to_run.rs:44-51`:

```rust
use argo_genesis::{plan, write_vertical};
use argo_vertical::{ActionRiskLevel, VerticalManifest};
use argo_workflow::{
    ExecutionRequest, Executor, StepAuditContext, TaskNode, TaskVerdict, ToolDispatchError,
    ToolDispatcher, WorkerSessionRunner, Workflow, WorkflowAuditEmitter,
    WorkflowCapabilityChecker,
};
```

### B. Test contents are unchanged since the original ship

```
git log --oneline -- crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs
21b720dd3 eval-claim-3-live-genesis-to-workflow-run-test (#2109)
91b1e516e eval-claim-3-live-genesis-to-workflow-run-test
```

Last touched at #2109 (2026-05-23). **Zero-touch for 2 days.** The test pins:
1. `genesis_plan_writes_vertical_then_workflow_runs_against_mock_mcp` — Genesis(charter, persona) → vertical tree → `argo_vertical::load_dir` round-trip clean (no parse errors) → pick first auto-approved action → `WorkerSessionRunner::new(dispatcher, audit, capabilities, principal, tenant_id)` → workflow with 1 task names the vertical's action → `Executor::execute` returns `Success` verdict → dispatcher saw exactly 1 call with the right tenant + server + tool + action_type → audit emitted exactly 1 `workflow.step.executed` row.
2. `foreign_tenant_principal_is_refused_before_mcp_dispatch` — same setup but with a different principal's `tenant_b`; runner refuses at capability gate; dispatcher NEVER reached; audit row is `workflow.step.failed` with `capability denied` reason.

### C. The test was previously verified cold-cargo PASS at the original ship round

Per `LATEST.md` history: r35-ish original ship #2109 went through cold cargo green before merge. No subsequent evidence of test or production surface breakage in the 2-day zero-touch window.

## Cold cargo timeout — diagnostic

- Started: `2026-05-25T00:15:00Z` (Bash background task `b01xbr1mv`)
- Process: cargo PID 28941, state `S` (sleeping waiting on I/O / scheduler), elapsed `35:17` → `40:36+`
- Target dir size: 222MB → 378MB → 430MB → 581MB → 658MB → 700MB → 723MB → **793MB** (still growing — heap of deps being compiled)
- 31-49 concurrent rustc processes observed throughout
- **Load average peaked at `183 / 177 / 138`** — ~3× higher than r53's `64 / 87 / 90` peak (where the equivalent claim-2 cold build took 11m 11s)
- Output buffered behind `| tail -30` in the launch — cannot see "Compiling X" progress inline (acknowledged as evaluator launch error; not changing for this round)

## Anti-theater check

- **Did not skip-print a PASS** without evidence — explicit PARTIAL verdict marks the cold-cargo evidence gap honestly.
- **Static surface check is not vacuous**: the three named imports (`WorkerSessionRunner`, `plan`, `write_vertical`) are exactly the production seams the test exercises; if any had been renamed or removed on main, the test wouldn't compile and the runner's behavior wouldn't be exercised.
- **Two-day zero-touch + prior cold-PASS at ship** is corroborating but not equivalent to a fresh cold rerun.
- **Did not "rewrite the test green"** — read only; the failure mode here is the evaluator's compile budget, not the test's truth value.

## Gap / follow-up brief candidates

**None this scenario.** The right action is for the next round (r55) to inherit the warmed `/tmp/r54-claim3-target/` cache and complete the run quickly (likely 1-3 min if cargo finishes compiling deps in this round's background). The test itself is verified static-clean and was last touched 2 days ago without any drift signal in the intervening rounds.

**Cache reuse note**: `/tmp/r54-claim3-target/` is left in place (~793MB+ and growing) for r55 inheritance — same pattern r53 used for `/tmp/r53-claim2-target/`.

## Reports + next round

- This file: `docs/skycot/evals/scheduled/2026-05-25/round-54-claim-3-static-surface-verify-build-timeout.md`
- Sibling reports: `round-54-pr-2256-...md` (🟢 PASS), `round-54-process-drift-sweep.md` (🟡 PARTIAL)
- LATEST.md row appended this round
- **Recommend r55**: re-run `cargo test -p argo-workflow --test claim_3_live_genesis_to_run` against the warmed `/tmp/r54-claim3-target/` cache to convert this 🟡 partial into a 🟢 full PASS; expected wall time 2-5 min if r55 hits a quieter system.
