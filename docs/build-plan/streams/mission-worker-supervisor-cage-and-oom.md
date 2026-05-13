# mission-worker-supervisor-cage-and-oom — wire WorkerResourceCage + OomRestartCoordinator into a worker supervisor

**Owner**: claude. **Depends on**: `argo_runtime::worker_resource_cage` (shipped, #1541); `argo_runtime::oom_restart` (shipped).
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> **Phase 3 wire-or-delete decision (CEO-authorized 2026-05-14): WIRE.** Audit cluster 3 O-2/O-4/O-5/O-6 found 4 worker-supervisor primitives (Pool elasticity, WorkerResourceCage, OOM restart, throttle) ALL un-wired. They share the same parent gap: no production supervisor module spawns/drains/restarts workers. This brief lands the supervisor.

## Scope

Create `crates/argo-orchestrator/src/worker_supervisor.rs` (≤400 LOC) — the production supervisor that:
1. Spawns workers via fork-exec subprocesses.
2. Calls `WorkerResourceCage::install_on(pid, config)` immediately after spawn (cgroup-v2 caps on Linux; NotSupported on macOS).
3. Watches `cgroup.events::oom_kill` via inotify; on event, calls `OomRestartCoordinator::decide_restart`.
4. Maintains a `Map<WorkerId, ChildHandle>` keyed by tenant_id.

NOT in this brief: pool elasticity (sized differently; future `mission-worker-supervisor-elasticity` brief); the throttle registry wire-up (separate brief `mission-throttle-registry-wire-dispatch`).

## Files you may touch
- `<NEW: crates/argo-orchestrator/src/worker_supervisor.rs>` (≤400 LOC).
- `<NEW: crates/argo-orchestrator/src/worker_supervisor_tests.rs>` (≤250 LOC).
- `crates/argo-orchestrator/src/lib.rs` — narrowly: `pub mod worker_supervisor;` + re-exports.
- `crates/argo-cli/src/handlers/gateway/boot_spawns.rs` — narrowly: construct + spawn the supervisor at gateway boot. ≤30 LOC.

## Acceptance
- [ ] Worker spawned via supervisor gets cgroup-v2 caps applied on Linux.
- [ ] OOM kill triggers `OomRestartCoordinator::decide_restart`; restart-budget enforced.
- [ ] macOS path: `NotSupported`; workers run unconfined but supervisor stays alive.
- [ ] Workspace clippy clean. Linux-only integration smoke under #[ignore].

## Branch
- `claude-mission-worker-supervisor-cage-and-oom`
