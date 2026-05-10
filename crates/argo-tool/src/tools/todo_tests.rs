//! Unit tests for the `todo` tool. Split out per the
//! `memory_tests.rs` / `vision_tests.rs` convention so the
//! implementation file stays close to the brief's 200 LOC sizing
//! hint.

use super::*;
use argo_types::{ContentBlock, Principal};

fn ctx(root: PathBuf) -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        root,
    )
}

fn body(r: &ToolCallResult) -> String {
    r.content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn add_then_list_returns_the_new_item() {
    let dir = tempfile::tempdir().unwrap();
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "add", "item": "draft PRD"}),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", body(&r));
    let added: TodoItem = serde_json::from_str(&body(&r)).unwrap();
    assert_eq!(added.id, 1);
    assert_eq!(added.text, "draft PRD");
    assert!(!added.done);

    let r = Todo
        .dispatch(
            serde_json::json!({"action": "list"}),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .unwrap();
    let listed: Vec<TodoItem> = serde_json::from_str(&body(&r)).unwrap();
    assert_eq!(listed, vec![added]);
}

#[tokio::test]
async fn complete_flips_done_in_place() {
    let dir = tempfile::tempdir().unwrap();
    Todo.dispatch(
        serde_json::json!({"action": "add", "item": "first"}),
        &ctx(dir.path().to_path_buf()),
    )
    .await
    .unwrap();
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "complete", "id": 1}),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .unwrap();
    let updated: TodoItem = serde_json::from_str(&body(&r)).unwrap();
    assert!(updated.done);
}

#[tokio::test]
async fn remove_deletes_by_id() {
    let dir = tempfile::tempdir().unwrap();
    for t in ["a", "b", "c"] {
        Todo.dispatch(
            serde_json::json!({"action": "add", "item": t}),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .unwrap();
    }
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "remove", "id": 2}),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    let listed = Todo
        .dispatch(
            serde_json::json!({"action": "list"}),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .unwrap();
    let items: Vec<TodoItem> = serde_json::from_str(&body(&listed)).unwrap();
    assert_eq!(items.iter().map(|i| i.id).collect::<Vec<_>>(), vec![1, 3]);
}

#[tokio::test]
async fn list_when_empty_returns_empty_array() {
    let dir = tempfile::tempdir().unwrap();
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "list"}),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert_eq!(body(&r), "[]");
}

#[tokio::test]
async fn complete_with_unknown_id_is_clean_error() {
    let dir = tempfile::tempdir().unwrap();
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "complete", "id": 99}),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .unwrap();
    assert!(r.is_error);
    assert!(body(&r).contains("no todo with id 99"), "{}", body(&r));
}

#[test]
fn add_requires_fs_write_capability() {
    let caps = Todo
        .required_capabilities(&serde_json::json!({"action": "add", "item": "x"}))
        .unwrap();
    assert!(caps.covers(&Capability::FsWrite {
        path_glob: ".argo/todo.json".into(),
    }));
}

#[test]
fn list_requires_only_fs_read_capability() {
    let caps = Todo
        .required_capabilities(&serde_json::json!({"action": "list"}))
        .unwrap();
    assert!(caps.covers(&Capability::FsRead {
        path_glob: ".argo/todo.json".into(),
    }));
    assert!(!caps.covers(&Capability::FsWrite {
        path_glob: ".argo/todo.json".into(),
    }));
}
