# codex-macos-smoke-real-action — Replace argo-help smoke with real macOS CU action execution (F-10 P1, demo-blocker)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `ad581eca` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-macos-smoke-real-action`.
**Source**: F-10 P1 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Scope

🔴 V1 DEMO BLOCKER. The `cu_macos_dispatch_smoke` integration test claims to verify "the runtime reaches the new dispatch arm" but the test body literally executes only `argo --help`:

```rust
// crates/argo-cli/tests/cu_macos_dispatch_smoke.rs:39-41
let out = Command::new(argo_bin())
    .args(["--help"])
    .output()
    .expect("argo binary must be buildable for this smoke");
```

This means the smoke scenario #20 in `scripts/smoke-skycot.sh` was a false-pass — the v1 demo gate per `V1_DEFINITION.md:165` was not actually proved by the 2026-05-16 smoke run.

The 2026-05-15 codex audit's F-001 work (#1719 + #1722 — autonomous macOS CU run loop) IS real code; only the SMOKE TEST is broken. This brief's scope is fixing the proof gate, not re-implementing the executor.

Scope:

- Rewrite `cu_macos_dispatch_smoke.rs` to actually exercise the macOS CU dispatch path:
  - Build a minimal vertical manifest with `executor.kind = "computer_use_macos"` and one action.
  - Invoke `argo worker actions execute <action>` (or whatever the v1 entry point is) against that manifest.
  - Assert the dispatch result is one of: `ActionOutcome::Completed { ... }` (autonomous success), or `ActionOutcome::HumanHandoff { bundle_path }` (preflight failed legitimately) — but NOT a silent skip and NOT an error from the executor missing.
  - On non-macOS, the `#![cfg(target_os = "macos")]` at top of file is correct; leave that intact so non-macOS builds don't try to exercise the path.
- Add a parallel smoke that runs under `ARGO_NATIVE_MACOS_INTEGRATION=1` — this gates against requiring macOS Accessibility/Screen-Recording permissions and only runs on the operator's demo machine. The default smoke should run without this env var and still exercise the dispatch resolution.
- Document in the test file's header comment: what "passes" means and what it does/doesn't prove.

LOC: ~150 (test rewrite + manifest fixture + assertion plumbing).

## Files you may touch

- `crates/argo-cli/tests/cu_macos_dispatch_smoke.rs` — full rewrite. Replace `argo --help` with real dispatch execution against a fixture manifest. ≤120 modified lines.
- `crates/argo-cli/tests/fixtures/cu_macos_smoke_manifest.toml` (NEW) — minimal vertical manifest pinning `computer_use_macos` for one test action. ≤40 LOC.
- Test sibling for the `ARGO_NATIVE_MACOS_INTEGRATION=1` path if cleanly extractable. ≤60 LOC.

## Files you must NOT touch

- `crates/argo-executor-computer-use-macos/**` — the macOS CU executor is correct; only the SMOKE TEST is broken.
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` — adapter is correct.
- The CuStepRunner trait from #1719 — that's the production-side fix already shipped.
- `scripts/smoke-skycot.sh` — scenario 20 invocation pattern stays the same; just the underlying test is being fixed.

## Acceptance criteria

- [ ] `cu_macos_dispatch_smoke.rs` does NOT call `argo --help` anywhere in its test body.
- [ ] Default test (no `ARGO_NATIVE_MACOS_INTEGRATION`) exercises CU dispatch resolution + asserts a valid `ActionOutcome`.
- [ ] When `ARGO_NATIVE_MACOS_INTEGRATION=1`, test exercises the autonomous Anthropic CU loop end-to-end on macOS.
- [ ] On non-macOS, `#![cfg(target_os = "macos")]` correctly skips the test (no false-pass via skip path).
- [ ] Test fixture manifest is checked in + parsed by the test.
- [ ] `cargo test -p argo-cli --test cu_macos_dispatch_smoke` passes on macOS.
- [ ] `bash scripts/smoke-skycot.sh ...` scenario #20 reports a meaningful pass (i.e., not a skip-disguised-as-pass).
- [ ] PR body explicitly acknowledges this fixes the 2026-05-16-audit F-10 false-positive on yesterday's smoke run.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-macos-smoke-real-action`
- PR title: `test(cli): replace argo-help with real macOS CU action smoke (codex-macos-smoke-real-action) — F-10 v1 demo blocker`

## Notes for the implementer

- The `#![cfg(target_os = "macos")]` at file top is correct. Don't remove it.
- The brief from #1722 (`codex-cu-macos-run-loop-wiring`) explicitly maps `stop_success → ActionOutcome::Completed` and preserves `HumanHandoff` for preflight-failure. The smoke should accept either as a successful proof — the executor working autonomously OR cleanly degrading to handoff.
- DO NOT lower the assertion to "test binary builds." That's what the current broken test does.
- The fixture manifest can be minimal: 1 action, 1 vertical. Tools like `gtm-engineer` or `marketing-intern` already exist as full verticals; build a smaller one to keep test focused.
- After this brief lands, the v1.0 demo recording can resume — the smoke proof will actually mean something.
