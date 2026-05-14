//! Unit tests for the `skill_version_migration` storage layer
//! (`mission-skill-version-migration-path` + publisher-ownership
//! gate from `mission-skill-version-migration-publisher-check`).
//!
//! Coverage map:
//! 1. `record_roundtrip` — insert + lookup returns matching row.
//! 2. `idempotent_on_pk` — re-recording the same triple is a no-op.
//! 3. `lookup_by_installed_version` — only the matching `from`
//!    surfaces.
//! 4. `lookup_no_match_returns_none` — unknown version → None.
//! 5. `list_for_skill_multi_rows` — list returns every row, newest
//!    first.
//! 6. `auto_upgrade_flag_roundtrip` — bool persists both ways.
//! 7. `reason_text_length_cap` — long reason truncated to
//!    `MIGRATION_REASON_MAX_LEN` chars.
//! 8. `same_tenant_subsequent_calls_succeed` — canonical publisher
//!    can record further rows for its own skill family.
//! 9. `cross_tenant_record_migration_rejected` — non-canonical
//!    publisher gets `CapabilityDenied(SkillMigrateRecord)` and
//!    cannot leak a row into another family's migration graph.
//! 10. `independent_families_have_independent_publishers` — TOFU is
//!     per `skill_id`; different families bind separately.
use super::*;
use argo_store::SessionStore;
use argo_types::{Capability, SkillId, TenantId};

async fn open_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn record_roundtrip() {
    let store = open_store().await;
    let skill_id = SkillId::new();
    let publisher = TenantId::new();
    record_migration(&store, skill_id, "v1", "v2", publisher, Some("bug fix"), false)
        .await
        .unwrap();
    let found = lookup_migration(&store, skill_id, "v1").await.unwrap();
    let row = found.expect("expected v1->v2 row");
    assert_eq!(row.from_version, "v1");
    assert_eq!(row.to_version, "v2");
    assert_eq!(row.reason.as_deref(), Some("bug fix"));
    assert!(!row.auto_upgrade);
}

#[tokio::test]
async fn idempotent_on_pk() {
    let store = open_store().await;
    let skill_id = SkillId::new();
    let publisher = TenantId::new();
    record_migration(&store, skill_id, "v1", "v2", publisher, None, false)
        .await
        .unwrap();
    record_migration(&store, skill_id, "v1", "v2", publisher, Some("new note"), true)
        .await
        .unwrap();
    let rows = list_for_skill(&store, skill_id).await.unwrap();
    assert_eq!(rows.len(), 1, "PK collision must NOT insert a duplicate");
    // First-write-wins: original empty reason + auto_upgrade=false stay.
    assert_eq!(rows[0].reason, None);
    assert!(!rows[0].auto_upgrade);
}

#[tokio::test]
async fn lookup_by_installed_version() {
    let store = open_store().await;
    let skill_id = SkillId::new();
    let publisher = TenantId::new();
    record_migration(&store, skill_id, "v1", "v2", publisher, None, false)
        .await
        .unwrap();
    record_migration(&store, skill_id, "v2", "v3", publisher, None, true)
        .await
        .unwrap();
    let v1 = lookup_migration(&store, skill_id, "v1").await.unwrap();
    assert_eq!(v1.unwrap().to_version, "v2");
    let v2 = lookup_migration(&store, skill_id, "v2").await.unwrap();
    assert_eq!(v2.unwrap().to_version, "v3");
}

#[tokio::test]
async fn lookup_no_match_returns_none() {
    let store = open_store().await;
    let skill_id = SkillId::new();
    let other_skill = SkillId::new();
    let publisher = TenantId::new();
    record_migration(&store, skill_id, "v1", "v2", publisher, None, false)
        .await
        .unwrap();
    let none_for_other = lookup_migration(&store, other_skill, "v1").await.unwrap();
    assert!(none_for_other.is_none());
    let none_for_unknown_ver = lookup_migration(&store, skill_id, "v9").await.unwrap();
    assert!(none_for_unknown_ver.is_none());
}

#[tokio::test]
async fn list_for_skill_multi_rows() {
    let store = open_store().await;
    let skill_id = SkillId::new();
    let publisher = TenantId::new();
    for (from, to) in [("v1", "v2"), ("v2", "v3"), ("v1", "v3")] {
        record_migration(&store, skill_id, from, to, publisher, None, false)
            .await
            .unwrap();
    }
    let rows = list_for_skill(&store, skill_id).await.unwrap();
    assert_eq!(rows.len(), 3);
    let pairs: Vec<(String, String)> = rows
        .iter()
        .map(|r| (r.from_version.clone(), r.to_version.clone()))
        .collect();
    assert!(pairs.contains(&("v1".to_string(), "v2".to_string())));
    assert!(pairs.contains(&("v2".to_string(), "v3".to_string())));
    assert!(pairs.contains(&("v1".to_string(), "v3".to_string())));
}

#[tokio::test]
async fn auto_upgrade_flag_roundtrip() {
    let store = open_store().await;
    let skill_id = SkillId::new();
    let publisher = TenantId::new();
    record_migration(&store, skill_id, "v1", "v2", publisher, None, true)
        .await
        .unwrap();
    let row = lookup_migration(&store, skill_id, "v1")
        .await
        .unwrap()
        .unwrap();
    assert!(row.auto_upgrade, "auto_upgrade=true must persist");
}

#[tokio::test]
async fn same_tenant_subsequent_calls_succeed() {
    // The canonical publisher (first record) records two further
    // (from, to) pairs without rejection.
    let store = open_store().await;
    let skill_id = SkillId::new();
    let publisher = TenantId::new();
    record_migration(&store, skill_id, "v1", "v2", publisher, None, false)
        .await
        .unwrap();
    record_migration(&store, skill_id, "v2", "v3", publisher, Some("note"), true)
        .await
        .unwrap();
    record_migration(&store, skill_id, "v1", "v3", publisher, None, false)
        .await
        .unwrap();
    let rows = list_for_skill(&store, skill_id).await.unwrap();
    assert_eq!(rows.len(), 3, "all three rows persisted for canonical publisher");
}

#[tokio::test]
async fn cross_tenant_record_migration_rejected() {
    // First call binds tenant A as canonical publisher. Tenant B's
    // attempt to record a different (from, to) pair must be rejected
    // and must NOT leak a row into the migration graph.
    let store = open_store().await;
    let skill_id = SkillId::new();
    let canonical = TenantId::new();
    let attacker = TenantId::new();
    assert_ne!(canonical, attacker);
    record_migration(&store, skill_id, "v1", "v2", canonical, None, false)
        .await
        .unwrap();
    let err = record_migration(&store, skill_id, "v2", "v3", attacker, None, false)
        .await
        .unwrap_err();
    match err {
        ArgoError::CapabilityDenied(Capability::SkillMigrateRecord) => {}
        other => panic!("expected CapabilityDenied(SkillMigrateRecord), got {other:?}"),
    }
    let rows = list_for_skill(&store, skill_id).await.unwrap();
    assert_eq!(rows.len(), 1, "rejected attempt must not leak a row");
    assert_eq!(rows[0].from_version, "v1");
    assert_eq!(rows[0].to_version, "v2");
}

#[tokio::test]
async fn independent_families_have_independent_publishers() {
    // TOFU binding is scoped per skill_id — tenant A owns skill X,
    // tenant B owns skill Y, and cross-family attempts on either
    // family still reject.
    let store = open_store().await;
    let skill_a = SkillId::new();
    let skill_b = SkillId::new();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    record_migration(&store, skill_a, "v1", "v2", tenant_a, None, false)
        .await
        .unwrap();
    record_migration(&store, skill_b, "v1", "v2", tenant_b, None, false)
        .await
        .unwrap();
    let cross_err = record_migration(&store, skill_a, "v2", "v3", tenant_b, None, false)
        .await
        .unwrap_err();
    assert!(matches!(
        cross_err,
        ArgoError::CapabilityDenied(Capability::SkillMigrateRecord)
    ));
}

#[tokio::test]
async fn reason_text_length_cap() {
    let store = open_store().await;
    let skill_id = SkillId::new();
    let publisher = TenantId::new();
    let long = "x".repeat(MIGRATION_REASON_MAX_LEN + 50);
    record_migration(&store, skill_id, "v1", "v2", publisher, Some(&long), false)
        .await
        .unwrap();
    let row = lookup_migration(&store, skill_id, "v1")
        .await
        .unwrap()
        .unwrap();
    let stored = row.reason.unwrap();
    let char_count = stored.chars().count();
    assert_eq!(
        char_count, MIGRATION_REASON_MAX_LEN,
        "reason must be truncated to MIGRATION_REASON_MAX_LEN chars"
    );
    assert!(stored.ends_with('…'), "truncated reason ends with ellipsis");
}
