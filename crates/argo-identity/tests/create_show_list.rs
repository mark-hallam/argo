use argo_identity::{Duty, IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;

#[tokio::test]
async fn create_show_list_workers() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let mut role = Role::new("Own approved marketplace follow-up.");
    role.duties
        .push(Duty::new("0 8 * * 1-5", "daily_follow_up", "Follow up."));
    role.kpis.push("approved replies sent".into());
    role.escalation = Some("operator@example.com".into());

    let worker = Worker::draft(
        tenant,
        "gtm-engineer",
        Persona::named("Maya", "operator@example.com"),
        role,
    );
    let identities = IdentityStore::new(&store);
    identities.create_worker(&worker).await.unwrap();

    let shown = identities
        .get_worker(tenant, worker.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(shown.persona.name, "Maya");
    assert_eq!(shown.role.duties[0].cron, "0 8 * * 1-5");
    assert_eq!(shown.memory_namespace.tenant_id, tenant);

    let listed = identities.list_workers(tenant, false).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, worker.id);
}
