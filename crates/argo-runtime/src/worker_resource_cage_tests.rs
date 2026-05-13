//! Unit tests for [`crate::worker_resource_cage`]. The path-
//! construction + config-default + disabled-config layers are pure
//! enough to test on every platform. The actual cgroup writes are
//! exercised in `crates/argo-runtime/tests/cage_smoke.rs`, gated
//! behind `#[cfg(target_os = "linux")] #[ignore]` so the suite
//! doesn't fault on macOS/Windows runners.

use crate::worker_resource_cage::{
    CageStatus, WorkerResourceCage, WorkerResourceConfig, DEFAULT_CGROUP_ROOT,
    DEFAULT_CPU_WEIGHT, DEFAULT_MEMORY_MAX_MIB, MAX_CPU_WEIGHT, MIN_CPU_WEIGHT, SLICE_PREFIX,
};

#[test]
fn defaults_match_documented_values() {
    let cfg = WorkerResourceConfig::default();
    assert!(cfg.enabled, "cage must default-enabled");
    assert_eq!(cfg.memory_max_mib, 1024);
    assert_eq!(cfg.cpu_weight, 100);
    assert_eq!(cfg.memory_max_mib, DEFAULT_MEMORY_MAX_MIB);
    assert_eq!(cfg.cpu_weight, DEFAULT_CPU_WEIGHT);
}

#[test]
fn cpu_weight_bounds_documented() {
    assert_eq!(MIN_CPU_WEIGHT, 1);
    assert_eq!(MAX_CPU_WEIGHT, 10_000);
}

#[test]
fn slice_path_uses_documented_prefix() {
    let cage = WorkerResourceCage::with_root("/sys/fs/cgroup");
    let path = cage.slice_path("wkr-abc-123");
    let expected = format!("{SLICE_PREFIX}wkr-abc-123");
    assert!(
        path.ends_with(&expected),
        "slice path {} must end with {}",
        path.display(),
        expected
    );
}

#[test]
fn slice_path_uses_configured_root() {
    let cage = WorkerResourceCage::with_root("/tmp/argo-test-cgroup");
    let path = cage.slice_path("wkr-1");
    assert!(
        path.starts_with("/tmp/argo-test-cgroup"),
        "expected configured root prefix, got {}",
        path.display()
    );
}

#[test]
fn default_cage_uses_sys_fs_cgroup_root() {
    let cage = WorkerResourceCage::new();
    assert_eq!(cage.cgroup_root().to_string_lossy(), DEFAULT_CGROUP_ROOT);
}

#[test]
fn disabled_config_returns_not_supported_with_canonical_reason() {
    let cage = WorkerResourceCage::with_root("/sys/fs/cgroup");
    let cfg = WorkerResourceConfig {
        enabled: false,
        ..WorkerResourceConfig::default()
    };
    let status = cage.install_on("wkr-1", 42, &cfg);
    match status {
        CageStatus::NotSupported { reason } => assert_eq!(reason, "disabled by config"),
        other => panic!("expected NotSupported, got {other:?}"),
    }
}

#[test]
#[cfg(not(target_os = "linux"))]
fn non_linux_returns_not_supported() {
    let cage = WorkerResourceCage::with_root("/sys/fs/cgroup");
    let cfg = WorkerResourceConfig::default();
    let status = cage.install_on("wkr-1", 42, &cfg);
    match status {
        CageStatus::NotSupported { reason } => {
            assert!(
                reason.contains("Linux"),
                "non-linux NotSupported reason should mention Linux: {reason}"
            );
        }
        other => panic!("expected NotSupported on non-Linux, got {other:?}"),
    }
}

#[test]
#[cfg(target_os = "linux")]
fn missing_cgroup_root_returns_not_supported_with_reason() {
    let tempdir = std::env::temp_dir().join(format!("argo-cage-missing-{}", std::process::id()));
    let cage = WorkerResourceCage::with_root(&tempdir);
    let cfg = WorkerResourceConfig::default();
    let status = cage.install_on("wkr-1", 42, &cfg);
    match status {
        CageStatus::NotSupported { reason } => {
            assert!(
                reason.contains("cgroup-v2 not mounted"),
                "expected mount-missing diagnostic, got: {reason}"
            );
        }
        other => panic!("expected NotSupported, got {other:?}"),
    }
}

#[test]
fn uninstall_is_idempotent_when_slice_absent() {
    let tempdir = std::env::temp_dir().join(format!("argo-cage-uninstall-{}", std::process::id()));
    let cage = WorkerResourceCage::with_root(&tempdir);
    // Slice was never created — uninstall must succeed (NotFound is tolerated).
    cage.uninstall("never-existed-wkr").expect("idempotent");
}

#[test]
fn config_roundtrips_through_serde_json() {
    let cfg = WorkerResourceConfig {
        enabled: false,
        memory_max_mib: 512,
        cpu_weight: 250,
    };
    let json = serde_json::to_string(&cfg).expect("serialize");
    let back: WorkerResourceConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(cfg, back);
}

#[test]
fn cage_status_active_serialises_with_kind_tag() {
    let status = CageStatus::Active {
        cgroup_path: "/sys/fs/cgroup/argo-worker-wkr-1".to_string(),
    };
    let json = serde_json::to_value(&status).expect("serialize");
    assert_eq!(json["kind"], "active");
    assert_eq!(json["cgroup_path"], "/sys/fs/cgroup/argo-worker-wkr-1");
}

#[test]
fn cage_status_not_supported_serialises_with_kind_tag() {
    let status = CageStatus::NotSupported {
        reason: "disabled by config".to_string(),
    };
    let json = serde_json::to_value(&status).expect("serialize");
    assert_eq!(json["kind"], "not_supported");
    assert_eq!(json["reason"], "disabled by config");
}
