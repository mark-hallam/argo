# claude-rebase-pr-1756 — recover #1756 claude-magic-link-auth-backend (P0 SHIP)

**Owner**: claude. **Recovery target**: PR [#1756](https://github.com/mark-hallam/Argo-Agent/pull/1756). **Original brief**: `streams/claude-magic-link-auth-backend.md`.
**Authored against**: origin/main `da8e0e0e` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1756`.
**Priority**: 🎯 **P0** — blocks `claude-jwt-cookie-security-hardening` + email-template consumer wire-up + `mission-signup-magic-link-spam-protection` (in TODO).

## Why this brief exists

PR #1756 was opened by a prior worker on 2026-05-16T13:07 and stayed DIRTY/CONFLICTING for ≥18h. Worker did not rebase despite two rebase comments on the PR. Per protocol stale-IN_REVIEW sweep at 24h: author a `claude-rebase-pr-<N>` recovery brief so a fresh worker can pick this up cleanly.

## Acceptance: pick ONE recovery path

### Path A — rebase the existing branch (preferred if the branch is salvageable)

1. `git fetch origin && git checkout claude-claude-magic-link-auth-backend`
2. `git rebase origin/main` — resolve conflicts. Likely surface: argo-cli routes_auth.rs (other auth-adjacent merges may have shifted it) + argo-store migrations (slots 106-109 added since #1756 opened).
3. Re-run worker verification per the original brief: `cargo test -p argo-cli -p argo-store`, `cargo clippy -p argo-cli -p argo-store --all-targets -- -D warnings`, `bash scripts/codex-pre-push.sh` or equivalent.
4. `git push --force-with-lease origin claude-claude-magic-link-auth-backend`.
5. Comment on PR #1756: "✅ rebased + force-pushed; verification clean. Ready for admin-merge."

### Path B — fresh start (use if conflict resolution would be more invasive than reimplementing)

1. Read the original brief at `streams/claude-magic-link-auth-backend.md` end-to-end.
2. Re-validate that its premise still holds against current origin/main (substrate-tables now MERGED — `tenant`, `user_account`, `user_tenant` tables exist; consume them).
3. Implement from scratch on a fresh branch `claude-magic-link-auth-backend-v2`.
4. Open a NEW PR.
5. Comment on #1756: "Superseded by #<new-PR>; closing." Close #1756.

**Picking the path:** if `git rebase origin/main` resolves cleanly in <30 min, Path A. If conflicts span >5 files OR you're not confident the rebase preserves intent, Path B.

## Files you may touch

Whatever the original brief's may-touch list permits. If Path B, recheck that may-touch against current main first (substrate has changed since the brief was authored).

## Acceptance criteria

- [ ] PR ends MERGEABLE / non-DIRTY against current origin/main.
- [ ] All worker-side verification clean (cargo test, cargo clippy, no behavior regression).
- [ ] BUILD_STATUS.md row for `claude-magic-link-auth-backend` flipped from TODO back to IN_REVIEW with the recovery PR number.
- [ ] If Path B: original PR #1756 closed with the supersedes comment.

## Hidden constraints

- **Don't `git push --force` without `--with-lease`** — protects against clobbering a worker's concurrent push.
- **`/tmp/claude-magic-link-auth-backend/` worktree** may exist from the prior worker; either inherit it (`cd` in, fetch, rebase) or `git worktree add` a fresh one if the old one is broken.
- **Magic-link tokens table substrate** — substrate-tables (#1777) added `tenant` + `user_account` + `user_tenant`. Magic-link may want to ATTACH a new token table (e.g. `magic_link_token` at migration slot 110) rather than the original brief's approach. Worker decides; document choice in PR body.
- **JWT signing key** — argo-config already has signing-key handling; reuse rather than introducing a parallel surface.
- **Email send** — argo-email-templates crate exists post-#1766. Use `MagicLinkEmail` template; don't hand-roll the email body.
