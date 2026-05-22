# Hotfix — `unused_assignments` cleanup on self_update.rs PEM parser

**Owner**: claude. **Depends on**: PR #1262 (claude-codex-self-update-cli) still open as DRAFT.
**Authored against**: origin/main `f4c7763a` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh hotfix-self-update-cli-unused-assignment`.

> **HOTFIX — DRAFT-PR cleanup.** PR #1262 was salvaged from a quota-stranded worker yesterday and pushed as DRAFT because it has a single `unused_assignments` clippy warning. The substantive work (ed25519 signature verify + tar extract + self-update CLI) is sound and reviewed. This brief is the 1-line clippy fix that unblocks marking the PR ready and merging.

## Scope

`crates/argo-cli/src/handlers/self_update.rs:264` has `inside = false;` set immediately before `break;`. The value is never read after assignment — `unused_assignments` warning under `-D warnings`. Fix: drop the assignment.

```rust
        if line == PUBKEY_FOOTER {
            inside = false;   // ← drop this line
            break;
        }
```

After the fix:
1. Run `cargo clippy -p argo-cli --all-targets -- -D warnings` to confirm clean.
2. Push to the `claude-codex-self-update-cli` branch with `--force-with-lease`.
3. Mark PR #1262 as ready-for-review (drop the `[WIP]` prefix + un-draft via `gh pr ready 1262`).

This is NOT in scope: any other change to self_update.rs; touching other files; opening a new PR.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/self_update.rs` — narrowly scoped: ONE line removal (the `inside = false;` on line 264 or near it; line number may drift if branch has been rebased — find the `if line == PUBKEY_FOOTER { ... break; }` block).

## Files you must NOT touch

- Any other file in the PR
- Any test file
- Any file outside the PR

## Acceptance criteria

- [ ] The `inside = false;` line immediately before `break;` in the PUBKEY_FOOTER arm is removed
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean (this specific warning resolves)
- [ ] `cargo test -p argo-cli` passes (skip pre-existing flakes)
- [ ] PR #1262 pushed with `--force-with-lease` to `claude-codex-self-update-cli`
- [ ] PR marked ready-for-review (`gh pr ready 1262`)
- [ ] BUILD_STATUS.md row for `codex-self-update-cli` flipped to IN_REVIEW (#1262)

## Branch + PR

- Branch: `claude-codex-self-update-cli` (existing — no new branch)
- PR: existing #1262 (no new PR)

## Notes for the implementer

- This is the absolute smallest possible fix. Resist the urge to "while I'm here" any other improvements — diff should be `-1 line, +0 lines`.
- The `let mut inside = false;` declaration earlier in the function is still needed (used by the `inside = true;` on entry to the PUBKEY_HEADER block). Don't touch that.
- If the line number has drifted from a rebase, search for the exact pattern `inside = false;\n.*break;` to find it.
- Once PR #1262 lands, the brief row in BUILD_STATUS.md for `codex-self-update-cli` flips MERGED via standard conductor merge flow. The hotfix row itself flips MERGED at the same time (this is a cleanup-brief whose deliverable IS the PR push).
