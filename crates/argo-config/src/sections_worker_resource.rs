//! `[runtime.worker_resource_cage]` — per-worker cgroup-v2 memory +
//! CPU caps (`mission-cgroup-per-worker-resource-limits`,
//! MISSION_BACKLOG §8 #236).
//!
//! Sibling of [`crate::sections_context`]; the parent module re-exports
//! these symbols so external imports stay `argo_config::WorkerResourceConfig`.
use serde::{Deserialize, Serialize};

/// Default memory cap (MiB) for the per-worker cgroup-v2 cage.
/// Mirrors `argo_runtime::worker_resource_cage::DEFAULT_MEMORY_MAX_MIB`.
pub const DEFAULT_WORKER_RESOURCE_MEMORY_MAX_MIB: u64 = 1024;
/// Default `cpu.weight` for the per-worker cgroup-v2 cage. Cgroup-v2
/// neutral weight; mirrors
/// `argo_runtime::worker_resource_cage::DEFAULT_CPU_WEIGHT`.
pub const DEFAULT_WORKER_RESOURCE_CPU_WEIGHT: u32 = 100;

/// `[runtime.worker_resource_cage]` — per-worker cgroup-v2 memory +
/// CPU caps (`mission-cgroup-per-worker-resource-limits`,
/// MISSION_BACKLOG §8 #236). Mirrors the in-memory shape in
/// `argo_runtime::worker_resource_cage::WorkerResourceConfig`; the
/// orchestrator maps this struct to the runtime one at worker-spawn
/// time. Split here (rather than reusing the runtime type directly)
/// keeps `argo-config` free of a build-time dependency on
/// `argo-runtime`.
///
/// Defaults to enabled with a 1 GiB memory cap + neutral
/// `cpu.weight=100`. On non-Linux hosts the runtime returns
/// `CageStatus::NotSupported` regardless of these values, so
/// default-enabled is safe for macOS dev workflows; the runbook
/// (`docs/skycot/runbook/worker-resource-cage.md`) covers
/// Docker/Lima for operators who need confinement off-Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkerResourceConfig {
    /// Master enable. When `false`, the runtime returns
    /// `NotSupported { reason: "disabled by config" }` on any host
    /// and the worker runs unconfined.
    pub enabled: bool,
    /// Hard memory cap per worker, in MiB. Written into
    /// `memory.max`; breach triggers a kernel OOM kill that the
    /// `oom_restart` handler downstream observes.
    pub memory_max_mib: u64,
    /// CPU weight per worker. cgroup-v2 range is `1..=10_000`; the
    /// runtime clamps out-of-range values. `100` is the neutral
    /// weight.
    pub cpu_weight: u32,
}

impl Default for WorkerResourceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_max_mib: DEFAULT_WORKER_RESOURCE_MEMORY_MAX_MIB,
            cpu_weight: DEFAULT_WORKER_RESOURCE_CPU_WEIGHT,
        }
    }
}
