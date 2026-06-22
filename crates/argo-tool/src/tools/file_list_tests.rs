//! Tests for the `file_list` directory lister — listing, glob filter,
//! and the workspace jail (mirrors `file_read`'s `#677` refusals).
use super::*;
use argo_types::Principal;
use std::sync::Arc;

fn ctx_in(dir: &std::path::Path) -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        dir.to_path_buf(),
    )
}

fn body(result: &ToolCallResult) -> String {
    match &result.content[0] {
        argo_types::ContentBlock::Text { text } => text.clone(),
        _ => panic!("expected text"),
    }
}

#[tokio::test]
async fn lists_workspace_directory_entries() {
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("survey.csv"), "a").await.unwrap();
    tokio::fs::write(dir.path().join("reef-1.csv"), "bb").await.unwrap();
    tokio::fs::create_dir(dir.path().join("reports")).await.unwrap();

    let tool: Arc<dyn Tool> = Arc::new(FileList);
    let result = tool
        .dispatch(serde_json::json!({ "path": "." }), &ctx_in(dir.path()))
        .await
        .unwrap();
    assert!(!result.is_error);
    let text = body(&result);
    assert!(text.contains("survey.csv"), "got: {text}");
    assert!(text.contains("reef-1.csv"), "got: {text}");
    // Directories are listed and sorted ahead of files.
    assert!(text.contains("dir   reports"), "got: {text}");
    let dir_pos = text.find("reports").unwrap();
    let file_pos = text.find("survey.csv").unwrap();
    assert!(dir_pos < file_pos, "dirs should sort before files: {text}");
}

#[tokio::test]
async fn honors_a_glob_filter_on_names() {
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("survey.csv"), "a").await.unwrap();
    tokio::fs::write(dir.path().join("notes.txt"), "b").await.unwrap();
    tokio::fs::write(dir.path().join("reef-1.csv"), "c").await.unwrap();

    let tool: Arc<dyn Tool> = Arc::new(FileList);
    let result = tool
        .dispatch(
            serde_json::json!({ "path": ".", "glob": "*.csv" }),
            &ctx_in(dir.path()),
        )
        .await
        .unwrap();
    let text = body(&result);
    assert!(text.contains("survey.csv"), "got: {text}");
    assert!(text.contains("reef-1.csv"), "got: {text}");
    assert!(!text.contains("notes.txt"), "glob should exclude .txt: {text}");
}

#[tokio::test]
async fn empty_directory_reports_empty() {
    let dir = tempfile::tempdir().unwrap();
    let tool: Arc<dyn Tool> = Arc::new(FileList);
    let result = tool
        .dispatch(serde_json::json!({ "path": "." }), &ctx_in(dir.path()))
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(body(&result).contains("(empty)"));
}

#[tokio::test]
async fn refuses_absolute_path_outside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let tool: Arc<dyn Tool> = Arc::new(FileList);
    let result = tool
        .dispatch(serde_json::json!({ "path": "/etc" }), &ctx_in(dir.path()))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(
        body(&result).contains("outside the workspace root"),
        "got: {}",
        body(&result)
    );
}

#[tokio::test]
async fn refuses_parent_traversal_path() {
    let dir = tempfile::tempdir().unwrap();
    let tool: Arc<dyn Tool> = Arc::new(FileList);
    let result = tool
        .dispatch(serde_json::json!({ "path": "../.." }), &ctx_in(dir.path()))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(body(&result).contains("`..`"), "got: {}", body(&result));
}

#[tokio::test]
async fn listing_a_nonexistent_dir_is_error_true() {
    let dir = tempfile::tempdir().unwrap();
    let tool: Arc<dyn Tool> = Arc::new(FileList);
    let result = tool
        .dispatch(serde_json::json!({ "path": "nope" }), &ctx_in(dir.path()))
        .await
        .unwrap();
    assert!(result.is_error);
}

#[test]
fn required_capability_is_fsread_narrowed_to_path() {
    let caps = FileList
        .required_capabilities(&serde_json::json!({ "path": "inbox" }))
        .unwrap();
    assert!(caps.covers(&Capability::FsRead {
        path_glob: "inbox".into()
    }));
}

#[test]
fn in_ctx_resolves_relative_against_root_and_grant_covers() {
    let ctx = ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        std::path::PathBuf::from("/tmp/w"),
    );
    let caps = FileList
        .required_capabilities_in_ctx(&serde_json::json!({ "path": "inbox" }), &ctx)
        .unwrap();
    let glob = match &caps.0[0] {
        Capability::FsRead { path_glob } => path_glob.replace('\\', "/"),
        other => panic!("expected FsRead, got {other:?}"),
    };
    assert_eq!(glob, "/tmp/w/inbox");
    let grant = CapabilitySet(vec![Capability::FsRead {
        path_glob: "/tmp/w/**".into(),
    }]);
    assert!(grant.covers(&caps.0[0]));
}

#[test]
fn glob_match_handles_common_patterns() {
    assert!(glob_match("*.csv", "survey.csv"));
    assert!(glob_match("reef-*", "reef-1.csv"));
    assert!(glob_match("*report*", "q3-report-final.txt"));
    assert!(glob_match("*", "anything"));
    assert!(glob_match("exact.txt", "exact.txt"));
    assert!(!glob_match("*.csv", "notes.txt"));
    assert!(!glob_match("reef-*", "coral.csv"));
    assert!(!glob_match("exact.txt", "exact.txtx"));
}
