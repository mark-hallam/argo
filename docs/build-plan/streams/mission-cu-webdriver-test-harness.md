# Mission load-bearing wire — Webdriver-style offline test harness for CU loop

**Owner**: claude. **Depends on**: none (additive test infrastructure).
**Authored against**: origin/main `d53a8f71` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-webdriver-test-harness`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 item #97 ("Add Webdriver-style test harness for CU loop"). Today the only end-to-end CU test is `cu_smoke.rs` (gated behind real `E2B_API_KEY` + `ANTHROPIC_API_KEY` — costs money, non-deterministic, skipped in CI). There's no offline harness that drives the full executor loop with mocked Anthropic + mocked E2B for deterministic regression coverage. Mission Claim 5 ("real-world execution") regressions land silently because the test signal is absent.

## Scope

Build a NEW `argo_executor_computer_use::testing` module (gated behind `#[cfg(any(test, feature = "test-harness"))]`) exposing:

- `MockAnthropic` — implements the same trait/contract `ComputerUseExecutor` consumes for Anthropic calls; takes a scripted sequence of canned responses (`Vec<CannedResponse>`); each call pops the next response.
- `MockE2bClient` — same idea for the E2B side; takes a scripted sequence of `(action, response)` pairs and asserts each action arrives in expected order.
- Helper `cu_harness_run(script: TestScript) -> TestRun` builds a `ComputerUseExecutor` wired to the mocks, runs the loop on a synthetic `RunRequest`, and returns the captured outcome + step trail for assertions.

Use the harness to write 5 offline regression tests covering:
1. Single-action turn → step → screenshot → next turn → stop_success
2. Multi-action turn (relies on `mission-cu-parser-multi-action` if merged; otherwise scoped to single)
3. Inline-screenshot path (relies on `mission-cu-double-screenshot-fix` if merged)
4. Error mid-loop: Anthropic returns malformed → CuError::Protocol bubbles up cleanly
5. Action sequence asserts E2B sees expected coordinates (regression guard for parser bugs)

Keep `cu_smoke.rs` unchanged — that's the real-world smoke; this brief adds the offline counterpart.

This is NOT in scope: replacing `cu_smoke.rs`; adding a property-test framework; mocking Anthropic at the HTTP level (we mock at the trait level, which is faster + less brittle); building a UI for replay.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use/src/testing.rs` (NEW, ≤ 400 LOC) — `MockAnthropic`, `MockE2bClient`, `TestScript`, `TestRun`, `cu_harness_run`, plus internal builder helpers. `#[cfg(any(test, feature = "test-harness"))]` gated.
- `crates/argo-executor-computer-use/src/lib.rs` — narrowly scoped: ONE `#[cfg(any(test, feature = "test-harness"))] pub mod testing;` line.
- `crates/argo-executor-computer-use/Cargo.toml` — narrowly scoped: ADD `[features] test-harness = []` if absent. NO other Cargo edits.
- `crates/argo-executor-computer-use/tests/cu_harness_offline.rs` (NEW, ≤ 300 LOC) — the 5 offline regression tests. Imports `argo_executor_computer_use::testing::*` (gated by the feature in dev-dependencies).
- `crates/argo-executor-computer-use/src/anthropic.rs` — narrowly scoped: IF the Anthropic call site uses a concrete struct, add a trait abstraction (`pub trait AnthropicCall`) so `MockAnthropic` can implement it. Wrap the existing impl as the production trait impl. NO behavior change to the production path. If a trait abstraction is structurally too disruptive, BLOCK with reason and propose `mission-cu-anthropic-trait-extraction` as a prerequisite brief.
- `crates/argo-executor-computer-use/src/e2b.rs` — same pattern: extract `pub trait E2bCall` if needed. The existing `E2bClient` becomes the production impl.

## Files you must NOT touch

- `crates/argo-executor-computer-use/tests/cu_smoke.rs` — real-world smoke; orthogonal
- `crates/argo-executor-computer-use/src/learn.rs` — feedback loop; orthogonal
- `crates/argo-cli/**` — no CLI exposure
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `testing` module exposes `MockAnthropic`, `MockE2bClient`, `TestScript`, `TestRun`, `cu_harness_run`
- [ ] Mocks are deterministic: same script → same outcome, byte-identical
- [ ] 5 offline regression tests live in `tests/cu_harness_offline.rs` and pass without ANY env vars set
- [ ] Tests run under 2 seconds total (no real network, no real LLM)
- [ ] If trait extraction was needed, the production path (running against real E2B + real Anthropic) is byte-identical to today's behavior — `cu_smoke.rs` continues to pass when env vars are set
- [ ] `MockAnthropic`'s scripted responses include ALL the response shapes the parser supports (tool_use click / move / type / key / stop_success / stop_failure)
- [ ] `MockE2bClient` records every action it received in order; tests assert against the recorded sequence
- [ ] `cargo build -p argo-executor-computer-use` clean (default features)
- [ ] `cargo build -p argo-executor-computer-use --features test-harness` clean
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use` passes (skip pre-existing flakes per KNOWN_FLAKES.md); the new offline tests pass without env vars
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-webdriver-test-harness`
- PR title: `cu-executor: offline test harness with MockAnthropic + MockE2bClient (mission-cu-webdriver-test-harness)`

## Notes for the implementer

- First 10 minutes: confirm the call shape on the production path. If `executor.rs` calls `self.anthropic.next_step(...)` against a concrete `AnthropicClient` struct, the trait extraction is small. If it calls a free function `anthropic::next_step(...)`, the trait extraction is bigger — consider whether to BLOCK and propose the prerequisite brief.
- The trait surface should be MINIMAL: whatever the executor calls today on the Anthropic side gets a `pub trait` of exactly that surface. Don't anticipate future needs.
- For the `MockE2bClient`, prefer the "scripted" pattern (caller sets up expected actions in advance) over the "callback" pattern (caller registers a closure). Scripted is easier to debug when an assertion fails — the test panic includes the expected vs received sequence.
- If a test's script doesn't match what the real loop emits (e.g. one fewer screenshot than expected), that's a signal — the brief calls those out as "regression guards." Document in test rustdoc what's being protected.
- The `test-harness` feature exists so external crates (e.g. a future `argo-executor-computer-use-macos` test crate) can reuse the mocks. v1 doesn't need that, but the feature is cheap insurance.
- Don't gate against compilation under non-test default features — keep the production path cargo-default unaffected.
