//! Integration tests for marketplace-library skill loading in the
//! runtime prompt path.
#![cfg(test)]

use super::*;
use argo_skill::SkillStore;
use argo_store::SessionStore;
use std::sync::Arc;

fn skill_md(name: &str, source: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"fixture\"\n\
         provenance:\n  source: {source}\n  hash: sha256-x\n---\n{body}",
    )
}

async fn migrated_store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn published_library_skill_reaches_prompt_without_private_library_leak() {
    let workspace_store = migrated_store().await;
    let library_store = migrated_store().await;
    let workspace = SkillStore::new(Arc::clone(&workspace_store));
    let library = SkillStore::new(library_store);

    let (tenant, ws) = workspace_store
        .ensure_workspace("tenant-a", "/tmp/library-load-tenant-a")
        .await
        .unwrap();
    let sid = workspace_store
        .create_session(tenant, ws, "m", "p", Some("worker"), None)
        .await
        .unwrap();

    // WS1: the prompt path is tenant-scoped (strict #2848 readers), so
    // fixtures install per-tenant. The session tenant's own rows are
    // visible; ANOTHER tenant's library row must never surface.
    let tenant_str = tenant.to_string();
    workspace
        .install_for_tenant(
            &skill_md("tenant-draft", "user", "workspace-private-body"),
            &tenant_str,
            None,
        )
        .await
        .unwrap();
    library
        .install_for_tenant(
            &skill_md(
                "marketplace-followup",
                "published:verified:vendor",
                "marketplace-body",
            ),
            &tenant_str,
            None,
        )
        .await
        .unwrap();
    library
        .install_for_tenant(
            &skill_md("other-tenant-private", "user", "private-library-body"),
            "tenant-b-someone-else",
            None,
        )
        .await
        .unwrap();

    let active = turn_helpers::load_prompt_skills_with_library(
        &workspace,
        sid,
        None,
        tenant,
        Some(&library),
        None,
    )
    .await
    .unwrap();
    let prompt = turn_helpers::render_skill_prompt(active);

    assert!(prompt.contains("workspace-private-body"));
    assert!(prompt.contains("marketplace-body"));
    assert!(prompt.contains("## marketplace-followup (v0.1.0)"));
    assert!(
        !prompt.contains("private-library-body"),
        "tenant-private skills from the library tier must not enter another worker prompt:\n{prompt}",
    );
}

#[tokio::test]
async fn worker_prompt_never_fed_other_tenants_skills_on_shared_store() {
    // Acceptance for `fix-layered-lookup-worker-prompt-tenant-scope`:
    // on ONE shared multi-tenant store (the production shape), a
    // session in tenant A must never have tenant B's active skills
    // loaded into its prompt — with and without a library layer.
    let shared = migrated_store().await;
    let workspace = SkillStore::new(Arc::clone(&shared));

    let (tenant_a, ws_a) = shared
        .ensure_workspace("tenant-a", "/tmp/ll-shared-a")
        .await
        .unwrap();
    let (tenant_b, _ws_b) = shared
        .ensure_workspace("tenant-b", "/tmp/ll-shared-b")
        .await
        .unwrap();
    assert_ne!(tenant_a, tenant_b);
    let sid_a = shared
        .create_session(tenant_a, ws_a, "m", "p", Some("worker"), None)
        .await
        .unwrap();

    workspace
        .install_for_tenant(
            &skill_md("a-own-skill", "user", "tenant-a-body"),
            &tenant_a.to_string(),
            None,
        )
        .await
        .unwrap();
    workspace
        .install_for_tenant(
            &skill_md("b-secret-skill", "user", "tenant-b-secret-body"),
            &tenant_b.to_string(),
            None,
        )
        .await
        .unwrap();

    // No-library fallback path (workspace.list_active_for_tenant).
    let active =
        turn_helpers::load_prompt_skills_with_library(&workspace, sid_a, None, tenant_a, None, None)
            .await
            .unwrap();
    let prompt = turn_helpers::render_skill_prompt(active);
    assert!(prompt.contains("tenant-a-body"), "own skill must load:\n{prompt}");
    assert!(
        !prompt.contains("tenant-b-secret-body") && !prompt.contains("b-secret-skill"),
        "tenant B's skill must NEVER reach tenant A's prompt (fallback path):\n{prompt}"
    );

    // LayeredLookup path (shared store doubling as the library layer —
    // worst case: B's row sits in BOTH layers).
    let active = turn_helpers::load_prompt_skills_with_library(
        &workspace,
        sid_a,
        None,
        tenant_a,
        Some(&workspace),
        None,
    )
    .await
    .unwrap();
    let prompt = turn_helpers::render_skill_prompt(active);
    assert!(prompt.contains("tenant-a-body"));
    assert!(
        !prompt.contains("tenant-b-secret-body") && !prompt.contains("b-secret-skill"),
        "tenant B's skill must NEVER reach tenant A's prompt (library path):\n{prompt}"
    );
}
