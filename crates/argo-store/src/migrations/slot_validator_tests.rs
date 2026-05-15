use super::{
    canonical_version_for_raw_file, unexpected_duplicate_slots, validate_registered_versions,
};

#[test]
fn clean_registry_versions_pass() {
    let versions = 1..=69;
    validate_registered_versions(versions).expect("dense registry is accepted");
}

#[test]
fn duplicate_registry_versions_fail() {
    let err = validate_registered_versions([1, 2, 2, 3]).expect_err("duplicate rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("duplicate migration registry version(s): 2"),
        "unexpected error: {msg}"
    );
}

#[test]
fn historical_raw_slot_collisions_are_allowlisted() {
    let collisions = unexpected_duplicate_slots([
        "19__anomaly.sql",
        "19__worker_fs_root.sql",
        "22__roster.sql",
        "22__tenant_archived.sql",
        "75__scim_provisioned_tombstones.sql",
        "75__siem_dispatch_cursor.sql",
    ]);
    assert!(
        collisions.is_empty(),
        "historical collisions are Path A allowlisted"
    );
}

#[test]
fn synthetic_raw_slot_collision_is_reported() {
    let collisions =
        unexpected_duplicate_slots(["78__first.sql", "78__second.sql", "79__third.sql"]);
    assert_eq!(collisions.len(), 1);
    assert_eq!(collisions[0].slot, 78);
    assert_eq!(
        collisions[0].files,
        vec!["78__first.sql".to_string(), "78__second.sql".to_string()]
    );
}

#[test]
fn canonical_mapping_ignores_duplicate_raw_prefixes() {
    assert_eq!(canonical_version_for_raw_file("19__anomaly.sql"), Some(19));
    assert_eq!(
        canonical_version_for_raw_file("19__worker_fs_root.sql"),
        Some(20)
    );
    assert_eq!(
        canonical_version_for_raw_file("75__scim_provisioned_tombstones.sql"),
        Some(64)
    );
    assert_eq!(
        canonical_version_for_raw_file("75__siem_dispatch_cursor.sql"),
        Some(0),
        "raw-only/lazy helpers do not advance schema_version"
    );
}
