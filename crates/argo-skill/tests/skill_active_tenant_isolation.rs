//! WS1 cross-tenant negative pins for the active-skill READ methods at
//! the argo-skill library layer.
//!
//! Brief: `skill-store-list-active-cross-tenant-leak`. Before this fix,
//! `SkillStore::find_active_by_name` and `SkillStore::list_active`
//! filtered only on `lifecycle_state = 'active'` with NO `tenant_id`
//! clause, so a tenant's skill lookup / active-skill list surfaced
//! EVERY tenant's active skills (cross-tenant read; also a skill-name
//! existence-probing channel via the name lookup). The `skill` table
//! does carry `tenant_id` (migration 131), and argo-store has a pin
//! test (`skill_query_tenant_bind_pin`) — but these two argo-skill
//! library methods were never bound.
//!
//! These tests pin the new strict variants (`*_for_tenant`) AND assert
//! the legacy global methods still leak (the documented v1 permissive
//! seam) so a future change that accidentally flips the seam is caught.

use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

/// Minimal valid SKILL.md document for `name`.
fn skill_md(name: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"isolation test {name}\"\n\
         provenance:\n  source: \"inline\"\n  hash: sha256-test\n\
         ---\nBody for {name}.\n"
    )
}

async fn store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

#[tokio::test]
async fn list_active_for_tenant_does_not_leak_across_tenants() {
    let store = store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let tenant_a = TenantId::new().to_string();
    let tenant_b = TenantId::new().to_string();

    skills
        .install_for_tenant(&skill_md("alpha-a"), &tenant_a, None)
        .await
        .unwrap();
    skills
        .install_for_tenant(&skill_md("beta-b"), &tenant_b, None)
        .await
        .unwrap();

    // Strict path: each tenant sees ONLY its own active skill.
    let a_names: Vec<String> = skills
        .list_active_for_tenant(&tenant_a)
        .await
        .unwrap()
        .iter()
        .map(|s| s.frontmatter.name.clone())
        .collect();
    assert_eq!(a_names, vec!["alpha-a".to_string()]);
    assert!(!a_names.contains(&"beta-b".to_string()));

    let b_names: Vec<String> = skills
        .list_active_for_tenant(&tenant_b)
        .await
        .unwrap()
        .iter()
        .map(|s| s.frontmatter.name.clone())
        .collect();
    assert_eq!(b_names, vec!["beta-b".to_string()]);
    assert!(!b_names.contains(&"alpha-a".to_string()));

    // Legacy non-tenant `list_active` is the documented permissive seam:
    // it returns BOTH tenants' rows. This is the leak the strict variant
    // closes; pinning it green guards the seam from a silent flip.
    assert_eq!(
        skills.list_active().await.unwrap().len(),
        2,
        "legacy list_active is the global permissive seam (returns all tenants)"
    );
}

#[tokio::test]
async fn find_active_by_name_for_tenant_does_not_leak_across_tenants() {
    let store = store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let tenant_a = TenantId::new().to_string();
    let tenant_b = TenantId::new().to_string();

    skills
        .install_for_tenant(&skill_md("private"), &tenant_a, None)
        .await
        .unwrap();

    // Tenant B can NOT see — nor probe the existence of — tenant A's
    // active skill by name.
    assert!(
        skills
            .find_active_by_name_for_tenant("private", &tenant_b)
            .await
            .unwrap()
            .is_none(),
        "tenant B must not surface tenant A's skill via name lookup"
    );

    // Tenant A can.
    let found = skills
        .find_active_by_name_for_tenant("private", &tenant_a)
        .await
        .unwrap();
    assert_eq!(found.unwrap().frontmatter.name, "private");

    // Legacy global name lookup still leaks across tenants (the seam).
    assert!(
        skills.find_active_by_name("private").await.unwrap().is_some(),
        "legacy find_active_by_name is the global permissive seam"
    );
}
