use super::*;
use argo_identity::IdentityStore;

fn base_spec() -> OnboardSpec {
    parse_spec_str(
        r##"
        vertical_id = "gtm-engineer"
        memory_namespace = "maya-demo"

        [persona]
        name = "Maya"
        voice = "Be concise and commercial."
        signature = "Maya"

        [role]
        description = "Own approved marketplace follow-up."

        [[channels]]
        channel = "email"
        target = "ops@example.test"
        risk_level = "high"

        [[duties]]
        cron = "0 9 * * 1"
        entry_action = "weekly_pipeline_review"
        description = "Review approved leads."
        "##,
    )
    .unwrap()
}

async fn store() -> (SessionStore, TenantId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    (store, TenantId::new())
}

#[test]
fn validate_rejects_missing_vertical_id() {
    let mut spec = base_spec();
    spec.vertical_id = None;
    let err = validate(&spec).unwrap_err();
    assert!(err.to_string().contains("missing required vertical_id"));
}

#[test]
fn validate_rejects_bad_vertical_id_and_big_charter() {
    let mut spec = base_spec();
    spec.vertical_id = Some("bad vertical".into());
    let err = validate(&spec).unwrap_err();
    assert!(err.to_string().contains("vertical_id 'bad vertical'"));

    spec.vertical_id = Some("gtm".into());
    spec.role.description = Some("x".repeat(MAX_CHARTER_BYTES + 1));
    let err = validate(&spec).unwrap_err();
    assert!(err.to_string().contains("role.description exceeds"));
}

#[tokio::test]
async fn create_from_spec_persists_worker_channels_and_duties() {
    let (store, tenant) = store().await;
    let out = create_from_spec(&store, tenant, base_spec(), None)
        .await
        .unwrap();
    let id = parse_worker_id(&out.worker_id).unwrap();
    let loaded = IdentityStore::new(&store)
        .get_worker(tenant, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.persona.name, "Maya");
    assert_eq!(loaded.channels.len(), 1);
    assert_eq!(loaded.role.duties.len(), 1);
    assert_eq!(
        loaded.charter.as_deref(),
        Some("Own approved marketplace follow-up.")
    );
}

#[tokio::test]
async fn too_many_channels_fail_before_db_write() {
    let (store, tenant) = store().await;
    let mut spec = base_spec();
    spec.channels = (0..9)
        .map(|i| {
            ChannelSpec {
                channel: Some("email".into()),
                target: format!("ops{i}@example.test"),
                risk_level: None,
            }
        })
        .collect();
    let err = create_from_spec(&store, tenant, spec, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("at most 8"));
    let workers = IdentityStore::new(&store)
        .list_workers(tenant)
        .await
        .unwrap();
    assert!(workers.is_empty());
}

#[tokio::test]
async fn unsupported_channel_fails_after_worker_create() {
    let (store, tenant) = store().await;
    let mut spec = base_spec();
    spec.channels = vec![ChannelSpec {
        channel: Some("fax".into()),
        target: "+15551234567".into(),
        risk_level: None,
    }];
    let err = create_from_spec(&store, tenant, spec, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("fax:+15551234567"));
    let workers = IdentityStore::new(&store)
        .list_workers(tenant)
        .await
        .unwrap();
    assert_eq!(workers.len(), 1);
    assert!(workers[0].channels.is_empty());
}

#[tokio::test]
async fn explicit_id_collision_fails_without_mutation() {
    let (store, tenant) = store().await;
    let id = WorkerId::new();
    create_from_spec(&store, tenant, base_spec(), Some(id))
        .await
        .unwrap();
    let mut second = base_spec();
    second.persona.name = Some("Maya Clone".into());
    let err = create_from_spec(&store, tenant, second, Some(id))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("already exists"));
    let loaded = IdentityStore::new(&store)
        .get_worker(tenant, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.persona.name, "Maya");
}

#[test]
fn parses_shorthand_duty_and_channel() {
    let spec = parse_spec_str(
        r##"
        vertical_id = "support"
        duties = ["0 8 * * 1::daily_check::Check queue"]
        [persona]
        name = "Ari"
        voice = "Terse."
        signature = "Ari"
        [role]
        description = "Triage support."
        [[channels]]
        channel = "slack"
        target = "#ops"
        "##,
    )
    .unwrap();
    validate(&spec).unwrap();
    assert_eq!(parse_duties(&spec.duties).unwrap().len(), 1);
    assert_eq!(parse_channel(&spec.channels[0]).unwrap().channel, "slack");
}
