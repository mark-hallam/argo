# claude-rebase-pr-1632 — Rebase #1632 L6 ghostwriter onto current main (UNBLOCKS L-track Phase 1)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `4164252e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1632`.

## Scope

PR #1632 (`L6-ghostwriter-prompt-to-ir`) is the **last L-track Phase 1 brief still in flight**. Once it merges, L7 + L8 + the Phase 2 chain unblocks. The PR went DIRTY when other gateway-route PRs landed (likely #1640 sso-oidc-jwt + #1650 ci-lint-parallel-pub-types both touched `crates/argo-cli/src/handlers/gateway/mod.rs` or `routes_dispatch_post.rs`).

The L6 work is sound — prompt→IR pipeline with offline ghostwriter + gateway route + frontend page. The brief: rebase the branch, resolve gateway routing conflicts, force-push.

LOC: ~100 (mostly conflict resolution on gateway mod.rs + routes_dispatch_post.rs).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/mod.rs` — merge route registration with current state. Both sides win (preserve all existing route registrations + add ghostwriter route).
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — merge POST route dispatch. Same pattern.
- `crates/argo-cli/src/handlers/gateway/routes_ghostwriter.rs` — PR's new file; preserve.
- `crates/argo-skill/src/ir/ghostwriter.rs`, `ghostwriter_prompts.rs`, `ghostwriter_tests.rs`, `mod.rs` — PR's new files; preserve.
- `web/app/(app)/workers/[workerId]/skills/new/page.tsx`, `web/components/skycot/canvas/GhostwriterForm.tsx`, `web/lib/api/ghostwriter.ts` — PR's new files; preserve.

## Files you must NOT touch

- Anything outside the original L6 brief scope.
- New files added by other PRs unrelated to ghostwriter routing.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Branch rebased onto current `origin/main`.
- [ ] All ghostwriter functionality preserved.
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `pnpm typecheck` clean.
- [ ] PR #1632 shows MERGEABLE / CLEAN post-rebase.
- [ ] Gateway routes_dispatch_post.rs registers ghostwriter route AND retains every route from prior PRs.
- [ ] Force-push to `claude-L6-ghostwriter-prompt-to-ir` branch.

## Branch + PR

- Branch (existing): `claude-L6-ghostwriter-prompt-to-ir`.
- PR (existing): #1632.

## Notes for the implementer

- Mission-claim alignment: claim 4 (Skills shared+learned — Ghostwriter is the gen-from-prompt → IR pipeline).
- This is the **critical Phase 1 unblock** for Track L. Phase 2-4 cascade hinges on this.
- Standard rebase recipe: `gh pr checkout 1632 && git fetch origin main && git rebase origin/main`. The gateway routing files (`mod.rs` + `routes_dispatch_post.rs`) need additive merge — keep both old + new routes. `git push --force-with-lease`.
