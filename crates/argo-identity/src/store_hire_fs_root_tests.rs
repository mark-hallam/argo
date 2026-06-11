//! Tests for the Genesis default `worker.fs_root` (day-1 unblock fix 1).
//!
//! Pins three behaviours of [`crate::resolved_fs_root`] + the
//! [`IdentityStore::create_worker_from_draft`] persistence:
//!
//! 1. a Genesis hire (draft carries `genesis_plan_id`) defaults
//!    `worker.fs_root` to `<home>/SkycotTeammates/<persona-slug>/`;
//! 2. a non-Genesis hire leaves `fs_root` `NULL` (unchanged behaviour);
//! 3. an explicit `draft.fs_root` always wins over the default.

#![allow(clippy::expect_used)]

use crate::store_hire_fs_root::{default_genesis_fs_root, resolved_fs_root};
use crate::{ApprovalPolicyChoice, HireDraft, HireDutyDraft, IdentityStore};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::{Path, PathBuf};

fn draft(name: &str, genesis_plan_id: Option<&str>, fs_root: Option<&str>) -> HireDraft {
    HireDraft {
        suggested_name: name.into(),
        charter: "Read invoice files from the inbox and draft reminders.".into(),
        suggested_vertical: "genesis-ava-019e".into(),
        approval_policy: ApprovalPolicyChoice {
            level: "balanced".into(),
        },
        duties: vec![HireDutyDraft {
            cron: "0 9 * * 1-5".into(),
            entry_action: "follow_up".into(),
            description: "Send approved follow-up.".into(),
            default_enabled: true,
        }],
        voice_profile: None,
        tagline: None,
        genesis_plan_id: genesis_plan_id.map(str::to_string),
        fs_root: fs_root.map(str::to_string),
    }
}

async fn mem_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    store
}

async fn stored_fs_root(store: &SessionStore, worker_id: &str) -> Option<String> {
    sqlx::query_scalar("SELECT fs_root FROM worker WHERE id = ?")
        .bind(worker_id)
        .fetch_one(store.pool())
        .await
        .expect("read fs_root")
}

#[test]
fn genesis_draft_defaults_to_teammates_dir_under_home() {
    let d = draft("Ava", Some("genesis-ava-019e"), None);
    let resolved = resolved_fs_root(&d, Some(Path::new("/home/sam")));
    assert_eq!(
        resolved,
        Some(PathBuf::from("/home/sam/SkycotTeammates/ava")),
    );
}

#[test]
fn persona_slug_sanitizes_spaces_and_case() {
    let root = default_genesis_fs_root(Path::new("/h"), "Maria Garcia");
    assert_eq!(root, PathBuf::from("/h/SkycotTeammates/maria-garcia"));
    // Nothing name-like survives → the safe "worker" fallback slug.
    let root = default_genesis_fs_root(Path::new("/h"), "!!!");
    assert_eq!(root, PathBuf::from("/h/SkycotTeammates/worker"));
}

#[test]
fn non_genesis_draft_resolves_to_none() {
    let d = draft("Ava", None, None);
    assert_eq!(resolved_fs_root(&d, Some(Path::new("/home/sam"))), None);
    // A whitespace-only plan id is NOT Genesis provenance.
    let d = draft("Ava", Some("  "), None);
    assert_eq!(resolved_fs_root(&d, Some(Path::new("/home/sam"))), None);
}

#[test]
fn explicit_fs_root_wins_over_genesis_default() {
    let d = draft("Ava", Some("genesis-ava-019e"), Some("/data/inbox"));
    assert_eq!(
        resolved_fs_root(&d, Some(Path::new("/home/sam"))),
        Some(PathBuf::from("/data/inbox")),
    );
}

#[test]
fn missing_home_skips_the_default_but_not_the_explicit_value() {
    let d = draft("Ava", Some("genesis-ava-019e"), None);
    assert_eq!(resolved_fs_root(&d, None), None);
    let d = draft("Ava", Some("genesis-ava-019e"), Some("/data/inbox"));
    assert_eq!(resolved_fs_root(&d, None), Some(PathBuf::from("/data/inbox")));
}

#[tokio::test]
async fn genesis_hire_persists_default_fs_root_column() {
    let store = mem_store().await;
    let d = draft("Ava", Some("genesis-ava-019e"), None);
    let id = IdentityStore::new(&store)
        .create_worker_from_draft(&d, TenantId::new(), "human:op")
        .await
        .expect("hire");
    // The test process always has HOME set under cargo; derive the
    // expected path from the same env var the production seam reads.
    let home = std::env::var("HOME").expect("HOME set in test env");
    let expected = Path::new(&home)
        .join("SkycotTeammates")
        .join("ava")
        .display()
        .to_string();
    assert_eq!(
        stored_fs_root(&store, &id.to_string()).await.as_deref(),
        Some(expected.as_str()),
    );
}

#[tokio::test]
async fn non_genesis_hire_leaves_fs_root_null() {
    let store = mem_store().await;
    let d = draft("Maya", None, None);
    let id = IdentityStore::new(&store)
        .create_worker_from_draft(&d, TenantId::new(), "human:op")
        .await
        .expect("hire");
    assert_eq!(stored_fs_root(&store, &id.to_string()).await, None);
}

#[tokio::test]
async fn explicit_draft_fs_root_persists_verbatim() {
    let store = mem_store().await;
    let d = draft("Ava", Some("genesis-ava-019e"), Some("/data/inbox"));
    let id = IdentityStore::new(&store)
        .create_worker_from_draft(&d, TenantId::new(), "human:op")
        .await
        .expect("hire");
    assert_eq!(
        stored_fs_root(&store, &id.to_string()).await.as_deref(),
        Some("/data/inbox"),
    );
}
