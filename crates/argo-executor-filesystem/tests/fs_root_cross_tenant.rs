//! Cross-tenant fs-root enforcement (mission claim 5).
//!
//! Per the gap manifest at
//! `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
//! (claim-5 row, `argo-executor-filesystem`):
//!
//! > `eval-filesystem-executor-fs-root-enforced` — assert a worker
//! > cannot read outside its `worker_fs_root`.
//!
//! `Sandbox` / `ops` are the per-worker isolation primitive that the
//! dispatch layer (`argo-cli/.../worker_actions_executor_stubs_filesystem.rs`)
//! opens once per (tenant, worker) at `worker.fs_root`. This file
//! pins the contract upstream callers rely on: given two disjoint
//! roots, no op exposed by the executor can let one root reach the
//! other through any documented escape vector (absolute path, `..`
//! traversal, or symlink pointing outside the root).
//!
//! Tests are named `worker_a_*` / `worker_b_*` because at this layer
//! the "tenant" axis collapses into "a pair of distinct fs_roots";
//! tenant_id plumbing happens upstream in the dispatch handler.

use argo_executor_filesystem::{ops, FsConfig, FsExecutorError, Sandbox};
use tempfile::{tempdir, TempDir};

/// Build a pair of isolated worker sandboxes under one parent
/// tempdir. Returns (parent_tempdir, sb_a, sb_b). Each sandbox has
/// its own canonical root and small explicit caps so quota tests
/// stay fast. Per-op cap is set well above any test write so quota
/// failures surface as `TotalSizeCap`, not `OpSizeCap`.
fn two_worker_sandboxes() -> (TempDir, Sandbox, Sandbox) {
    let parent = tempdir().unwrap();
    let root_a = parent.path().join("worker_a");
    let root_b = parent.path().join("worker_b");
    let sb_a = Sandbox::open(&root_a).unwrap().with_caps(8192, 4096);
    let sb_b = Sandbox::open(&root_b).unwrap().with_caps(8192, 4096);
    (parent, sb_a, sb_b)
}

#[test]
fn worker_a_and_worker_b_have_distinct_canonical_roots() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    assert_ne!(sb_a.root(), sb_b.root());
    assert!(!sb_a.root().starts_with(sb_b.root()));
    assert!(!sb_b.root().starts_with(sb_a.root()));
}

#[test]
fn worker_b_cannot_read_worker_a_secret_via_absolute_path() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    ops::write_file(&sb_a, "secret.txt", b"tenant_a loot").unwrap();
    // worker_b's executor receives "<absolute path inside worker_a>".
    let abs = sb_a.root().join("secret.txt");
    let err = ops::read_file(&sb_b, &abs).unwrap_err();
    assert!(matches!(err, FsExecutorError::PathEscape(_)));
}

#[test]
fn worker_b_cannot_write_into_worker_a_via_absolute_path() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    let abs = sb_a.root().join("planted.txt");
    let err = ops::write_file(&sb_b, &abs, b"x").unwrap_err();
    assert!(matches!(err, FsExecutorError::PathEscape(_)));
    assert!(!abs.exists(), "no bytes should have landed in worker_a's root");
}

#[test]
fn worker_b_cannot_reach_worker_a_via_parent_traversal_no_matter_how_deep() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    ops::write_file(&sb_a, "secret.txt", b"tenant_a loot").unwrap();

    // Each of these forms tries to climb out of worker_b and down
    // into worker_a. `resolve` rejects on the first `..` it sees.
    for attempt in [
        "../worker_a/secret.txt",
        "./../worker_a/secret.txt",
        "sub/../../worker_a/secret.txt",
        "a/b/c/../../../../worker_a/secret.txt",
    ] {
        let err = ops::read_file(&sb_b, attempt).unwrap_err();
        assert!(
            matches!(err, FsExecutorError::PathEscape(_)),
            "expected PathEscape for {attempt}, got {err:?}"
        );
    }
}

#[test]
fn worker_b_cannot_list_or_stat_worker_a_root_via_absolute_path() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    ops::write_file(&sb_a, "secret.txt", b"loot").unwrap();
    let abs = sb_a.root();

    let list_err = ops::list_dir(&sb_b, abs).unwrap_err();
    assert!(matches!(list_err, FsExecutorError::PathEscape(_)));

    let stat_err = ops::stat(&sb_b, abs).unwrap_err();
    assert!(matches!(stat_err, FsExecutorError::PathEscape(_)));
}

#[test]
fn worker_b_cannot_remove_entries_inside_worker_a_via_absolute_path() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    ops::write_file(&sb_a, "stay.txt", b"hi").unwrap();
    let abs = sb_a.root().join("stay.txt");

    let err = ops::remove(&sb_b, &abs).unwrap_err();
    assert!(matches!(err, FsExecutorError::PathEscape(_)));
    assert!(abs.exists(), "worker_a's file must remain");
}

#[cfg(unix)]
#[test]
fn worker_b_cannot_read_through_pre_planted_symlink_into_worker_a() {
    use std::os::unix::fs::symlink;
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    ops::write_file(&sb_a, "secret.txt", b"tenant_a loot").unwrap();

    // Pre-plant a symlink inside worker_b's root pointing at
    // worker_a's secret. This is the "adversary planted a symlink
    // out-of-band" scenario — the executor's `assert_no_symlink_escape`
    // is the load-bearing defence here.
    let link = sb_b.root().join("peek.txt");
    symlink(sb_a.root().join("secret.txt"), &link).unwrap();

    let err = ops::read_file(&sb_b, "peek.txt").unwrap_err();
    assert!(matches!(err, FsExecutorError::SymlinkEscape(_)));
}

#[cfg(unix)]
#[test]
fn worker_b_cannot_overwrite_existing_file_inside_worker_a_via_symlink() {
    use std::os::unix::fs::symlink;
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    ops::write_file(&sb_a, "secret.txt", b"original").unwrap();

    let link = sb_b.root().join("clobber.txt");
    symlink(sb_a.root().join("secret.txt"), &link).unwrap();

    // The file the symlink targets already exists, so the symlink
    // escape check fires before the write lands.
    let err = ops::write_file(&sb_b, "clobber.txt", b"pwn").unwrap_err();
    assert!(matches!(err, FsExecutorError::SymlinkEscape(_)));

    let still = ops::read_file(&sb_a, "secret.txt").unwrap();
    assert_eq!(still, b"original");
}

#[test]
fn default_root_for_returns_distinct_paths_per_worker_id() {
    let p_a = FsConfig::default_root_for("worker-a-uuid");
    let p_b = FsConfig::default_root_for("worker-b-uuid");
    assert_ne!(p_a, p_b);
    assert!(!p_a.starts_with(&p_b));
    assert!(!p_b.starts_with(&p_a));
}

#[test]
fn worker_a_total_size_cap_does_not_consume_worker_b_quota() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    // sb_a's 4096-byte total cap is unrelated to sb_b's.
    ops::write_file(&sb_a, "fat.bin", &[0u8; 4096]).unwrap();
    // worker_b's quota is still pristine — it can fill independently.
    ops::write_file(&sb_b, "fat.bin", &[0u8; 4000]).unwrap();
    assert_eq!(ops::stat(&sb_b, "fat.bin").unwrap().size, 4000);
}

#[test]
fn worker_a_filling_its_cap_does_not_leak_into_worker_b_walk() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    ops::write_file(&sb_a, "fill_a.bin", &[0u8; 3000]).unwrap();
    // sb_b's preflight_total_size walks ONLY sb_b's root, so the
    // 3000 bytes sitting inside worker_a don't count against
    // worker_b's 4096-byte cap.
    ops::write_file(&sb_b, "fill_b.bin", &[0u8; 3500]).unwrap();
    assert_eq!(ops::stat(&sb_b, "fill_b.bin").unwrap().size, 3500);
}

#[test]
fn worker_b_cannot_list_dir_via_parent_traversal_into_worker_a() {
    let (_parent, sb_a, sb_b) = two_worker_sandboxes();
    ops::mkdir(&sb_a, "treasure").unwrap();
    ops::write_file(&sb_a, "treasure/map.txt", b"x marks the spot").unwrap();

    let err = ops::list_dir(&sb_b, "../worker_a/treasure").unwrap_err();
    assert!(matches!(err, FsExecutorError::PathEscape(_)));
}

#[test]
fn empty_relative_path_resolves_to_worker_root_only() {
    // Sanity pin — `list_dir(sb, "")` and `list_dir(sb, ".")` should
    // both land on the worker's own root and never escape upward.
    let (_parent, sb_a, _sb_b) = two_worker_sandboxes();
    ops::write_file(&sb_a, "hi.txt", b"hi").unwrap();
    let entries_dot = ops::list_dir(&sb_a, ".").unwrap();
    let entries_empty = ops::list_dir(&sb_a, "").unwrap();
    assert_eq!(entries_dot, entries_empty);
    assert!(entries_dot.iter().any(|e| e.name == "hi.txt"));
}

// --- FINDINGS -----------------------------------------------------
//
// Finding 1 — directory-symlink in a non-leaf path component
// bypasses `assert_no_symlink_escape` for writes.
//
// Root cause: `Sandbox::assert_no_symlink_escape` short-circuits on
// `!abs.exists()`. For a write target that does not yet exist, the
// final path doesn't exist either, so the canonicalisation check
// never runs. `fs::write` then follows the directory-symlink in the
// middle of the path and lands the bytes outside the sandbox.
//
// Exploit requires an adversary to plant a directory-symlink inside
// the worker's `fs_root` out-of-band — the 6 ops exposed today don't
// create symlinks, so a well-behaved worker cannot reach this state
// alone. Treat as defense-in-depth: a partially-failed mkdir, an
// inherited fs_root, or a misconfigured operator helper could leave
// a stale symlink behind, and the executor would happily ferry bytes
// through it.
//
// Fix-brief (to be queued): `fix-fs-executor-reject-symlink-dir-components`
// — walk every intermediate component of `abs` in `assert_no_symlink_escape`
// and reject if any resolves to a symlink whose target escapes the root,
// regardless of whether the final path exists.
//
// Until the fix lands, this regression hook stays `#[ignore]`'d and
// documents the gap. Flip to live on the same PR that ships the fix.

#[cfg(unix)]
#[test]
#[ignore = "FINDINGS: assert_no_symlink_escape doesn't walk intermediate symlink dir-components; fix-brief: fix-fs-executor-reject-symlink-dir-components"]
fn write_through_directory_symlink_dir_component_escapes_to_other_worker_root() {
    use std::os::unix::fs::symlink;
    let (_parent, _sb_a, sb_b) = two_worker_sandboxes();
    // Pre-plant a directory-symlink inside worker_b's root pointing
    // at worker_a's root.
    let outside = tempdir().unwrap();
    symlink(outside.path(), sb_b.root().join("peek")).unwrap();

    // Write through the dir-symlink. Today this *succeeds* and the
    // bytes land in `outside`/`new.txt` — i.e. ESCAPE.
    let res = ops::write_file(&sb_b, "peek/new.txt", b"loot");
    assert!(
        res.is_err(),
        "executor must reject write through directory-symlink that escapes root"
    );
}
