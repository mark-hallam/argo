# claude-rebase-pr-1867-revocation — rebase the revocation+middleware PR after #1866 merges

**Owner**: claude. **Recovery target**: PR [#1867](https://github.com/mark-hallam/Argo-Agent/pull/1867). **Original brief**: `streams/mission-revocation-with-verify-middleware-v1x.md`.
**Authored against**: origin/main `70fc980b` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1867-revocation`.
**Priority**: P1 — closes the revocation half of audit P0-1 with verify-middleware enforcement.
**Depends on**: `claude-rebase-pr-1866-cookies` MERGED (which itself unblocks the cookie+CSRF+jti claim half).

## Why this brief exists

PR #1867 (`mission-revocation-with-verify-middleware-v1x`) ships the revoked_tokens table + helpers + logout endpoint + HS256 cookie-JWT verify-middleware as ONE cohesive PR. It's DIRTY against current main because:

1. The sibling cookies-split PR (#1866) hasn't merged yet — this PR may need the `jti` claim that ships in the sibling.
2. Several merges since this PR opened touched routes.rs + the dispatch path (#1864 dashboard-auth, #1869 loc-band wave-2, #1873 wave-3).

The LOC-ceiling waiver from the original brief still applies — files MAY exceed 500 LOC if a clean split would harm cohesion.

## Fix steps

**Wait for `claude-rebase-pr-1866-cookies` to be MERGED before claiming this brief.** Check `git log origin/main --grep="cookies-split.*merged\|claude-rebase-pr-1835-cookies merged"` or the BUILD_STATUS row.

Once #1866 is MERGED:

1. `git fetch origin && git checkout claude-mission-revocation-with-verify-middleware-v1x`
2. `git rebase origin/main`. Expected conflicts:
   - **`crates/argo-cli/src/handlers/gateway/routes.rs`** — dispatch order again. Mount the new `middleware_session_verify` AFTER the bearer-auth gate (from #1863) and the cookies-side middleware (from sibling #1866), BEFORE route dispatch. Exemptions still apply: magic-link verify, signup, WS Ed25519 path.
   - **`crates/argo-cli/src/handlers/gateway/mod.rs`** — module registration union.
   - **`crates/argo-store/src/lib.rs`** — `pub mod revoked_tokens;` re-export union with whatever else has accreted.
   - **`crates/argo-store/src/migrations/mod.rs`** + `registry_compliance.rs` — verify the migration slot the PR uses is still free (the PR should use slot 124 since `123` is taken by `123__session_grant_target.sql`). If slot 124 has accreted into something else, bump to the next free slot.
3. Verify locally:
   - `cargo clippy -p argo-cli -p argo-store -p argo-config --all-targets -- -D warnings` clean
   - `cargo test -p argo-store revoked_tokens` (5 tests) + `cargo test -p argo-cli middleware_session_verify routes_auth_logout` (5 + 4 tests) all pass — load-bearing: the `session_revoked_jti_rejected` case.
4. `git push --force-with-lease origin claude-mission-revocation-with-verify-middleware-v1x`
5. Comment on #1867: "✅ rebased onto current main post-#1866 merge. Migration slot preserved; verify-middleware mounted after bearer auth + cookies middleware."

## Acceptance criteria

- [ ] PR #1867 ends MERGEABLE / non-DIRTY against current main.
- [ ] Migration slot is correct (124 or next free), no duplicate-slot collision.
- [ ] Verify-middleware rejects valid sig + valid exp + revoked jti (load-bearing test passes).
- [ ] All 14 tests pass (5 revoked_tokens + 5 middleware + 4 routes_auth_logout).
- [ ] `cargo clippy -p argo-cli -p argo-store -p argo-config --all-targets -- -D warnings` clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.

## Out of scope

- Cookie/CSRF/Origin middleware — those are owned by sibling #1866.
- Token rotation / refresh tokens — separate v1.x briefs.
- OAuth2/OIDC — separate.
