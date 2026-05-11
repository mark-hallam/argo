//! Inline tests for [`IdentityStore::create_worker`] / `get_worker` /
//! `list_workers`. Lives in its own file so `store.rs` stays under the
//! 500-LOC ceiling.
#![cfg(test)]

use crate::{Duty, IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;

#[tokio::test]
async fn create_get_list_roundtrip() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let mut role = Role::new("Own outbound follow-up.");
    role.duties
        .push(Duty::new("0 9 * * 1-5", "follow_up", "Send approved follow-up."));
    let worker = Worker::draft(tenant, "gtm-engineer", Persona::named("Maya", "op"), role);
    let identities = IdentityStore::new(&store);

    identities.create_worker(&worker).await.unwrap();
    let loaded = identities
        .get_worker(tenant, worker.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.persona.name, "Maya");
    assert_eq!(loaded.role.duties[0].entry_action, "follow_up");
    assert_eq!(identities.list_workers(tenant, false).await.unwrap().len(), 1);
}
