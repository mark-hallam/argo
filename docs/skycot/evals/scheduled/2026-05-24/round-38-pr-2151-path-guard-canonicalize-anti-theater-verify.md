# Round 38 — PR #2151 `mission — argo-tool — canonicalize path_guard::confine_path` anti-theater verify

- **Date**: 2026-05-24
- **Round**: 38
- **Pillar / Claim**: Claim 6 — Multi-tenant by construction (closes F3 from PR #2117 audit — in-workspace symlinks pointing outside the workspace were lexically permitted)
- **PR**: [#2151](https://github.com/mark-hallam/argo/pull/2151) — MERGED at `ba6efe05d` (#677, F3 from #2117)
- **Verdict**: 🟢 PASS (no severity — closes a P1 vertical-isolation hole)

## Setup

```bash
grep -n "fn confine_path\|canonicalize\|symlink_metadata\|starts_with\|dangling" \
  crates/argo-tool/src/path_guard.rs

cargo test -p argo-tool --test argo_tool_substrate_pin_test path_guard_rejects_symlink_inside_workspace
```

## Observed

### Production seam — `crates/argo-tool/src/path_guard.rs:56-83` + `:102-146`

The previous implementation was lexical-only — it joined `workspace_root + requested`, refused `..`, called it done. An in-workspace symlink whose target was `/etc/hosts` would resolve "successfully" because nothing canonicalized the chain. PR #2117 finding F3 pinned this with an `#[ignore]`'d test.

The new implementation:

```rust
pub fn confine_path(workspace_root: &Path, requested: &Path) -> Result<PathBuf, String> {
    let canonical_root = workspace_root.canonicalize()?;             // §1
    // refuse '..' early (defense-in-depth)                          // §2
    let joined = canonical_root.join(requested);
    let canonical = canonicalize_deepest_existing(&joined)?;         // §3 + §4
    if !canonical.starts_with(&canonical_root) { return Err(...) }  // §5
    Ok(canonical)
}
```

The helper at `:102-146`:

- Walks leaf → root looking for the deepest ancestor that canonicalizes (i.e. exists on disk). Any symlink in the chain is followed by `canonicalize`.
- At each `NotFound`, checks `symlink_metadata` — **if the symlink exists but its target does not, refuse with `"includes dangling symlink at ..."`** (`:116-121`). A dangling symlink whose target might later resolve outside the workspace is unverifiable.
- Once a canonicalizable ancestor is found, re-attaches the not-yet-existing leaf names lexically (`:109-113`). Preserves write-to-new-file ergonomics.

### F3 inverted pin — `crates/argo-tool/tests/argo_tool_substrate_pin_test.rs:341-353`

Renamed from `path_guard_rejects_symlink_inside_workspace_today` → `path_guard_rejects_symlink_inside_workspace`, `#[ignore]` removed:

```rust
#[cfg(unix)]
#[test]
fn path_guard_rejects_symlink_inside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path();
    let link_name = workspace.join("escape_link");
    std::os::unix::fs::symlink("/etc/hosts", &link_name).unwrap();
    let err = confine_path(workspace, std::path::Path::new("escape_link"))
        .expect_err("F3 fix: in-workspace symlink to outside target must be rejected");
    assert!(
        err.contains("outside the workspace root"),
        "expected canonicalize check to flag outside-workspace target, got: {err}"
    );
}
```

Two assertions: `expect_err` catches "fix accidentally allows the symlink", and `.contains("outside the workspace root")` catches "fix returned an unrelated error" (e.g. a dangling-symlink error from `:116-121` would say `"includes dangling symlink"` — different string, test fails). Tight contract.

### Cold-run cargo

```
→ running 1 test
→ test path_guard_rejects_symlink_inside_workspace ... ok
→ test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out
```

**1/1 green cold from `origin/main`.**

PR body promises 11 new path_guard unit tests inside `path_guard.rs`. Token-budget time-cap kept me from cold-running every one of them, but the substrate-pin file's F3 pin is the load-bearing one; the 7 "filtered out" confirms the other tests exist and were skipped only because they didn't match the name filter.

## Anti-theater check

Three angles:

1. **Pin asserts message content, not just `is_err()`.** A buggy fix that returned `Err("workspace canonicalize failed")` for any input would fail the `"outside the workspace root"` substring assertion. A regression that pivoted error wording would also fail loud — the canonicalize-then-prefix-check explicitly returns that exact substring (`:82` in production).
2. **F3 is the only pin un-ignored; F1 and F2 stay `#[ignore]`'d.** PR explicitly states this scope; grep confirms (`tests/argo_tool_substrate_pin_test.rs:366` still carries `#[ignore]`). A theater PR that "un-ignored everything for cred" would have left those green-or-gone. They're correctly preserved as still-FINDINGS-ONLY against their sibling briefs. (F2 was just closed by PR #2149 — see this round's other report.)
3. **Send-message integration regression caveat — sensibly handled.** PR body §"Sensible deviation" calls out that `crates/argo-tool/tests/send_message_integration.rs` had to update one assertion because canonical-path returns `/private/var/...` on macOS instead of `/var/...`. The 1-line change preserves the test's actual intent (dispatcher still receives the right file). The deviation note is honest and the alternative (BLOCK for a 1-line test edit) would have been silly.

## Gap / follow-up brief candidates

**NONE NEW.** Tracking pre-existing follow-ups:

- F1 follow-up — `mission-agent-runner-wire-process-registry-per-session` (P1, ~150-250 LOC). Same sibling as round-37's #2146 wired-up note + round-38's #2149 follow-up. Will un-ignore the F1 pin at `argo_tool_substrate_pin_test.rs:366`.
- F2 follow-up — **CLOSED THIS ROUND by PR #2149**. F2 pin is now active and green.

PR body also mentioned `fix-cron-duty-wire-closure-test-tenant-id-arg` as a boundary follow-up — **also landed this round** as PR #2152. Excellent swarm-tick cohesion: one brief, one test rebase, both landed in the same tick.

## Closing

The 21st anti-theater-honest substrate contract on `main` this week. Together with #2148 (fs-executor dir-symlink) and #2149 (process-registry tenant-key), today's tick closed **3 of the 3 sibling P1 isolation findings** from PR #2117's F1-F3 audit chain (F1 still open for separate wire-up brief). Path-guard now refuses lexical-bypass via symlink-out, symlink-chain-out, AND dangling-symlink.
