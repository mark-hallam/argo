# eval-e2b-executor-substrate-status

- Date: 2026-05-24
- Brief: `eval-e2b-executor-substrate-status`
- Worker: codex, per operator override 2026-05-24 for `claude` rows
- Main anchor when claimed: `66ec26f06`
- Verdict: PASS. The E2B-backed `argo-executor-computer-use` substrate is
  honest as DEFERRED-v1.x: its current public surface is pinned, offline
  contracts are test-backed, and production callers still match the PR #2171
  allowlist exactly.

## Scope Guard

This audit did not wire E2B into any new production path, did not open a real
E2B sandbox, and did not call Anthropic. The new test uses source snapshots and
the crate's existing `MockAnthropic` / `MockE2bClient` harness.

The pre-claim freshness check flagged only SHA distance:
`ef55f5422` was 162 commits behind main. Path validation, may-touch headroom,
and deep stub checks all passed, so the brief remained actionable as a current
status audit.

## Public Surface Pin

New contract: `crates/argo-executor-computer-use/tests/substrate_status_contract.rs`.

The test snapshots the `src/lib.rs` public module and re-export lines. Any
addition, removal, or rename in the exported substrate surface now fails with an
audit-specific message unless the snapshot is intentionally updated.

It also compiles the expected re-export paths:

- `CuConfig`
- `CuStepRunner`
- `E2bReapReport`
- `E2bSandboxInfo`
- `CuError`
- `ComputerUseExecutor`
- `CuOutcome`
- `RunRequest`
- `RunResult`
- `Profile`
- `ProfileRegistry`

## Offline Contracts Covered

The new contract and existing test suite cover the offline boundary:

- Anthropic response parsing: existing unit tests cover tool calls, multi-action
  turns, stop signals, malformed responses, unknown tool names, token charging,
  and tool-variant request shape.
- Mock harness loop: new contract exercises parsed action shapes for
  `double_click`, `scroll`, and `navigate` through `MockAnthropic` and
  `MockE2bClient`, including inline screenshot reuse.
- Config: `CuConfig::for_tests` is verified to supply placeholder credentials,
  local/test URLs, timeout, and the current default tool variant without reading
  real credentials.
- Budget: `BudgetTracker` is verified for profile caps plus USD-derived token
  caps and exhaustion reasons.
- Pool: `SandboxPool` release and empty reaper paths are verified without
  calling `acquire`, so no sandbox is opened.
- E2B reaper trait: `MockE2bClient::reap_idle` verifies stale sandbox selection
  and close reporting against fake sandbox rows.
- Restart: `RestartCoordinator`, snapshot cadence, retry budget, and
  `restore_from` are verified as pure data-path contracts.

## Production Caller Grep

Static grep scope matches PR #2171: `crates/argo-cli/src/`,
`crates/argo-agent-runner/src/`, and `crates/argo-orchestrator/src/`.

Observed callers for `argo_executor_computer_use::`:

- `crates/argo-cli/src/handlers/action_replay_executors.rs`
- `crates/argo-cli/src/handlers/cu_replay.rs`
- `crates/argo-cli/src/handlers/cu_replay_tests.rs`
- `crates/argo-cli/src/handlers/cu_screenshots.rs`
- `crates/argo-cli/src/handlers/sandbox_e2b.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_linux.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs`

That is byte-for-byte the PR #2171 allowlist. No
`argo-agent-runner` or `argo-orchestrator` source caller was observed.

## What Works Offline Today

- The shared CU loop can be driven deterministically by fake Anthropic and fake
  E2B clients.
- Parser, action-shape, audit-row, budget, inline-screenshot, restart, and
  reaper behavior are covered without credentials.
- The E2B client and pool can be constructed offline; pure pool/reaper paths can
  be tested without provisioning sandboxes.
- CLI caller drift is guarded by both the existing `argo-sandbox` PR #2171 pin
  and the new computer-use substrate contract.

## What Remains Deferred

- E2B is not the v1.0 demo path for Claim 5. Per `CLAUDE.md`, macOS
  computer-use is the only v1-hardened computer-use route; E2B, Linux, and
  Windows remain DEFERRED-v1.x.
- The allowlisted CLI surfaces can construct and run the E2B executor when
  credentials are present, but this audit does not promote that path to v1.0
  hardened status.
- No live E2B sandbox lifecycle, browser-template behavior, external network
  reliability, or operator credential posture was validated here.
- E2B tenancy remains API-key scoped for the reaper/admin path, as documented in
  `src/e2b_reap.rs` and `crates/argo-cli/src/handlers/sandbox_e2b.rs`.

## Validation

Command:

```bash
CARGO_TARGET_DIR=/tmp/eval-e2b-executor-substrate-status-target cargo test -p argo-executor-computer-use --tests
```

Result:

- `src/lib.rs` unit tests: 96 passed
- `tests/cu_harness_offline.rs`: 7 passed
- `tests/cu_smoke.rs`: 1 passed
- `tests/restart_smoke.rs`: 1 passed
- `tests/substrate_status_contract.rs`: 5 passed

Total: 110 passed, 0 failed.

## Gaps and Follow-ups

No real gap found in this audit. No follow-up brief proposed.
