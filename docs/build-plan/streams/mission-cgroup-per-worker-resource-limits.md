# Stream mission-cgroup-per-worker-resource-limits — cgroup-v2 per-worker memory + CPU caps (Linux)

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `173e24dc` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cgroup-per-worker-resource-limits`.

> SOURCE: MISSION_BACKLOG.md Section 8 #236 ("Per-worker resource limits (cgroup, memory cap, CPU share)"). Today a runaway worker can OOM the host, starve other tenants' workers, or burn unbounded CPU. A per-worker cgroup-v2 slice with `memory.max` + `cpu.weight` confines the blast radius to the offending worker.

## Scope

Add a `WorkerResourceCage` that, on Linux with cgroup-v2 mounted, creates a per-worker cgroup slice under `/sys/fs/cgroup/argo-worker-<worker_id>/` and writes:

- `memory.max` ← `WorkerResourceConfig::memory_max_mib * 1MiB` (default 1024 MiB)
- `cpu.weight` ← `WorkerResourceConfig::cpu_weight` (default 100, range 1-10000)
- Moves the worker subprocess pid into `cgroup.procs`

On non-Linux or no-cgroup-v2 hosts, `WorkerResourceCage::install_on(pid)` returns `Ok(NotSupported)` and the runtime continues without confinement — operators who need confinement on macOS dev hosts use Docker / Lima as documented in the runbook addition.

The OOM-restart handler from `mission-oom-restart-handler` (already MERGED) detects when `memory.events::oom_kill > 0` and emits an audit row + restart. This brief is the *upstream* of that: it sets the cap that triggers the OOM. Together they form the confinement loop.

**Not in scope**: per-tenant aggregate limits (separate brief — `mission-tenant-resource-aggregate`); IO throttling (`io.max`); CPU pinning (`cpuset.cpus`); cgroup-v1 backport (cgroup-v2 only); Windows/macOS native confinement.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `<NEW: crates/argo-runtime/src/worker_resource_cage.rs>` (≤400 LOC) — `WorkerResourceCage` with `install_on(pid, config) -> CageStatus` and `uninstall(worker_id)`. CageStatus: `Active { cgroup_path }` / `NotSupported { reason }` / `Failed { reason }`.
- `<NEW: crates/argo-runtime/src/worker_resource_cage_tests.rs>` (≤200 LOC) — unit tests for the path-construction + config-parsing layers (the actual cgroup writes are Linux-only and gated behind `#[cfg(target_os = "linux")]` integration tests).
- `crates/argo-runtime/src/lib.rs` — narrowly scoped: add ONE `pub mod worker_resource_cage;` line + cfg-test mod line. Re-export `WorkerResourceCage, WorkerResourceConfig, CageStatus`.
- `crates/argo-config/src/sections_context.rs` — narrowly scoped: add `pub struct WorkerResourceConfig { pub enabled: bool, pub memory_max_mib: u64, pub cpu_weight: u32 }` with defaults `enabled=true, memory_max_mib=1024, cpu_weight=100`. Add ONE field on RuntimeConfig.
- `<NEW: docs/skycot/runbook/worker-resource-cage.md>` (≤80 LOC) — operator runbook covering: Linux requirements (cgroup-v2 mounted at /sys/fs/cgroup, argo process has write access OR `systemd-run --user --scope` wrapper), how to view current caps (`cat /sys/fs/cgroup/argo-worker-<id>/memory.current`), how to disable per-tenant or globally, and the recommended Docker/Lima setup for macOS dev hosts.
- `Cargo.toml` — NO new dependencies. Use `std::fs` for cgroup file IO.

## Files you must NOT touch

- `crates/argo-runtime/src/oom_restart.rs` — the downstream consumer; locked from `mission-oom-restart-handler`. This brief only sets the cap that triggers the OOM kill it watches for.
- Any worker-spawn site in `argo-orchestrator` — install_on() will be called from a follow-up brief (`mission-worker-cage-wire-spawn`) once this primitive is reviewable in isolation.
- Cgroup-v1 paths or `prlimit`-based fallbacks.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] `WorkerResourceCage::install_on(pid, config)` returns:
  - `Active { cgroup_path }` on Linux with cgroup-v2 + write access, after writing `memory.max` + `cpu.weight` + `cgroup.procs`.
  - `NotSupported { reason }` on macOS, on Linux with cgroup-v1, or when `/sys/fs/cgroup` isn't writable.
  - `Failed { reason }` only on unexpected IO errors.
- [ ] Slice name format: `argo-worker-<worker_id>` (where `worker_id` is the typed `Uuid`-shaped string).
- [ ] `WorkerResourceCage::uninstall(worker_id)` removes the cgroup directory; tolerated when already gone (idempotent).
- [ ] Unit tests cover: path-construction, config-default values, TOML parse, `NotSupported` reason strings.
- [ ] `#[cfg(target_os = "linux")] #[ignore]` integration test under `tests/cage_smoke.rs` that fork-exec's `/bin/sleep 5`, installs a 16 MiB cap, asserts the pid is in `cgroup.procs`, then waits for OOM kill (run with `cargo test --ignored` on a CI runner that supports cgroup-v2).
- [ ] Disabled-config path returns `NotSupported { reason: "disabled by config" }` on any host.
- [ ] No `unsafe` Rust outside `#[cfg(unix)]`.
- [ ] `cargo clippy -p argo-runtime -p argo-config --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-runtime` passes (default test set; ignored ones gated to Linux CI).
- [ ] Every modified file < 500 LOC.
- [ ] Runbook entry covers the macOS dev-host story explicitly.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-cgroup-per-worker-resource-limits`
- PR title: `runtime: cgroup-v2 per-worker memory + CPU caps (mission-cgroup-per-worker-resource-limits)`

## Notes for the implementer

- Cgroup-v2 file writes are plain ASCII (`memory.max\n` → `1073741824\n`). No special framing.
- The `/sys/fs/cgroup/` mount point is conventional; let operators override via `WorkerResourceConfig::cgroup_root` if you want, but `/sys/fs/cgroup` default is fine for v1.
- `cgroup.procs` accepts one pid per line, atomic — write the pid, flush, done.
- For the `Active` path, the cgroup dir must be created first (`std::fs::create_dir`) and the controllers enabled via the parent's `cgroup.subtree_control` (e.g. `+memory +cpu`). Document the systemd-managed-cgroup case in the runbook (operators on systemd-host setups should use `systemd-run --user --scope` to get a writable slice).
- macOS dev hosts: `NotSupported { reason: "cgroup-v2 requires Linux" }`. Runbook points operators at Docker / Lima.
- This is a primitive PR. Wire-up to the actual worker-spawn path is a follow-up brief (`mission-worker-cage-wire-spawn`).
- Per CLAUDE.md "No `unsafe` Rust outside marked FFI shims" — `std::fs` writes are safe; no `unsafe` needed.
- Per CLAUDE.md "No global mutable state" — `WorkerResourceCage` is per-worker; instantiate via AgentDeps in the follow-up wire-up brief.
