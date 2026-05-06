# V1.2 — Code execution executor

**Owner**: claude. **Depends on**: A-executor-trait merged.

## Scope

New `ExecutorKind::CodeExecution` for Python / JavaScript
code execution in a sandbox. Mission: "any task a human can
do on a PC/Mac"; many tasks (data wrangling, ad-hoc API
scripts, computation) are most cleanly expressed as code
rather than browser interactions or specific MCP calls.

Sandboxing rules:
- Per-action E2B Firecracker microVM (reuse the existing
  `argo-sandbox` integration from CU executor)
- Languages: Python 3.12 + Node 20; auto-detect from
  manifest's `executor.language` field
- Time limit: 60s default, env-overridable
  `ARGO_CODE_EXEC_TIMEOUT_S`
- Memory limit: 1 GiB default
- No network egress unless action manifest declares
  `executor.allow_network = true`
- `executor.allow_pkg_install` opt-in (default false)

I/O contract:
- Action input written to `/tmp/input.json` in sandbox
- Action expected output at `/tmp/output.json`
- Stdout + stderr captured + included in result
- Files dropped at `/tmp/artifacts/` exfiltrated to a
  per-worker S3-like store (or the existing fs_root if
  filesystem executor lands first)

Approval-gate: write/network/pkg-install variants always
gated; pure compute (no network, no pkg install) honors the
existing AutoApprovalPolicy.

## Files you may touch
- EXHAUSTIVE-MATCH amendment applies (adding
  `ExecutorKind::CodeExecution` variant)
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` —
  narrowly: add `CodeExecution` arm to ExecutorKind +
  parse_kind logic
- `crates/argo-cli/src/handlers/worker_actions_executor_code.rs`
  (NEW — the executor implementation, target ~400 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor_code_tests.rs`
  (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 2 new
  `pub mod` lines
- `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs` —
  narrowly: matching stub for offline-tests

## Acceptance criteria
- [ ] Python + Node both runnable
- [ ] Sandbox honors time + memory limits
- [ ] No network egress unless declared
- [ ] No pkg install unless declared
- [ ] Stdout/stderr captured into result
- [ ] Artifacts at `/tmp/artifacts/` collected
- [ ] Approval-gate marks correct per opt-in
- [ ] 10 unit tests (offline w/ stub) + 2 integration tests
      gated on `ARGO_E2B_TOKEN`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-code-execution-executor`
- PR title: `runtime: sandboxed code-execution executor (Python/Node) (claude-code-execution-executor)`
