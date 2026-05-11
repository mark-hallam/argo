# Mission load-bearing wire — Linux native CU executor dispatch

**Owner**: claude. **Depends on**: `mission-cu-native-macos-executor-dispatch` MERGED #955 (canonical pattern); `claude-cu-native-linux-poc` MERGED #992 (executor crate `crates/argo-executor-computer-use-linux/` exists with `xdotool`+`scrot` X11 primitives).
**Authored against**: origin/main `09673df7` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-native-linux-executor-dispatch`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #100 ("Build native Linux computer-use executor") — executor crate already SHIPPED #992; the dispatch wiring (parallel to macOS dispatch #955) is the open follow-up. Mission claim 5 ("real-world execution") needs Linux dispatch parity so verticals running on Linux hosts can reach the native CU loop without falling through to the E2B sandbox path.

## Scope

Mirror `mission-cu-native-macos-executor-dispatch` for Linux:

1. Add `ExecutorKind::ComputerUseLinux` variant + `"computer_use_linux"` / `"computer-use-linux"` parse aliases + display string in `crates/argo-cli/src/handlers/worker_actions_executor.rs` (under HARD RULE 5 exhaustive-match allowance).
2. Add `resolve_executor` dispatch arm that on Linux constructs `LinuxCuExecutor::new()` + preflight, then HumanHandoff with the bundle (mirror of macOS thin-adapter pattern); on non-Linux returns clean `ArgoError::Provider` with hint.
3. NEW adapter `crates/argo-cli/src/handlers/worker_actions_executor_cu_linux.rs` (≤ 200 LOC) — adapter struct + platform-gated `#[cfg(target_os = "linux")]` block calling into `argo_executor_computer_use_linux`.
4. NEW sibling tests file `crates/argo-cli/src/handlers/worker_actions_executor_cu_linux_tests.rs` — 4 unit tests + 1 cfg-gated non-Linux unit test mirroring the macOS pattern.
5. NEW gated smoke `crates/argo-cli/tests/cu_linux_smoke.rs` (≤ 80 LOC) — `#[cfg(target_os = "linux")]` + gated by `ARGO_LINUX_CU_TEST=1` (requires xdotool, scrot, DISPLAY).

This is NOT in scope: full `run_loop` orchestrator port (deferred to a follow-up brief, identical to the macOS dispatch pattern that landed HumanHandoff in v1); Wayland support (`scrot` is X11-only — Wayland gap noted in #992 stays open); macOS or Windows dispatch wiring; `ExecutorKind` enum reordering or renaming.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor.rs` — narrowly scoped: ADD `ComputerUseLinux` variant + parse arms + display string + ONE `resolve_executor` arm
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_linux.rs` (NEW, ≤ 200 LOC) — Linux adapter mirroring `worker_actions_executor_cu_macos.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_linux_tests.rs` (NEW, sibling, ≤ 200 LOC) — 4 + 1 unit tests
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: ONE `pub mod worker_actions_executor_cu_linux;` line
- `crates/argo-cli/tests/cu_linux_smoke.rs` (NEW, ≤ 80 LOC) — cfg-gated + env-gated smoke
- `crates/argo-cli/Cargo.toml` — narrowly scoped: ADD `argo-executor-computer-use-linux` as a `target.'cfg(target_os = "linux")'.dependencies` entry (mirror macOS dep cfg)

## Files you must NOT touch

- `crates/argo-executor-computer-use-linux/**` — executor crate is stable; reuse public API only
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` — sibling adapter stable
- Any other crate
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `ExecutorKind::ComputerUseLinux` parses from `"computer_use_linux"` and `"computer-use-linux"`
- [ ] `ExecutorKind::ComputerUseLinux` display string is `"computer_use_linux"`
- [ ] `resolve_executor` arm: on Linux runs `LinuxCuExecutor::new + preflight` then returns HumanHandoff with permission summary in the bundle; on non-Linux returns `ArgoError::Provider` with hint to "use computer_use sandbox executor on non-Linux hosts"
- [ ] 4 unit tests: parse-canonical, parse-alias, display-roundtrip, cfg-gated non-Linux returns Provider error
- [ ] 1 cfg-gated smoke `cu_linux_smoke.rs` (skipped on non-Linux, skipped without `ARGO_LINUX_CU_TEST=1` env var)
- [ ] No `unwrap`/`expect` outside `#[cfg(test)]`
- [ ] `cargo build -p argo-cli` clean on Linux and macOS (host) and Windows (cfg-gated branches compile out)
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` passes (gated smoke skipped without env)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-native-linux-executor-dispatch`
- PR title: `cli: dispatch ExecutorKind::ComputerUseLinux into native Linux executor (mission-cu-native-linux-executor-dispatch)`

## Notes for the implementer

- This brief is the exact peer of #955 (macOS dispatch). Open `crates/argo-cli/src/handlers/worker_actions_executor.rs` and find the `ExecutorKind::ComputerUseMacOS` arms — copy each section and substitute "Linux" / `linux` / `target_os = "linux"`.
- HARD RULE 5 exhaustive-match allowance covers the single `resolve_executor` match site. Other `ExecutorKind::ComputerUse*` usages are variant constructors, not match arms — no extra files needed.
- The `argo-executor-computer-use-linux` crate exposes `LinuxCuExecutor::new() + preflight() + handoff()` mirroring the macOS step primitives. Like the macOS adapter, this v1 lands as a HumanHandoff path; full `run_loop` integration follows in `mission-cu-native-linux-run-loop` (separate brief, after this lands).
- Wayland gap: documented in #992. `scrot` is X11-only; on Wayland the preflight will fail clean with "X11 required" — leave that gap; not in scope here.
- Pattern verified on `crates/argo-cli/src/handlers/worker_actions_executor.rs:69,102,110,125,228` (current macOS arms) — copy each.
