# claude-rebase-pr-1697 — Rebase #1697 L11-cu-recording-to-ir onto current main

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main HEAD on 2026-05-15.

## Scope

PR #1697 (L11-cu-recording-to-ir) went DIRTY when sister L-track PRs merged ahead of it. Conflicts on ir/mod.rs (L14 added pub mod composition simultaneously) — each PR added neighboring `pub mod` declarations + types.rs variants.

## Files you may touch

- All files in original PR #1697
- Additive merge: keep both sides' `pub mod` declarations + variants.

## Acceptance criteria

- [ ] PR #1697 shows MERGEABLE / CLEAN after force-push.
- [ ] crates/argo-skill cargo test passes.

## Branch + PR

- Branch (existing): `codex-L11-cu-recording-to-ir`
- PR (existing): #1697

## Notes

Standard rebase: `gh pr checkout 1697 && git fetch origin main && git rebase origin/main`. Additive on mod.rs + types.rs. `git push --force-with-lease`.
