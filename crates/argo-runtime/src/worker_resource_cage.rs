//! Per-worker cgroup-v2 resource cage
//! (`mission-cgroup-per-worker-resource-limits`, MISSION_BACKLOG §8
//! #236 — Tier P reliability).
//!
//! Today a runaway worker can OOM the host, starve other tenants'
//! workers, or burn unbounded CPU. This module defines the
//! supervisor-level primitive that confines the blast radius to a
//! single worker: a cgroup-v2 slice under
//! `/sys/fs/cgroup/argo-worker-<worker_id>/` with `memory.max` and
//! `cpu.weight` written, and the worker subprocess pid moved into
//! `cgroup.procs`.
//!
//! ## Platform matrix
//!
//! | Platform | Backend             | [`install_on`] outcome             |
//! | -------- | ------------------- | ---------------------------------- |
//! | Linux v2 | cgroup-v2 unified   | [`CageStatus::Active`] on success  |
//! | Linux v1 | legacy cgroup-v1    | [`CageStatus::NotSupported`]       |
//! | macOS    | n/a                 | [`CageStatus::NotSupported`]       |
//! | Windows  | n/a                 | [`CageStatus::NotSupported`]       |
//!
//! macOS dev hosts use Docker / Lima for confinement — see
//! `docs/skycot/runbook/worker-resource-cage.md`.
//!
//! ## Composition with the OOM-restart handler
//!
//! [`crate::oom_restart`] watches `memory.events::oom_kill > 0` and
//! decides whether to respawn the worker. This module is the
//! *upstream* of that: it sets the cap that triggers the OOM kill
//! the restart handler observes. Together they form the
//! confinement-then-recovery loop.
//!
//! ## Wiring status
//!
//! This is the primitive PR. The actual call-site that invokes
//! [`WorkerResourceCage::install_on`] on every worker spawn lives in
//! a follow-up brief (`mission-worker-cage-wire-spawn`) so the
//! primitive can be reviewed in isolation.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Conventional cgroup-v2 mount point on systemd Linux.
pub const DEFAULT_CGROUP_ROOT: &str = "/sys/fs/cgroup";
/// Default memory cap (MiB) per worker.
pub const DEFAULT_MEMORY_MAX_MIB: u64 = 1024;
/// Default `cpu.weight` per worker (cgroup-v2 default is 100; the
/// range is `1..=10_000`).
pub const DEFAULT_CPU_WEIGHT: u32 = 100;
/// Minimum legal `cpu.weight` value per the cgroup-v2 ABI.
pub const MIN_CPU_WEIGHT: u32 = 1;
/// Maximum legal `cpu.weight` value per the cgroup-v2 ABI.
pub const MAX_CPU_WEIGHT: u32 = 10_000;
/// Bytes per MiB — `memory.max` accepts a byte count.
pub const MIB_BYTES: u64 = 1024 * 1024;
/// Slice-name prefix used under [`DEFAULT_CGROUP_ROOT`].
pub const SLICE_PREFIX: &str = "argo-worker-";

/// Per-worker cgroup-v2 resource limits. Mirrors the TOML shape in
/// `argo_config::WorkerResourceConfig` so operators can override the
/// runtime defaults from `argo.toml`.
///
/// Defaults are intentionally conservative-but-not-restrictive:
/// 1 GiB memory + cgroup-v2's neutral cpu.weight of 100. Operators
/// running heavier verticals (browser-based computer-use, large
/// document executors) should raise `memory_max_mib`; operators with
/// soft-multitenant SLAs should lower `cpu_weight` on low-priority
/// worker pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkerResourceConfig {
    /// Master enable. When `false`, [`WorkerResourceCage::install_on`]
    /// returns `NotSupported { reason: "disabled by config" }` on any
    /// host, so the wire stays zero-overhead until operators opt in.
    pub enabled: bool,
    /// Hard memory cap (MiB). Written into `memory.max` as
    /// `memory_max_mib * 1024 * 1024` bytes. Breach triggers a kernel
    /// OOM kill, which [`crate::oom_restart`] catches downstream.
    pub memory_max_mib: u64,
    /// CPU weight written into `cpu.weight`. Range is `1..=10_000`;
    /// the cage clamps out-of-range values rather than rejecting them.
    /// `100` is cgroup-v2's neutral weight — all workers compete
    /// equally under load.
    pub cpu_weight: u32,
}

impl Default for WorkerResourceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_max_mib: DEFAULT_MEMORY_MAX_MIB,
            cpu_weight: DEFAULT_CPU_WEIGHT,
        }
    }
}

/// Outcome of [`WorkerResourceCage::install_on`]. Separating
/// `NotSupported` from `Failed` lets the supervisor distinguish
/// "platform doesn't support confinement, run unconfined" from "we
/// hit an unexpected IO error, audit it".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CageStatus {
    /// Cgroup slice created and pid moved in. `cgroup_path` is the
    /// absolute path of the slice (suitable for passing to
    /// [`crate::oom_restart::classify_exit`]'s `cgroup_path`
    /// parameter on worker death).
    Active {
        /// Absolute path of the created cgroup slice.
        cgroup_path: String,
    },
    /// Platform does not support cgroup-v2 confinement (macOS,
    /// Windows, cgroup-v1 hosts, disabled-by-config). The worker
    /// runs unconfined; the operator wanted this implicitly or
    /// chose Docker / Lima for confinement on dev hosts.
    NotSupported {
        /// Human-readable diagnostic — propagated into supervisor
        /// logs so an operator can tell *which* fallback path
        /// applied without re-deriving it from the host.
        reason: String,
    },
    /// Unexpected IO error during cgroup setup (e.g. the
    /// `cgroup.procs` write failed even though the slice was
    /// created). Supervisor should log + audit, and may choose to
    /// continue with the worker unconfined or abort the spawn.
    Failed {
        /// Source error chain rendered as a string.
        reason: String,
    },
}

/// Per-worker cgroup-v2 cage. Cheap to construct — the struct holds
/// only the configured cgroup root path. Operators on systemd-managed
/// cgroup hosts override the root via [`Self::with_root`] (e.g. to
/// nest under `/sys/fs/cgroup/argo.slice/`).
#[derive(Debug, Clone)]
pub struct WorkerResourceCage {
    cgroup_root: PathBuf,
}

impl Default for WorkerResourceCage {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerResourceCage {
    /// Build a cage using the conventional [`DEFAULT_CGROUP_ROOT`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            cgroup_root: PathBuf::from(DEFAULT_CGROUP_ROOT),
        }
    }

    /// Build a cage rooted at a custom path. Used by the integration
    /// test (root set to a `tempdir`) and by operators on systemd-
    /// managed cgroup hosts.
    #[must_use]
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            cgroup_root: root.into(),
        }
    }

    /// Borrow the configured cgroup root.
    #[must_use]
    pub fn cgroup_root(&self) -> &Path {
        &self.cgroup_root
    }

    /// Compose the absolute slice path for `worker_id`. Pure
    /// path-construction — does not touch the filesystem. Surfaces
    /// so callers can pass the same string to
    /// [`crate::oom_restart::classify_exit`]'s `cgroup_path` param
    /// without re-deriving the slice name.
    ///
    /// ```
    /// use argo_runtime::worker_resource_cage::WorkerResourceCage;
    /// let cage = WorkerResourceCage::with_root("/sys/fs/cgroup");
    /// let path = cage.slice_path("wkr-abc");
    /// assert!(path.ends_with("argo-worker-wkr-abc"));
    /// ```
    #[must_use]
    pub fn slice_path(&self, worker_id: &str) -> PathBuf {
        self.cgroup_root.join(format!("{SLICE_PREFIX}{worker_id}"))
    }

    /// Create the slice, write `memory.max` + `cpu.weight`, and
    /// move `pid` into `cgroup.procs`.
    ///
    /// Returns [`CageStatus::NotSupported`] without side-effects on
    /// macOS, Windows, cgroup-v1 hosts, and when the master `enabled`
    /// flag is `false`. Returns [`CageStatus::Failed`] only on
    /// unexpected IO errors after the platform check passed.
    pub fn install_on(
        &self,
        worker_id: &str,
        pid: u32,
        config: &WorkerResourceConfig,
    ) -> CageStatus {
        if !config.enabled {
            return CageStatus::NotSupported {
                reason: "disabled by config".to_string(),
            };
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (worker_id, pid);
            CageStatus::NotSupported {
                reason: "cgroup-v2 requires Linux".to_string(),
            }
        }
        #[cfg(target_os = "linux")]
        {
            self.install_on_linux(worker_id, pid, config)
        }
    }

    #[cfg(target_os = "linux")]
    fn install_on_linux(
        &self,
        worker_id: &str,
        pid: u32,
        config: &WorkerResourceConfig,
    ) -> CageStatus {
        // cgroup-v2 mounts always carry a top-level `cgroup.controllers`
        // file. Cgroup-v1 hosts have `cpu/`, `memory/` subdirs instead.
        let controllers = self.cgroup_root.join("cgroup.controllers");
        if !controllers.exists() {
            return CageStatus::NotSupported {
                reason: format!(
                    "cgroup-v2 not mounted at {} (no cgroup.controllers file)",
                    self.cgroup_root.display()
                ),
            };
        }
        let slice = self.slice_path(worker_id);
        if let Err(e) = fs::create_dir_all(&slice) {
            if e.kind() == io::ErrorKind::PermissionDenied {
                return CageStatus::NotSupported {
                    reason: format!(
                        "permission denied creating {} (run under systemd-run --user --scope?)",
                        slice.display()
                    ),
                };
            }
            return CageStatus::Failed {
                reason: format!("create_dir_all({}): {e}", slice.display()),
            };
        }
        let memory_bytes = config.memory_max_mib.saturating_mul(MIB_BYTES);
        if let Err(e) = fs::write(slice.join("memory.max"), format!("{memory_bytes}\n")) {
            return cage_write_failure("memory.max", &e, &slice);
        }
        let cpu_weight = config.cpu_weight.clamp(MIN_CPU_WEIGHT, MAX_CPU_WEIGHT);
        if let Err(e) = fs::write(slice.join("cpu.weight"), format!("{cpu_weight}\n")) {
            return cage_write_failure("cpu.weight", &e, &slice);
        }
        if let Err(e) = fs::write(slice.join("cgroup.procs"), format!("{pid}\n")) {
            return cage_write_failure("cgroup.procs", &e, &slice);
        }
        CageStatus::Active {
            cgroup_path: slice.display().to_string(),
        }
    }

    /// Remove the slice directory. Tolerates "already gone" so
    /// supervisors can call it unconditionally on worker death
    /// without checking liveness first. Other IO errors propagate.
    ///
    /// Note: cgroup-v2 requires the slice to be empty (no processes
    /// in `cgroup.procs`) before `rmdir` succeeds. Supervisors
    /// should call `uninstall` only after confirming the worker
    /// subprocess has exited.
    pub fn uninstall(&self, worker_id: &str) -> io::Result<()> {
        let slice = self.slice_path(worker_id);
        match fs::remove_dir(&slice) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(target_os = "linux")]
fn cage_write_failure(field: &str, err: &io::Error, slice: &Path) -> CageStatus {
    if err.kind() == io::ErrorKind::PermissionDenied {
        return CageStatus::NotSupported {
            reason: format!(
                "permission denied writing {} in {} (run under systemd-run --user --scope?)",
                field,
                slice.display()
            ),
        };
    }
    CageStatus::Failed {
        reason: format!("write {} in {}: {err}", field, slice.display()),
    }
}

