//! Linux-only end-to-end smoke test for
//! [`argo_runtime::worker_resource_cage::WorkerResourceCage`]
//! (`mission-cgroup-per-worker-resource-limits`).
//!
//! Marked `#[ignore]` so the default `cargo test` suite doesn't try
//! to run it on macOS / Windows runners or on Linux CI without
//! cgroup-v2 write access. Run explicitly on a cgroup-v2 host with:
//!
//! ```bash
//! cargo test --test cage_smoke -- --ignored
//! ```
//!
//! What it exercises:
//!
//! 1. `fork-exec /bin/sleep 5` and capture the child pid.
//! 2. `install_on` with a 16 MiB memory cap.
//! 3. Assert the pid is present in the slice's `cgroup.procs`.
//! 4. Allocate >16 MiB in a worker subprocess and confirm the
//!    kernel OOM-killer fires (signal 9) within the cap.
//! 5. `uninstall` cleans the slice after the worker exits.
//!
//! Cap is 16 MiB so the test fits on minimal CI runners; `sleep`
//! itself uses <1 MiB of RSS so the basic membership check passes
//! without triggering OOM (the OOM-kill assertion lives in a
//! follow-up E2E suite once we have a known-large worker binary on
//! the runner image).

#![cfg(target_os = "linux")]

use argo_runtime::worker_resource_cage::{
    CageStatus, WorkerResourceCage, WorkerResourceConfig,
};
use std::process::Command;

#[test]
#[ignore]
fn install_on_real_cgroup_v2_host() {
    let cage = WorkerResourceCage::new();
    let worker_id = format!("smoke-{}", std::process::id());
    let cfg = WorkerResourceConfig {
        enabled: true,
        memory_max_mib: 16,
        cpu_weight: 100,
    };

    let mut child = Command::new("/bin/sleep")
        .arg("5")
        .spawn()
        .expect("spawn /bin/sleep");
    let pid = child.id();

    let status = cage.install_on(&worker_id, pid, &cfg);
    let cgroup_path = match status {
        CageStatus::Active { cgroup_path } => cgroup_path,
        CageStatus::NotSupported { reason } => {
            // CI without cgroup-v2 write access — skip gracefully.
            // `--ignored` runners that don't grant write access should
            // not fail the assertion; the test exists to confirm the
            // primitive works *when* the platform supports it.
            child.kill().ok();
            child.wait().ok();
            eprintln!("skip: host lacks cgroup-v2 write access: {reason}");
            return;
        }
        CageStatus::Failed { reason } => {
            child.kill().ok();
            child.wait().ok();
            panic!("install_on failed: {reason}");
        }
    };

    let procs_path = std::path::Path::new(&cgroup_path).join("cgroup.procs");
    let procs = std::fs::read_to_string(&procs_path).expect("read cgroup.procs");
    assert!(
        procs.lines().any(|l| l.trim() == pid.to_string()),
        "pid {pid} must appear in {}: got {procs:?}",
        procs_path.display()
    );

    // Sleep should still be alive — 16 MiB is enough for `/bin/sleep`.
    child.kill().expect("kill sleep");
    child.wait().expect("reap sleep");

    cage.uninstall(&worker_id).expect("uninstall");
    assert!(
        !std::path::Path::new(&cgroup_path).exists(),
        "slice {cgroup_path} should be removed after uninstall"
    );
}
