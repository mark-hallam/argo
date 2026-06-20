//! Tests for the Genesis default `worker.fs_root` (day-1 unblock fix 1).
//!
//! Pins three behaviours of [`crate::resolved_fs_root`] + the
//! [`IdentityStore::create_worker_from_draft`] persistence:
//!
//! 1. a Genesis hire (draft carries `genesis_plan_id`) defaults
//!    `worker.fs_root` to `<home>/SkycotTeammates/<persona-slug>/`;
//! 2. a non-Genesis hire leaves `fs_root` `NULL` (unchanged behaviour);
//! 3. an explicit `draft.fs_root` UNDER HOME wins over the default, but an
//!    outside-HOME / traversal explicit value (untrusted HTTP hire field) is
//!    rejected and falls through — fix-explicit-fs-root-tier-validate-under-home.

#![allow(clippy::expect_used)]

use crate::store_hire_fs_root::{default_genesis_fs_root, resolved_fs_root, safe_access_fs_root};
use crate::{ApprovalPolicyChoice, HireDraft, HireDutyDraft, IdentityStore};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::{Path, PathBuf};

fn draft(name: &str, genesis_plan_id: Option<&str>, fs_root: Option<&str>) -> HireDraft {
    draft_with_access(name, genesis_plan_id, fs_root, None)
}

fn draft_with_access(
    name: &str,
    genesis_plan_id: Option<&str>,
    fs_root: Option<&str>,
    access_fs_root: Option<&str>,
) -> HireDraft {
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
        kpis: vec![],
        voice_profile: None,
        tagline: None,
        genesis_plan_id: genesis_plan_id.map(str::to_string),
        fs_root: fs_root.map(str::to_string),
        access_fs_root: access_fs_root.map(str::to_string),
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
fn explicit_fs_root_under_home_wins_over_genesis_default() {
    let d = draft("Ava", Some("genesis-ava-019e"), Some("/home/sam/Projects/acme"));
    assert_eq!(
        resolved_fs_root(&d, Some(Path::new("/home/sam"))),
        Some(PathBuf::from("/home/sam/Projects/acme")),
    );
}

// fix-explicit-fs-root-tier-validate-under-home (security, acceptance pin 1):
// the explicit `draft.fs_root` field rides the untrusted `POST /api/v1/workers`
// hire body. An outside-HOME / traversal value must NOT bind — it is rejected
// and falls back to the generated Genesis default (it can no longer escape a
// fenced teammate's sandbox to `/etc`).
#[test]
fn explicit_outside_home_fs_root_is_rejected_falls_to_default() {
    for outside in ["/etc", "/", "/Users/OtherUser/secrets", "../../etc"] {
        let d = draft("Ava", Some("genesis-ava-019e"), Some(outside));
        assert_eq!(
            resolved_fs_root(&d, Some(Path::new("/home/sam"))),
            Some(PathBuf::from("/home/sam/SkycotTeammates/ava")),
            "outside-HOME explicit fs_root {outside:?} must fall back to the default",
        );
    }
}

// On a NON-Genesis hire (no default to fall to), an outside-HOME explicit value
// yields None rather than binding it.
#[test]
fn explicit_outside_home_fs_root_non_genesis_is_none() {
    let d = draft("Ava", None, Some("/etc"));
    assert_eq!(resolved_fs_root(&d, Some(Path::new("/home/sam"))), None);
}

// uf-bind-intake-access-folder-to-fs-root — bind the folder the user NAMED
// at intake (acceptance pin 1: real folder under HOME → that folder).
#[test]
fn captured_access_folder_under_home_binds_over_genesis_default() {
    let d = draft_with_access("Ava", Some("genesis-ava-019e"), None, Some("SkycotDemo/Inbox"));
    assert_eq!(
        resolved_fs_root(&d, Some(Path::new("/home/sam"))),
        Some(PathBuf::from("/home/sam/SkycotDemo/Inbox")),
    );
    // A `~`-rooted answer expands to HOME.
    let d = draft_with_access("Ava", Some("genesis-ava-019e"), None, Some("~/Documents/Invoices"));
    assert_eq!(
        resolved_fs_root(&d, Some(Path::new("/home/sam"))),
        Some(PathBuf::from("/home/sam/Documents/Invoices")),
    );
}

// Acceptance pin 2 (regression): a vague / non-path Access answer leaves the
// generated Genesis default unchanged.
#[test]
fn vague_access_answer_keeps_genesis_default() {
    // Whitespace and a bare integration name are not local folder paths.
    for vague in ["   ", "Gmail and the shared drive"] {
        let d = draft_with_access("Ava", Some("genesis-ava-019e"), None, Some(vague));
        assert_eq!(
            resolved_fs_root(&d, Some(Path::new("/home/sam"))),
            Some(PathBuf::from("/home/sam/SkycotTeammates/ava")),
            "vague access answer {vague:?} must keep the default",
        );
    }
}

// Acceptance pin 3 (security): a traversal / outside-HOME Access path is
// rejected and falls back to the generated default — never bound.
#[test]
fn traversal_or_outside_home_access_path_is_rejected() {
    for unsafe_path in ["../../etc", "/etc/passwd", "SkycotDemo/../../../etc"] {
        let d = draft_with_access("Ava", Some("genesis-ava-019e"), None, Some(unsafe_path));
        assert_eq!(
            resolved_fs_root(&d, Some(Path::new("/home/sam"))),
            Some(PathBuf::from("/home/sam/SkycotTeammates/ava")),
            "unsafe access path {unsafe_path:?} must fall back to the default",
        );
    }
}

// win-identity-fs-root-path-portability: a platform-native ABSOLUTE explicit
// path under home must be recognized as a path (not mis-rejected by the
// unix-only `/` heuristic) and kept as-is. Before the fix, a Windows
// `C:\...` candidate has no `/`, so safe_access_fs_root returned None and the
// teammate's sandbox silently fell back to the generated default.
#[test]
fn native_absolute_under_home_access_is_accepted() {
    #[cfg(windows)]
    let (home, candidate, want) = (
        Path::new(r"C:\Users\sam"),
        r"C:\Users\sam\Projects\acme",
        PathBuf::from(r"C:\Users\sam\Projects\acme"),
    );
    #[cfg(not(windows))]
    let (home, candidate, want) = (
        Path::new("/home/sam"),
        "/home/sam/Projects/acme",
        PathBuf::from("/home/sam/Projects/acme"),
    );
    assert_eq!(safe_access_fs_root(candidate, home), Some(want));
}

// win-identity-fs-root-path-portability: a multi-segment relative candidate
// must persist with NATIVE separators — `home.join("a/b")` preserved the
// literal `/` on Windows, producing mixed separators (`C:\home\a/b`). Assert
// the string form so the mixed-separator regression is caught directly.
#[test]
fn relative_access_folder_uses_native_separators() {
    #[cfg(windows)]
    let (home, want) = (
        Path::new(r"C:\Users\sam"),
        r"C:\Users\sam\SkycotDemo\Inbox",
    );
    #[cfg(not(windows))]
    let (home, want) = (Path::new("/home/sam"), "/home/sam/SkycotDemo/Inbox");
    let got = safe_access_fs_root("SkycotDemo/Inbox", home).expect("binds under home");
    assert_eq!(got.to_string_lossy(), want);
}

// Acceptance pin 3b (security, adversarial-review hardening 2026-06-11): an
// empty `home` must NOT let an absolute Access path through — an empty path
// is a prefix of everything, so `starts_with` would be vacuously true. The
// guard fails closed (returns None) instead of binding e.g. `/etc`.
#[test]
fn empty_home_rejects_absolute_access_path() {
    for abs in ["/etc", "/", "/Users/OtherUser/secrets"] {
        assert_eq!(
            safe_access_fs_root(abs, Path::new("")),
            None,
            "empty home must reject absolute access path {abs:?}",
        );
        // And via the draft resolver: empty home → access tier skipped, a
        // genesis hire falls back to None (no HOME to base a default on).
        let d = draft_with_access("Ava", Some("genesis-ava-019e"), None, Some(abs));
        assert_eq!(resolved_fs_root(&d, Some(Path::new(""))), None);
    }
}

// An explicit `fs_root` UNDER HOME still beats a captured Access folder.
#[test]
fn explicit_fs_root_under_home_wins_over_captured_access_folder() {
    let d = draft_with_access(
        "Ava",
        Some("genesis-ava-019e"),
        Some("/home/sam/Projects/acme"),
        Some("SkycotDemo/Inbox"),
    );
    assert_eq!(
        resolved_fs_root(&d, Some(Path::new("/home/sam"))),
        Some(PathBuf::from("/home/sam/Projects/acme")),
    );
}

// fix-explicit-fs-root-tier-validate-under-home: an OUTSIDE-HOME explicit value
// is rejected, so a valid captured Access folder under HOME still binds — the
// explicit field can no longer silently escape past the captured-Access tier.
#[test]
fn outside_home_explicit_falls_through_to_captured_access() {
    let d = draft_with_access(
        "Ava",
        Some("genesis-ava-019e"),
        Some("/etc"),
        Some("SkycotDemo/Inbox"),
    );
    assert_eq!(
        resolved_fs_root(&d, Some(Path::new("/home/sam"))),
        Some(PathBuf::from("/home/sam/SkycotDemo/Inbox")),
    );
}

// With no HOME to scope under, even an explicit `fs_root` cannot be validated
// as in-bounds, so it is skipped (fail-closed) — the resolver returns None
// rather than binding an unvalidated path. (The CLI operator-trusted path,
// which does NOT use this resolver, is the seam for a deliberate outside-HOME
// root.)
#[test]
fn missing_home_skips_default_and_unvalidatable_explicit() {
    let d = draft("Ava", Some("genesis-ava-019e"), None);
    assert_eq!(resolved_fs_root(&d, None), None);
    let d = draft("Ava", Some("genesis-ava-019e"), Some("/data/inbox"));
    assert_eq!(resolved_fs_root(&d, None), None);
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
async fn captured_access_folder_persists_to_fs_root_column() {
    let store = mem_store().await;
    // A relative folder name joins onto HOME, so it is in-bounds and binds.
    let d = draft_with_access(
        "Ava",
        Some("genesis-ava-019e"),
        None,
        Some("SkycotDemo/Inbox"),
    );
    let id = IdentityStore::new(&store)
        .create_worker_from_draft(&d, TenantId::new(), "human:op")
        .await
        .expect("hire");
    let home = std::env::var("HOME").expect("HOME set in test env");
    let expected = Path::new(&home)
        .join("SkycotDemo")
        .join("Inbox")
        .display()
        .to_string();
    assert_eq!(
        stored_fs_root(&store, &id.to_string()).await.as_deref(),
        Some(expected.as_str()),
    );
}

#[tokio::test]
async fn explicit_under_home_draft_fs_root_persists() {
    let store = mem_store().await;
    // Derive an in-bounds explicit path from the same env var the production
    // seam reads, so the under-HOME guard accepts it.
    let home = std::env::var("HOME").expect("HOME set in test env");
    let explicit = Path::new(&home).join("Projects").join("acme");
    let explicit_str = explicit.display().to_string();
    let d = draft("Ava", Some("genesis-ava-019e"), Some(&explicit_str));
    let id = IdentityStore::new(&store)
        .create_worker_from_draft(&d, TenantId::new(), "human:op")
        .await
        .expect("hire");
    assert_eq!(
        stored_fs_root(&store, &id.to_string()).await.as_deref(),
        Some(explicit_str.as_str()),
    );
}

// fix-explicit-fs-root-tier-validate-under-home (security, round-trip): an
// outside-HOME explicit `fs_root` on the untrusted hire path is NOT persisted;
// the column gets the generated Genesis default instead.
#[tokio::test]
async fn outside_home_explicit_fs_root_persists_genesis_default() {
    let store = mem_store().await;
    let d = draft("Ava", Some("genesis-ava-019e"), Some("/etc"));
    let id = IdentityStore::new(&store)
        .create_worker_from_draft(&d, TenantId::new(), "human:op")
        .await
        .expect("hire");
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
