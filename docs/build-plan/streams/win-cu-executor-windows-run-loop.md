# win-cu-executor-windows-run-loop

**Owner**: windows-fleet — **P1 — DEC-054: give Windows a real model-in-loop CU `run()` (the genuine prerequisite the BLOCKED E2E row identified).**
**Depends on**: #2921 (resolve_executor wire) MERGED; #2906/#2910/#2919 (primitives + real PNG) MERGED.
**Authored against**: origin/main `648930d73d6d29fda0ee208d63d560e6f74405af` on `2026-06-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-cu-executor-windows-run-loop`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform changes → conductor re-verifies unix (RUN tests, not just compile) before merge. Push branch `win-cu-executor-windows-run-loop`, flip row to IN_REVIEW; **do NOT self-merge** — the macOS conductor gates/reviews/merges.

## Why this exists (the BLOCKED E2E row diagnosed it)
The sibling brief `win-cu-executor-end-to-end-run` is **BLOCKED** because the wired dispatch arm routes to a preflight+`HumanHandoff` adapter, **not** a model-in-loop `run()` — and **no Windows CU `run()` loop exists**. The Windows crate ships only step primitives; there is no public `run()` and no `CuStepRunner` impl, so the shared loop can never drive it. This brief builds exactly that missing piece, mirroring macOS, with a HERMETIC verifier that needs NO API key and NO live desktop (the two operator-only gates that keep the E2E row blocked). Landing this unblocks the real E2E run.

## Problem statement (file:line evidence)
- `crates/argo-executor-computer-use-windows/src/executor.rs:124-129` — `execute_step()` is "One CU loop tick … The full `run()` loop builds on top of this." There is **no `run()` method** on `WindowsCuExecutor`.
- `crates/argo-executor-computer-use-windows/src/executor.rs:11-13` — module doc admits the executor is "not yet wired" (now wired by #2921, but the loop it would drive still does not exist).
- The shared loop `argo_executor_computer_use::run_loop` lives at `crates/argo-executor-computer-use/src/executor_step_helpers.rs:37`, generic over `R: CuStepRunner` (trait at `crates/argo-executor-computer-use/src/cu_step_runner.rs:30`). The Windows executor does **not** implement `CuStepRunner`, so it can never be passed in.
- macOS is the proven mirror: `MacosCuExecutor::run()` at `crates/argo-executor-computer-use-macos/src/executor.rs:167` calls `run_loop_with_client` (`executor_loop.rs:21`); `impl CuStepRunner for MacosCuExecutor` at `executor.rs:286`; hermetic `FakeRunner` test at `crates/argo-executor-computer-use-macos/src/run_loop_tests.rs:58` drives the loop with NO network and NO desktop.

## Fix (mirror the macOS layout — keep files <500 LOC, fns <80 LOC)
1. Add `WindowsCuSession` (the per-dispatch handle the trait's `type Session` needs), mirroring `MacosCuSession`.
2. `impl CuStepRunner for WindowsCuExecutor` in a NEW sibling file `crates/argo-executor-computer-use-windows/src/executor_loop.rs` (NOT in executor.rs — already 289 LOC; macOS keeps this in a sibling too). Map existing primitives onto the trait: `preflight` (gate on `WindowsPreflightReport::all_passed`), `warm_up`/`capture_screenshot` (→ `capture_all_monitors` / the real PNG path from #2919), `execute_step_recorded` (→ `post(action, &config)` from `events.rs`, returning the shared `ActionResult`), and `recover_after_error` handing off with a one-line reason (mirror macOS `executor.rs:333`).
3. Add `pub async fn run(&self, req: WindowsCuRunRequest) -> WindowsCuResult<WindowsCuOutcome>` that runs preflight (handoff with `report.actionable_summary()` if any axis fails — NO THEATER), builds the `AnthropicClientTemplate` from `req.anthropic_api_key`, constructs `Profile`/`BudgetTracker`/`AuditLog`, calls the shared `run_loop(...)`, and maps `LoopExit::{Success,Handoff}` → `WindowsCuOutcome::{Completed,HumanHandoff}`.
4. Have the resolve_executor arm (wired in #2921) call this real `run()` instead of the preflight+HumanHandoff stub — keeping the SAME default-OFF opt-in posture and truth-boundary/capability fence as the macOS native executor (no behavior change for macOS/Linux; conductor re-verifies the unix arms are byte-unchanged).

## NO THEATER
- If a Windows primitive genuinely can't drive the loop yet (e.g. an action kind not supported by `events::post`), `execute_step_recorded` returns an honest `CuError`/handoff for that action — never a fake `Ok`.
- The hermetic test uses a `FakeRunner` (a `CuStepRunner` impl with a scripted Anthropic client) — it proves the LOOP PLUMBING, and the test name/doc must say plainly it does NOT exercise real input injection or a real model.

## May touch (explicit)
- `crates/argo-executor-computer-use-windows/src/executor.rs` (add `run()` + `WindowsCuSession`; stay <500 LOC — push loop glue into the sibling).
- `crates/argo-executor-computer-use-windows/src/executor_loop.rs` (NEW — `CuStepRunner` impl + loop glue).
- `crates/argo-executor-computer-use-windows/src/lib.rs` (declare the new module + re-export `WindowsCuSession`).
- `crates/argo-executor-computer-use-windows/src/run_loop_tests.rs` (NEW — hermetic `FakeRunner` test, mirroring macOS).
- The resolve_executor Windows arm in argo-cli (grep `fn resolve_executor`) — swap the stub for real `run()`; keep unix/macOS arms byte-unchanged.
- `docs/skycot/windows/CU-RUN-LOOP.md` (NEW — the report).
- `docs/build-plan/FEATURE_MATRIX.md` (in-band ledger — see below).

## Parallel-surface declaration
- `run_loop` / `CuStepRunner` ALREADY EXIST in `argo-executor-computer-use` (canonical home). This brief does **not** add a parallel loop — it makes the Windows executor a `CuStepRunner` and reuses the shared `run_loop`. It MUST NOT duplicate the loop body (the very anti-pattern the BLOCKED row called out).
- `WindowsCuOutcome` / `WindowsCuRunRequest` / `WindowsStepRecord` already exist in `executor.rs` — extend/reuse, do not re-declare.
- `WindowsCuSession` is NEW; `rg 'pub (struct|enum|type) WindowsCuSession'` confirms no prior definition before declaring it.

## Acceptance criteria (CHECKABLE Windows verifier — fully autonomous)
1. ON REAL WINDOWS, hermetic + no key + no live desktop: `cargo test -p argo-executor-computer-use-windows` is GREEN, **including a NEW `run_loop_tests.rs` test that calls the real `run()` plumbing through a `FakeRunner` and asserts a `Completed` outcome on a scripted stop_success and a `HumanHandoff` on budget-exhaustion** (mirror macOS run_loop_tests.rs).
2. ON REAL WINDOWS: `cargo clippy -p argo-executor-computer-use-windows -- -D warnings` CLEAN, and `cargo build -p argo-cli` succeeds (the resolve_executor arm now calls `run()`).
3. `WindowsCuExecutor` implements `CuStepRunner` and exposes `run()`; the shared `run_loop` drives it (no duplicated loop body).
4. `docs/skycot/windows/CU-RUN-LOOP.md` records the test output + an honest statement of what the hermetic test does and does NOT prove (no key, no live desktop — the real model-in-loop run is the now-unblocked `win-cu-executor-end-to-end-run`).
5. Conductor re-verifies on macOS: the resolve_executor unix/macOS arms are byte-unchanged and `cargo test -p argo-cli` is green.
6. Branch `win-cu-executor-windows-run-loop` pushed; ledger row flipped to IN_REVIEW with the branch name. **Do NOT self-merge.**

## In-band ledger (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` row 169 (`Computer-use — Linux / Windows | 🟧 Scaffold | Real impls compile but 0 tests`) — flip the Windows half toward "🟡 Partial: real `run()` loop + hermetic loop tests on Windows; live model-in-loop run pending operator key/desktop", with the commit SHA + the `argo-executor-computer-use-windows` crate cited, in the SAME commit as the code. Do NOT commit the generated feature-matrix HTML.