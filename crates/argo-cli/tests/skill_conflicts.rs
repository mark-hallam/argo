//! Integration test for the skill-conflicts subsystem (Stream
//! `claude-skill-conflict-resolution`). Builds two SkillStores
//! (one workspace, one library), seeds two skills with the same
//! name into both, and walks `detect_conflicts` →
//! `transition_lifecycle` to assert the resolve flow archives
//! the loser.
use argo_skill::{detect_conflicts, render_diff, LifecycleState, SkillLayer, SkillStore};
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_skill_store() -> SkillStore {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    SkillStore::new(store)
}

fn skill_md(name: &str, version: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: stub\nlock: none\n\
         provenance:\n  source: test\n  hash: sha256-{}\n---\n{body}",
        body.len()
    )
}

#[tokio::test]
async fn detect_conflicts_finds_shared_names_only() {
    let workspace = fresh_skill_store().await;
    let library = fresh_skill_store().await;

    workspace
        .install(&skill_md("summariser", "0.4", "ws body"), None)
        .await
        .unwrap();
    workspace
        .install(&skill_md("ws-only-skill", "0.1", "x"), None)
        .await
        .unwrap();

    library
        .install(&skill_md("summariser", "0.3", "lib body"), None)
        .await
        .unwrap();
    library
        .install(&skill_md("lib-only-skill", "0.1", "y"), None)
        .await
        .unwrap();

    let conflicts = detect_conflicts(&workspace, &library).await.unwrap();
    assert_eq!(conflicts.len(), 1, "only `summariser` collides");
    let c = &conflicts[0];
    assert_eq!(c.name, "summariser");
    assert_eq!(c.workspace.frontmatter.version, "0.4");
    assert_eq!(c.library.frontmatter.version, "0.3");
}

#[tokio::test]
async fn render_diff_shows_per_layer_marks() {
    let workspace = fresh_skill_store().await;
    let library = fresh_skill_store().await;
    workspace
        .install(&skill_md("summariser", "0.4", "alpha\nbeta"), None)
        .await
        .unwrap();
    library
        .install(&skill_md("summariser", "0.3", "alpha\ngamma"), None)
        .await
        .unwrap();
    let conflicts = detect_conflicts(&workspace, &library).await.unwrap();
    let d = render_diff(&conflicts[0]);
    assert!(d.starts_with("--- workspace/summariser@0.4"));
    assert!(d.contains("+++ library/summariser@0.3"));
    assert!(d.contains(" alpha"));
    assert!(d.contains("-beta"));
    assert!(d.contains("+gamma"));
}

#[tokio::test]
async fn resolve_archives_the_loser() {
    let workspace = fresh_skill_store().await;
    let library = fresh_skill_store().await;
    workspace
        .install(&skill_md("summariser", "0.4", "ws"), None)
        .await
        .unwrap();
    library
        .install(&skill_md("summariser", "0.3", "lib"), None)
        .await
        .unwrap();

    let conflicts = detect_conflicts(&workspace, &library).await.unwrap();
    let conflict = &conflicts[0];

    // Keep workspace → archive library.
    let loser = conflict.loser(SkillLayer::Workspace);
    library
        .transition_lifecycle(loser.id, LifecycleState::Archived)
        .await
        .unwrap();

    // After resolution: detect_conflicts (which only walks active
    // skills) must show zero conflicts because the library row is
    // now Archived.
    let post = detect_conflicts(&workspace, &library).await.unwrap();
    assert!(
        post.is_empty(),
        "library row should be archived; conflicts list must be empty"
    );

    // Workspace winner is still active.
    let still = workspace
        .find_active_by_name("summariser")
        .await
        .unwrap()
        .expect("workspace summariser still active");
    assert_eq!(still.frontmatter.version, "0.4");
}
