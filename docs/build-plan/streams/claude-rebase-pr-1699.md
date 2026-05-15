# claude-rebase-pr-1699 — Rebase #1699 L14-skill-ir-composition onto current main

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main HEAD on 2026-05-15.

## Scope

PR #1699 (L14-skill-ir-composition) went DIRTY when sister L-track PRs merged ahead of it. Conflicts on ir/mod.rs + types.rs (L11/L12 added neighboring entries) — each PR added neighboring `pub mod` declarations + types.rs variants.

## Files you may touch

- All files in original PR #1699
- Additive merge: keep both sides' `pub mod` declarations + variants.

## Acceptance criteria

- [ ] PR #1699 shows MERGEABLE / CLEAN after force-push.
- [ ] crates/argo-skill cargo test passes.

## Branch + PR

- Branch (existing): `codex-L14-skill-ir-composition`
- PR (existing): #1699

## Notes

Standard rebase: `gh pr checkout 1699 && git fetch origin main && git rebase origin/main`. Additive on mod.rs + types.rs. `git push --force-with-lease`.
