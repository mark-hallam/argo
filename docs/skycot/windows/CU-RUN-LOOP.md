# Windows CU executor — Anthropic run-loop

**Brief**: `win-cu-executor-windows-run-loop` (P1 — DEC-054).
**Depends on**: `win-cu-executor-wire-into-resolve-executor` (#2920, MERGED),
`win-cu-screenshot-real-png-encode` (#2919, MERGED),
`win-computer-use-executor-first-build` (#2906, MERGED).
**Fleet**: WINDOWS. NO THEATER — this records what genuinely compiled and ran
on a real Windows host, with the honest scope of what is and is NOT verified.

## What landed

The native Windows CU executor previously shipped only step *primitives*
(`preflight`, `warm_up`, `execute_step_recorded`, screenshot) and the
`resolve_executor` dispatch arm landed as a preflight-and-handoff stub. This
brief adds the **Anthropic model-in-the-loop driver** so the Windows executor
can run a turn-taking CU session end to end:

- **`WindowsCuExecutor: CuStepRunner`** (`src/executor_loop.rs`) — implements
  the shared `argo_executor_computer_use::CuStepRunner` trait
  (`preflight`/`warm_up`/`capture_screenshot`/`execute_step_recorded`/
  `recover_after_error`), with `type Session = WindowsCuSession`. `warm_up` and
  `capture_screenshot` return the real primary-monitor PNG via the #2919
  encode path.
- **`run_loop_with_client<R, A>`** (`src/executor_loop.rs`) — the per-step loop:
  screenshot → model step → handle each response (tool-call or stop) → execute
  the action → re-screenshot → audit row → repeat. Generic over the Anthropic
  client (`A: AnthropicCall`) so a scripted client can drive it with **no API
  key and no live desktop**.
- **`WindowsCuExecutor::run(req)`** (`src/executor.rs`) — the public entry:
  preflight gate → warm-up → `AuditLog::create` → build `Profile` +
  `AnthropicClient` + `BudgetTracker` → call `run_loop_with_client` →
  `WindowsCuOutcome`.
- **Risk-tiered floor (DEC-051)** (`src/cu_risk.rs`) — AUTONOMOUS ops (observe
  + STAGE input: screenshot/scroll/cursor-move/`type`) run; ACT ops
  (click/`key`/drag/navigate/launch) are held for operator approval at the
  first occurrence; unknown → ACT (fail closed). Identical policy to macOS.
- **`resolve_executor` rewire** (`argo-cli/.../worker_actions_executor_cu_windows.rs`)
  — the Windows dispatch arm now calls `run()` (was: preflight + `HumanHandoff`
  stub) and maps `WindowsCuOutcome` → `ActionOutcome`. The auto-dispatch path
  passes `act_approved: false`, so the loop hands off at the first committing
  op — the operator's "Approve & run" is the only grant. Same posture as the
  macOS native executor.

## Why this mirrors macOS instead of calling the shared `run_loop`

The brief asked to "reuse the shared `run_loop`". That is **not reachable**:
`argo_executor_computer_use`'s shared loop is `pub(crate)` (not exported) **and**
builds a *concrete* `AnthropicClient` internally, so it can be neither (a) called
from this crate nor (b) driven by a scripted client for a hermetic test. macOS
hit the exact same wall and ships its own `run_loop_with_client` generic over
`A: AnthropicCall` — this module mirrors that proven pattern. The loop itself is
NOT re-invented: turn-taking, the DEC-051 ACT floor, budgeting, and audit-row
shape match the macOS/shared semantics byte-for-byte in behaviour.

True de-duplication (making the shared loop generic + `pub`) is a canonical-crate
refactor left to a follow-up. The macOS arms were re-verified **byte-unchanged**
by this brief.

## Verification (this Windows host, 2026-06-14)

All on **LATITUDE5440** — Windows 11 Pro 10.0.26200, `x86_64-pc-windows-msvc`,
rustc/cargo 1.96.0, branch `win-cu-executor-windows-run-loop`.

### 1. Windows CU crate — clippy `-D warnings` + full test suite (acceptance #1 + #2)

```text
$ cargo clippy -p argo-executor-computer-use-windows --all-targets -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 9m 31s
CLIPPY_EXIT=0
```

```text
$ cargo test -p argo-executor-computer-use-windows
running 55 tests
...
test run_loop_tests::act_op_held_when_not_approved ... ok
test run_loop_tests::observe_only_run_completes_without_approval ... ok
test run_loop_tests::budget_exhaustion_hands_off ... ok
test run_loop_tests::autonomous_loop_completes_with_mocked_steps ... ok
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests\windows_screenshot_real.rs   → 1 passed
     Running tests\windows_smoke.rs             → 1 passed
   Doc-tests argo_executor_computer_use_windows → 2 passed
TEST_EXIT=0
```

The 4 hermetic run-loop tests cover the acceptance behaviours with **no API key
and no live desktop** (a scripted `AnthropicCall` + `FakeRunner`): a mocked
autonomous run completes on `stop_success`; an ACT op is held when
`act_approved=false` (DEC-051 floor); an observe-only run completes without
approval; budget exhaustion hands off.

### 2. argo-cli — rewired dispatch compiles + cu_windows tests green

```text
$ cargo test -p argo-cli --bin argo cu_windows
   Compiling argo-cli v0.0.1 ...
   (clean — only pre-existing warnings: macOS-sibling dead-code on a Windows
    build + the unrelated `self_update_install::pid`; NO warnings from this change)
     Running unittests src\main.rs (...\argo-9977d9f79b35e333.exe)
running 6 tests
test ...cu_windows_tests::dispatch_on_windows_hands_off_when_preflight_gapped ... ok
test ...cu_windows_tests::maps_completed_and_handoff_outcomes ... ok
test ...cu_windows_tests::manifest_underscore_form_resolves_to_windows_kind ... ok
test ...cu_windows_tests::manifest_hyphen_alias_resolves_to_windows_kind ... ok
test ...cu_windows_tests::windows_kind_display_string_round_trips ... ok
test ...cu_windows_tests::unknown_kind_with_windows_in_name_does_not_resolve ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 3052 filtered out; finished in 0.03s
CLI_BIN_TEST_EXIT=0
```

### On `cargo build -p argo-cli` + the `argo.exe` file-lock (honest note, NO THEATER)

A plain `cargo build -p argo-cli` **compiles 100% clean** but fails its *final*
step with `error: failed to remove file argo.exe — Access is denied (os error
5)`. This is a Windows file-lock, **not a code defect**: 17 concurrent swarm
`argo.exe` worker processes hold the output binary, so cargo cannot swap it.
The `--bin argo cu_windows` run above proves the rewired crate compiles — it
builds the bin's hash-named test harness (`argo-9977d9f79b35e333.exe`), which
does not collide with the locked `argo.exe` — and the dispatch/mapping tests
pass. The macOS conductor's re-verify (acceptance #5: `cargo test -p argo-cli`
on an unlocked host) covers the full bin build plus the non-cfg
`maps_completed_and_handoff_outcomes` test on that platform.

## Honest gaps (NO THEATER)

1. **No live end-to-end run with a real API key + real desktop.** The loop is
   verified *hermetically* (scripted Anthropic client, fake runner). Driving a
   real Claude CU session against the live Windows desktop is the operator-gated
   `win-cu-executor-end-to-end-run` brief (needs a real `ANTHROPIC_API_KEY` and
   live-desktop input permission). This brief does **not** claim that path works
   — only that the loop, the floor, the budget exit, and the outcome mapping are
   correct under a mocked client.
2. **Shared `run_loop` is still duplicated, not de-duplicated.** See the section
   above — promoting the shared loop to generic + `pub` is a follow-up.
3. **`recover_after_error` is a soft stub.** It returns a one-line note rather
   than attempting a restart/re-warm; the Windows restart path is not yet
   exercised (macOS has the richer restart-coordinator behaviour).
