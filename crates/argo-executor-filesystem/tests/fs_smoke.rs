//! Integration smoke for `argo-executor-filesystem`. Runs each op
//! end-to-end against a real `tempdir()` sandbox and checks the
//! sandbox-violation paths fail-closed before any I/O lands.

use argo_executor_filesystem::{ops, FsExecutorError, FsOpKind, Sandbox as FsSandbox};

#[test]
fn full_sandbox_lifecycle_six_ops() {
    let dir = tempfile::tempdir().unwrap();
    let sb = FsSandbox::open(dir.path()).unwrap();

    // 1. mkdir + 2. write_file
    ops::mkdir(&sb, "a/b").unwrap();
    ops::write_file(&sb, "a/b/note.txt", b"hello").unwrap();

    // 3. stat
    let meta = ops::stat(&sb, "a/b/note.txt").unwrap();
    assert!(meta.exists && !meta.is_dir && meta.size == 5);

    // 4. read_file
    let body = ops::read_file(&sb, "a/b/note.txt").unwrap();
    assert_eq!(body, b"hello");

    // 5. list_dir
    let entries = ops::list_dir(&sb, "a/b").unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "note.txt");

    // 6. remove
    ops::remove(&sb, "a/b/note.txt").unwrap();
    assert!(!ops::stat(&sb, "a/b/note.txt").unwrap().exists);

    // Read-only classification matches the AutoApprovalPolicy
    // brief — read-side ops auto-approve, mutating ops gate.
    assert!(FsOpKind::ReadFile.is_read_only());
    assert!(FsOpKind::ListDir.is_read_only());
    assert!(FsOpKind::Stat.is_read_only());
    assert!(!FsOpKind::WriteFile.is_read_only());
    assert!(!FsOpKind::Mkdir.is_read_only());
    assert!(!FsOpKind::Remove.is_read_only());
}

#[test]
fn sandbox_refuses_three_known_escape_attempts() {
    let dir = tempfile::tempdir().unwrap();
    let sb = FsSandbox::open(dir.path()).unwrap();

    // Absolute path
    let abs_err = ops::read_file(&sb, "/etc/passwd").unwrap_err();
    assert!(matches!(abs_err, FsExecutorError::PathEscape(_)));

    // Parent traversal
    let parent_err = ops::write_file(&sb, "../leak.bin", b"x").unwrap_err();
    assert!(matches!(parent_err, FsExecutorError::PathEscape(_)));

    // Per-op size cap
    let tiny = FsSandbox::open(dir.path()).unwrap().with_caps(4, 1024);
    let cap_err = ops::write_file(&tiny, "big.bin", b"five!").unwrap_err();
    assert!(matches!(cap_err, FsExecutorError::OpSizeCap { .. }));
}
