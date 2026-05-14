# claude-rebase-pr-1606 — Rebase #1606 gateway-boot-split-v2 onto current main

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `4164252e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1606`.

## Scope

PR #1606 (`claude-gateway-boot-split-v2`) is a pure refactor splitting `crates/argo-cli/src/handlers/gateway/boot.rs` into `boot.rs` + `boot_helpers.rs` to stay under the 500-LOC ceiling. The PR went DIRTY when other gateway-area PRs landed touching the same `mod.rs` line range.

Pure refactor; behavior-preserving. Rebase, resolve, push.

LOC: ~50 (conflict resolution on `mod.rs` only — `boot.rs` and `boot_helpers.rs` are isolated).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/boot.rs` — PR's modified version preserves.
- `crates/argo-cli/src/handlers/gateway/boot_helpers.rs` — PR's new file; preserves.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — merge `pub mod boot_helpers;` declaration with current mod.rs state (which may have changed due to other route registrations).

## Files you must NOT touch

- Anything outside the original boot.rs split scope.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Branch rebased onto current `origin/main`.
- [ ] `boot.rs` < 500 LOC.
- [ ] `boot_helpers.rs` < 500 LOC.
- [ ] Public surface unchanged (re-exports if needed).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli` passes (focus: gateway boot tests).
- [ ] PR #1606 shows MERGEABLE / CLEAN post-rebase.
- [ ] Force-push to `claude-claude-gateway-boot-split-v2` branch.

## Branch + PR

- Branch (existing): `claude-claude-gateway-boot-split-v2`.
- PR (existing): #1606.

## Notes for the implementer

- Mission-claim alignment: production hardening (file-LOC-ceiling discipline; supports all 6 claims).
- Standard rebase recipe: `gh pr checkout 1606 && git fetch origin main && git rebase origin/main`. Conflict is on `mod.rs` — keep both sides' `pub mod` declarations. `git push --force-with-lease`.
