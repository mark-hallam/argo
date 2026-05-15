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

    workspace
        .install(
            &skill_md("tenant-draft", "user", "workspace-private-body"),
            None,
        )
        .await
        .unwrap();
    library
        .install(
            &skill_md(
                "marketplace-followup",
                "published:verified:vendor",
                "marketplace-body",
            ),
            None,
        )
        .await
        .unwrap();
    library
        .install(
            &skill_md("other-tenant-private", "user", "private-library-body"),
            None,
        )
        .await
        .unwrap();

    let active =
        turn_helpers::load_prompt_skills_with_library(&workspace, sid, None, Some(&library))
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
