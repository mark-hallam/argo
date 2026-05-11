# Stream G1 — Native Windows CU executor dispatch wiring (mirror of macOS #955)

**Owner**: claude. **Depends on**: `mission-cu-native-windows-poc` MERGED (the Windows POC crate from Track B).
**Authored against**: origin/main `602af858` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh G1-cu-native-windows-executor-dispatch`.
**Tier tag**: PRODUCT_ROADMAP.md Track G brief 1 of 3 — closes mission claim 5 (third leg native CU).

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Executor selection | New `ExecutorKind::ComputerUseWindows` variant; manifest declares which OS the action wants | Same. Per-tenant policy that gates which executors a worker may use (e.g. high-security tenants disable browser CU) | `mission-enterprise-executor-acl` |
| Cross-platform compile | All three native CU crates compile clean on all 3 OSes (runtime probe = Unsupported off-platform) | Same | n/a |
| Fallback when probe = Unsupported | Action fails with a `Capability` error; operator visible | Same; v1 ships this | n/a |
| Audit attribution | Each CU action audit-logged with the executor variant + host OS | Same | n/a |

v1 is acceptable: this is integration wiring with no
new security surface; the POC crate already passed
its own brief's safety review.

## Why

Mission claim 5 ("real-world execution = any task a
human can do on a PC/Mac") requires native CU on
macOS (shipped #941 + dispatch #955), Linux (POC
shipped via `claude-cu-native-linux-poc`; dispatch
pending), and Windows (POC newly authored via
`mission-cu-native-windows-poc`; dispatch is THIS
brief).

Without dispatch wiring, the Windows POC crate is
unreachable — workers can't actually call the
Windows native CU executor. This brief closes that
last mile.

## Scope

Mirror `mission-cu-native-macos-executor-dispatch` #955
exactly, swapping `MacosComputerUse` for
`WindowsComputerUse`:

- New `ExecutorKind::ComputerUseWindows` variant in
  `argo-vertical`.
- New adapter file
  `argo-runtime/src/handlers/worker_actions_executor_cu_windows.rs`
  registering the dispatch path.
- Manifest support: `[actions.<id>] executor =
  "computer-use-windows"` resolves to the new
  variant.
- Runtime probe on action dispatch: if
  `WindowsComputerUse::probe()` returns
  `Unsupported`, action errors with
  `Capability::UnsupportedExecutor` (operator-
  surfaced error type).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Shared hotspots:
  - `crates/argo-vertical/src/manifest.rs`
    (executor enum — narrowly extend)
  - `crates/argo-runtime/src/handlers/mod.rs`
    (register new adapter module)

## Files you may touch

- `crates/argo-vertical/src/manifest.rs` — narrowly
  scoped: add `ComputerUseWindows` variant to
  `ExecutorKind`; add `"computer-use-windows"`
  string mapping. No other edits. ≤ 280 LOC after.
- `crates/argo-runtime/src/handlers/worker_actions_executor_cu_windows.rs`
  — NEW. ≤ 300 LOC. Mirrors the macOS adapter
  file exactly.
- `crates/argo-runtime/src/handlers/mod.rs` —
  narrowly scoped: 1-line `pub mod` addition.
- `crates/argo-runtime/src/handlers/worker_actions_dispatch.rs`
  — narrowly scoped: 1 arm added to the
  match-on-ExecutorKind. ≤ 250 LOC after.
- `crates/argo-runtime/Cargo.toml` — narrowly
  scoped: 1 dep addition for
  `argo-executor-computer-use-windows`.
- `crates/argo-runtime/src/handlers/worker_actions_executor_cu_windows_tests.rs`
  — NEW. ≥ 5 tests (probe-fail / dispatch-routes-
  to-windows / unsupported-error-shape / config-
  parse / smoke-on-windows-only `#[cfg]`-gated).
  ≤ 250 LOC.

## Files you must NOT touch

- `argo-executor-computer-use-windows` (the POC
  crate from Track B) — locked from that brief.
- `argo-executor-computer-use-macos`,
  `argo-executor-computer-use-linux`,
  `argo-executor-computer-use` (E2B) — locked.
- Other adapter files.

## Acceptance criteria

- [ ] New `ExecutorKind::ComputerUseWindows`
      variant exists; string parser accepts
      `"computer-use-windows"`.
- [ ] Manifest `[actions.<id>] executor =
      "computer-use-windows"` resolves correctly
      in `argo vertical doctor`.
- [ ] Dispatch from `worker_actions_dispatch.rs`
      routes Windows CU actions to the new
      adapter.
- [ ] Probe on Windows where
      `argo-executor-computer-use-windows` reports
      `Supported`: action dispatches and the
      Windows executor's `session()` is invoked.
- [ ] Probe on non-Windows (macOS / Linux) →
      action fails with
      `Capability::UnsupportedExecutor` carrying
      a clear operator message: `"Action requires
      Windows native computer-use; current host is
      ${os}. Reassign to a Windows worker or use a
      different executor."`
- [ ] 5 unit tests pass.
- [ ] `cargo build --workspace` clean.
- [ ] `cargo clippy --workspace --all-targets -- -D
      warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-G1-cu-native-windows-executor-dispatch`
- PR title: `executor(cu): native Windows dispatch wiring (G1)`

## Notes for the implementer

- **Mirror #955 file-by-file.** The macOS adapter
  shipped `worker_actions_executor_cu_macos.rs`
  with a small dispatch function + probe-then-
  invoke logic. Read that file end-to-end before
  writing this one; symmetry pays off in test
  parity.

- **Probe contract.** Each native CU crate exports
  a `probe() -> ProbeResult { Supported,
  Unsupported(reason) }`. Call once per dispatch;
  on `Unsupported`, return early with the
  Capability error.

- **Error message uniformity.** Use the same
  message format across the 3 native CU executors
  (macOS / Linux / Windows). The error type stays
  the existing `argo-error::Capability` variant; no
  new error type.

- **`#[cfg(target_os = "windows")]`-gated tests.**
  The integration smoke (actually-invokes-the-
  executor) runs only on Windows CI runners.
  Probe-fail + dispatch-routes tests run on all
  OSes.

- **Don't add a feature flag for this.** Dispatch
  wiring should be unconditionally compiled; the
  probe handles runtime gating. Feature flags
  would mean operators need different builds per
  OS, which we explicitly DON'T want.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-executor-acl` — per-tenant
  policy gating which executors a worker may use.
