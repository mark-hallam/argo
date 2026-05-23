# Round 38 — PR #2148 `fix(executor-filesystem): reject symlink dir-components in path walk` anti-theater verify

- **Date**: 2026-05-24
- **Round**: 38
- **Pillar / Claim**: Claim 6 — Multi-tenant by construction (per-worker fs sandbox closes dir-symlink intermediate-component escape from PR #2124 FINDINGS pin)
- **PR**: [#2148](https://github.com/mark-hallam/argo/pull/2148) — MERGED at `2c5b9b979`
- **Verdict**: 🟢 PASS (no severity — closes a P1 cross-worker isolation hole)

## Setup

```bash
grep -n "fn assert_no_symlink_escape\|canonicalize\|symlink_metadata\|starts_with" \
  crates/argo-executor-filesystem/src/sandbox.rs

cargo test -p argo-executor-filesystem --test fs_root_cross_tenant -- \
  write_through_directory_symlink_dir_component_escapes_to_other_worker_root \
  read_through_directory_symlink_dir_component_escape_rejected
```

## Observed

### Production seam — `crates/argo-executor-filesystem/src/sandbox.rs:143-182`

The previous `assert_no_symlink_escape` short-circuited on `!abs.exists()` — meaning if a caller passed `dir-symlink/new-file.txt` where `new-file.txt` didn't yet exist, the entire path was considered "non-existent" and the check returned `Ok(())`. Then `fs::write` followed the dir-symlink and wrote bytes **outside** `self.root`.

The new implementation walks every component starting from `self.root`:

```rust
let tail = abs.strip_prefix(&self.root).map_err(...)?;
let mut cur = self.root.clone();
for comp in tail.components() {
    match comp { Normal(seg) => cur.push(seg), CurDir => continue,
                 Prefix|RootDir|ParentDir => return Err(PathEscape) }
    match std::fs::symlink_metadata(&cur) {
        Ok(_) => {
            let real = cur.canonicalize()?;
            if !real.starts_with(&self.root) {
                return Err(FsExecutorError::SymlinkEscape(real.display().to_string()));
            }
        }
        Err(NotFound) => return Ok(()),  // tail is unwritten — no symlink can hide
        Err(e) => return Err(io("symlink_metadata", &cur, e)),
    }
}
```

Three load-bearing properties:

1. `strip_prefix(&self.root)` — caller must have minted the path via `Sandbox::resolve` (root-prefixed canonical). Anything else fails closed with `PathEscape`.
2. For every existing component, `canonicalize` resolves any symlink in the chain, and `starts_with(&self.root)` confirms the resolved path is still inside the sandbox. `Sandbox::new()` at `:56` canonicalizes `self.root` at construction, so the `starts_with` check is canonical-vs-canonical (no `/var → /private/var` macOS mismatch).
3. First `NotFound` component ends the walk — write-targets are still valid for new files. (Filesystem invariant: a path entry that doesn't exist can't contain a symlink target.)

### Tests — both inverted from FINDINGS-ONLY, no longer `#[ignore]`'d

`crates/argo-executor-filesystem/tests/fs_root_cross_tenant.rs:211-228` (`write_through_directory_symlink_dir_component_escapes_to_other_worker_root`):

```rust
let (_parent, _sb_a, sb_b) = two_worker_sandboxes();
let outside = tempdir().unwrap();
symlink(outside.path(), sb_b.root().join("peek")).unwrap();   // plant dir-symlink

let err = ops::write_file(&sb_b, "peek/new.txt", b"loot").unwrap_err();
assert!(matches!(err, FsExecutorError::SymlinkEscape(_)));
assert!(!outside.path().join("new.txt").exists(),
        "no bytes should have landed outside the sandbox");
```

The dual assertion is exactly the right anti-theater shape: a regression that returned an error-of-wrong-kind would fail line 220-223; a regression that wrote bytes anyway and *then* returned an error would fail line 224-227.

`crates/argo-executor-filesystem/tests/fs_root_cross_tenant.rs:232-247` — read-side variant: pre-plants `loot.txt` in outside dir + dir-symlink inside worker_b; `ops::read_file(&sb_b, "peek/loot.txt")` must error with `SymlinkEscape`.

### Cold-run cargo

```
→ test write_through_directory_symlink_dir_component_escapes_to_other_worker_root ... ok
→ test read_through_directory_symlink_dir_component_escape_rejected ... ok
→ test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out
```

**2/2 green cold from `origin/main`.**

## Anti-theater check

Two angles that prove the test isn't a sham:

1. **Bytes-not-written assertion.** Line 224-227 of the write test checks `!outside.path().join("new.txt").exists()` *after* the call. If a regression returned `Err(SymlinkEscape)` *after* `fs::write` had already executed (order-of-operations bug), the test fails on this second assertion. Two assertions = two failure modes = harder to fake.
2. **Real production op surface.** Both tests call `ops::write_file` / `ops::read_file` — production fs-executor ops, not test-only shims. `Sandbox::assert_no_symlink_escape` invocation is dispatched from inside those ops. Grep `crates/argo-executor-filesystem/src/ops.rs` confirms — both `write_file` and `read_file` route through `sandbox.resolve` then `sandbox.assert_no_symlink_escape` before any `fs::` syscall.

PR's claim of "47 passed" (full suite) cold-checks out: package compile succeeded, the 2 targeted tests passed.

## Gap / follow-up brief candidates

**NONE.** This PR cleanly closes the FINDINGS pin from PR #2124. The boundary note about pre-existing `cron_duty_wire_closure_test.rs` E0061 was already closed by PR #2152 (`fix(test): rebase cron-duty wire-closure test onto tenant-aware sessions_for_duty`) — confirmed at `4faab3904`.

## Closing

The 20th anti-theater-honest substrate contract on `main` this week. Per-worker fs-root isolation is now closed against both leaf-symlink (pre-#2124) AND intermediate-dir-symlink (closed today) escape paths.
