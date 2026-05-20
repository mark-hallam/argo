# claude-rebase-pr-1866-cookies — rebase the cookies-split PR against current main

**Owner**: claude. **Recovery target**: PR [#1866](https://github.com/mark-hallam/Argo-Agent/pull/1866). **Original brief**: `streams/claude-rebase-pr-1835-cookies.md`.
**Authored against**: origin/main `70fc980b` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1866-cookies`.
**Priority**: **P0** — closes audit P0-1 cookie+CSRF half. Unblocks #1867 revocation+middleware (the dependent split).

## Why this brief exists

PR #1866 (`claude-rebase-pr-1835-cookies`) is the cookie + CSRF + Origin + JWT-jti hardening half of the operator's #1835 split. It went DIRTY when #1864 (gateway-dashboard-auth-gate) merged + moved dashboard mounts to after the bearer-auth gate in `routes.rs::dispatch_route_inner`. Subsequently #1869 (loc-band wave-2 refactor) also touched routes-dispatch siblings, and #1873 (wave-3) split more files in the same area.

The PR's cookie/CSRF middleware mounts are still semantically correct — they just need to be re-applied in their new positions in the refactored dispatch path.

## Fix steps

1. `git fetch origin && git checkout claude-claude-jwt-cookie-security-hardening`
2. `git rebase origin/main` (against `70fc980b` or newer). Expected conflict surfaces:
   - **`crates/argo-cli/src/handlers/gateway/routes.rs`** — multiple merges have moved dispatch order; resolve by preserving:
     - **Existing bearer-auth gate** (from #1863 + #1864): mounted at the top before dashboards
     - **This PR's middleware_csrf + middleware_origin_check + middleware_session_set_cookie mounts**: must mount AFTER bearer auth gate but BEFORE the route dispatch. Mirror the pattern of #1864's dashboard-after-auth approach.
   - **`crates/argo-cli/src/handlers/gateway/mod.rs`** — module registration accretion. Take union.
   - **`crates/argo-cli/src/handlers/gateway/auth_magic_link.rs`** — the jti claim emission. Should still apply cleanly; if conflict, preserve both this PR's jti emission and any other refactoring on main.
   - **`crates/argo-config/src/lib.rs`** — pub-use re-exports; long-standing accretion hotspot. Union of all.
3. Verify locally:
   - `cargo clippy -p argo-cli -p argo-config -p argo-channels --all-targets -- -D warnings` clean
   - `cargo test -p argo-cli middleware_csrf middleware_origin_check auth_magic_link` pass
   - `pnpm typecheck && pnpm test` in `web/`
4. `git push --force-with-lease origin claude-claude-jwt-cookie-security-hardening`
5. Comment on #1866: "✅ rebased onto current main; routes.rs dispatch order preserved; middleware mounts re-applied after bearer-auth gate."

## Acceptance criteria

- [ ] PR #1866 ends MERGEABLE / non-DIRTY against current main.
- [ ] Cookie + CSRF + Origin + jti hardening preserved (no scope reduction beyond the original split).
- [ ] `cargo clippy -p argo-cli -p argo-config -p argo-channels --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli middleware_csrf middleware_origin_check auth_magic_link` pass.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] `pnpm typecheck && pnpm test && pnpm build` in `web/` clean.

## Out of scope

- Adding the revoked_tokens table or verify-middleware — those are owned by the sibling brief `mission-revocation-with-verify-middleware-v1x` (PR #1867, DIRTY pending this brief MERGED).
- Any functional changes beyond the rebase.
