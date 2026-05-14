//! Unit tests for [`crate::worker_supervisor`]. Included via
//! `#[path]` so the test file stays out of the parent's LOC count.
//!
//! Real fork-exec is exercised against `/bin/sleep` (long-lived) and
//! `true` (clean exit). Linux-only cage smoke lives in
//! `crates/argo-orchestrator/tests/worker_supervisor_smoke.rs` under
//! `#[ignore]`.

use super::*;
use argo_identity::WorkerId;
use argo_runtime::oom_restart::OomRestartConfig;
use argo_runtime::worker_resource_cage::{
    CageStatus, WorkerResourceCage, WorkerResourceConfig,
};
use argo_types::TenantId;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

fn fast_restart_config() -> OomRestartConfig {
    OomRestartConfig {
        max_restarts: 3,
        base_backoff: Duration::from_millis(20),
        max_backoff: Duration::from_millis(50),
        restart_window: Duration::from_secs(60),
    }
}

fn unconfined_supervisor() -> Arc<WorkerSupervisor> {
    // Use a non-existent root so `install_on` short-circuits with
    // `NotSupported` on every platform — the supervisor still
    // tracks the worker, the test stays hermetic.
    let cage = WorkerResourceCage::with_root("/tmp/argo-test-no-cage");
    Arc::new(WorkerSupervisor::new(
        cage,
        WorkerResourceConfig {
            enabled: false,
            ..WorkerResourceConfig::default()
        },
        fast_restart_config(),
    ))
}

fn sleep_spec(tenant: TenantId) -> WorkerSpec {
    let program = which_or_default("sleep", "/bin/sleep");
    WorkerSpec {
        worker_id: WorkerId::new(),
        tenant_id: tenant,
        program,
        args: vec!["30".to_string()],
        envs: Vec::new(),
    }
}

fn true_spec(tenant: TenantId) -> WorkerSpec {
    let program = which_or_default("true", "/usr/bin/true");
    WorkerSpec {
        worker_id: WorkerId::new(),
        tenant_id: tenant,
        program,
        args: Vec::new(),
        envs: Vec::new(),
    }
}

/// Best-effort PATH lookup. Falls back to `default_path` so the
/// tests stay deterministic on minimal containers where `which` is
/// absent.
fn which_or_default(bin: &str, default_path: &str) -> PathBuf {
    let candidates = [
        format!("/bin/{bin}"),
        format!("/usr/bin/{bin}"),
        default_path.to_string(),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return PathBuf::from(c);
        }
    }
    PathBuf::from(default_path)
}

#[test]
fn with_defaults_constructs_supervisor() {
    let s = WorkerSupervisor::with_defaults();
    // `worker_count` is async; just confirm we built the struct.
    let _ = Arc::new(s);
}

#[tokio::test]
async fn spawn_worker_registers_in_map() {
    let supervisor = unconfined_supervisor();
    let tenant = TenantId::new();
    let spec = sleep_spec(tenant);
    let worker_id = spec.worker_id;
    let snapshot = supervisor.spawn_worker(spec).await.expect("spawn");
    assert_eq!(snapshot.worker_id, worker_id);
    assert_eq!(snapshot.tenant_id, tenant);
    assert_eq!(snapshot.respawn_count, 0);
    assert!(snapshot.pid > 0);
    assert_eq!(supervisor.worker_count().await, 1);
    let fetched = supervisor.snapshot(&worker_id).await.expect("snapshot");
    assert_eq!(fetched.pid, snapshot.pid);
}

#[tokio::test]
async fn cage_disabled_yields_not_supported_status() {
    let supervisor = unconfined_supervisor();
    let spec = sleep_spec(TenantId::new());
    let snapshot = supervisor.spawn_worker(spec).await.expect("spawn");
    match snapshot.cage {
        CageStatus::NotSupported { reason } => {
            assert!(
                reason.contains("disabled") || reason.contains("cgroup") || reason.contains("Linux"),
                "expected platform/disabled reason, got: {reason}"
            );
        }
        other => panic!("expected NotSupported, got {other:?}"),
    }
}

#[tokio::test]
async fn workers_for_tenant_filters_by_tenant_id() {
    let supervisor = unconfined_supervisor();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let _ = supervisor.spawn_worker(sleep_spec(tenant_a)).await.expect("a1");
    let _ = supervisor.spawn_worker(sleep_spec(tenant_a)).await.expect("a2");
    let _ = supervisor.spawn_worker(sleep_spec(tenant_b)).await.expect("b1");
    assert_eq!(supervisor.workers_for_tenant(tenant_a).await, 2);
    assert_eq!(supervisor.workers_for_tenant(tenant_b).await, 1);
    assert_eq!(supervisor.worker_count().await, 3);
}

#[tokio::test]
async fn snapshot_returns_none_for_unknown_worker() {
    let supervisor = unconfined_supervisor();
    let phantom = WorkerId::new();
    assert!(supervisor.snapshot(&phantom).await.is_none());
}

#[tokio::test]
async fn clean_exit_deregisters_worker() {
    let supervisor = unconfined_supervisor();
    let spec = true_spec(TenantId::new());
    let worker_id = spec.worker_id;
    let _ = supervisor.spawn_worker(spec).await.expect("spawn /bin/true");
    // /bin/true exits immediately with code 0; classify_exit returns
    // Clean, decide_restart returns IgnoreCleanExit, the watcher
    // breaks the loop and deregisters.
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(25)).await;
        if supervisor.snapshot(&worker_id).await.is_none() {
            return;
        }
    }
    panic!("worker {worker_id} should have deregistered after clean exit");
}

#[tokio::test]
async fn killed_worker_respawns_within_budget() {
    let supervisor = unconfined_supervisor();
    let spec = sleep_spec(TenantId::new());
    let worker_id = spec.worker_id;
    let initial = supervisor.spawn_worker(spec).await.expect("spawn sleep");
    let initial_pid = initial.pid;

    send_sigkill(initial_pid);

    // Watcher should see signal 9, decide_restart returns Restart,
    // fork-exec a new sleep, and bump respawn_count to 1.
    for _ in 0..80 {
        tokio::time::sleep(Duration::from_millis(25)).await;
        if let Some(snap) = supervisor.snapshot(&worker_id).await {
            if snap.respawn_count >= 1 && snap.pid != initial_pid {
                return;
            }
        }
    }
    panic!("worker {worker_id} should have respawned after SIGKILL");
}

#[tokio::test]
async fn cage_status_is_not_supported_when_root_missing() {
    let supervisor = unconfined_supervisor();
    let spec = sleep_spec(TenantId::new());
    let snapshot = supervisor.spawn_worker(spec).await.expect("spawn");
    let cgroup_path = active_cgroup_path(&snapshot.cage);
    assert!(
        cgroup_path.is_none(),
        "unconfined supervisor must yield None cgroup_path, got {cgroup_path:?}"
    );
}

fn send_sigkill(pid: u32) {
    // Shell out to /bin/kill to avoid taking a libc dev-dep just
    // for the supervisor tests. Production paths never SIGKILL
    // workers; the cgroup-v2 OOM-kill path drives that.
    let _ = std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status();
}
