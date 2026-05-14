# claude-rebase-pr-1657 — Rebase #1657 argo-delegate triage onto current main

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `4164252e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1657`.

## Scope

PR #1657 (`claude-argo-delegate-triage` Path A delete) was opened CLEAN earlier but went DIRTY when #1656 (`claude-toml-inherit-workspace-versions`) landed and rewrote 236 workspace Cargo.toml lines — colliding with #1657's workspace-member removal of `crates/argo-delegate`. Audit reports `docs/skycot/audit/2026-05-14-phase-A-crate-inventory.md` and `phase-D-cross-cutting-drift.md` may also conflict (other PRs annotated them).

The triage decision is sound — Path A (DELETE argo-delegate): the crate is unused, audit Phase A's claim-2 mapping was an error. This brief: rebase the branch onto current main, resolve conflicts (workspace Cargo.toml + audit annotations), force-push.

LOC: ~50 (mostly conflict resolution).

## Files you may touch

- `crates/argo-delegate/` — DELETE (Path A, preserve original PR intent).
- Workspace `Cargo.toml` — remove `argo-delegate` from `[workspace.members]`. Merge with current state from #1656.
- `Cargo.lock` — re-resolve.
- `docs/skycot/audit/2026-05-14-phase-A-crate-inventory.md` — preserve original PR's audit annotation about delegate.
- `docs/skycot/audit/2026-05-14-phase-D-cross-cutting-drift.md` — same.

## Files you must NOT touch

- Code outside argo-delegate (the original triage scope is delete-only).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Branch rebased onto current `origin/main` (no merge commit; clean linear).
- [ ] `crates/argo-delegate/` directory absent post-rebase.
- [ ] Workspace `Cargo.toml` `[workspace.members]` does not list argo-delegate AND retains all #1656 workspace-inheritance changes.
- [ ] `cargo build --workspace` clean.
- [ ] PR #1657 shows MERGEABLE / CLEAN post-rebase.
- [ ] Force-push to `codex-claude-argo-delegate-triage` branch.

## Branch + PR

- Branch (existing): `codex-claude-argo-delegate-triage` — DO NOT create new branch; rebase in place.
- PR (existing): #1657.

## Notes for the implementer

- Mission-claim alignment: production hardening (eliminates dead workspace member).
- Standard rebase recipe: `gh pr checkout 1657 && git fetch origin main && git rebase origin/main`. Resolve conflicts (take both sides on workspace Cargo.toml — remove argo-delegate AND preserve #1656 workspace-inheritance changes). `git push --force-with-lease`.
