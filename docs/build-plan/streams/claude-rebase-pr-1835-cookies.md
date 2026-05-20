# claude-rebase-pr-1835-cookies — split #1835 into cookie+CSRF half (no revocation infra)

**Owner**: claude. **Recovery target**: PR [#1835](https://github.com/mark-hallam/Argo-Agent/pull/1835) (descope + rebase) OR new PR. **Original brief**: `streams/claude-jwt-cookie-security-hardening.md`.
**Authored against**: origin/main `58d3e8a1` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1835-cookies`.
**Priority**: P0 — closes audit P0-1 (web magic-link cookie bridge + CSRF). Half-split of the original PR.

## Why this brief exists (split rationale)

PR #1835 was operator-locked tick-2 to also include the JWT-verify-middleware that consults `revoked_tokens`. Re-examining (2026-05-20):

The 2026-05-17 HOLD review used the word "regression" deliberately: shipping the revoked-tokens table + logout-writes WITHOUT a verify-middleware reading from that table at request-time is a **silent security gap** — operators logout, the table is populated, but tokens still validate until exp because no middleware consults the table. Auditor will catch this in a pen-test.

But the cookie + CSRF + Origin-allowlist hardening in #1835 is **independently valuable** and closes audit P0-1. The structurally honest path is:

- **THIS brief** (`claude-rebase-pr-1835-cookies`) — ships cookies + CSRF + Origin defense + JWT `jti` claim + `auth_magic_link` jti emission. NO `revoked_tokens` table, NO logout endpoint writing to it. Clean win.
- **Sibling brief** (`mission-revocation-with-verify-middleware-v1x`) — ships `revoked_tokens` table + helpers + logout endpoint + verify-middleware + 5 tests **as one cohesive unit**. Lands together or not at all.

This split removes the "infrastructure-without-enforcement window" entirely. Operators never see a half-built revocation surface.

The operator-locked decision from tick-2 is **reversed** by the operator (2026-05-20) in favor of this split.

## Acceptance: descope-and-rebase

### Path A — rebase #1835's branch + descope (preferred — preserves the cookie work)

1. `git fetch origin && git checkout claude-claude-jwt-cookie-security-hardening`
2. **Drop the revocation-related commits**:
   - The slot-116 migration for `revoked_tokens` table
   - `argo-store::revoked_tokens` module (helper, tests)
   - `POST /api/v1/auth/logout` handler (the logout endpoint that writes to revoked_tokens)
   - `GET /api/v1/auth/csrf` if it's only needed alongside logout (probably stays — CSRF mirror is independent)
   - The `revoked_tokens` boundary checks in `auth_magic_link.rs` if any
3. Use `git rebase -i origin/main` to drop the revocation-specific commits, OR cherry-pick only the cookie+CSRF+Origin+jti commits onto a fresh branch from current main.
4. **Keep**:
   - `__Host-session` + `__Host-csrf` cookies (cookies hardening)
   - `middleware_csrf` + `middleware_origin_check` (CSRF defense + Origin allowlist)
   - JWT `jti` claim (uuid v7) on session JWTs — needed by the future revocation brief; benign to land now
   - `auth_magic_link::verify_response` cookie-flag updates
   - SPA-side `client.ts` X-CSRF-Token attachment
5. Resolve conflicts vs main (multiple landings since 2026-05-17: #1827, #1841, others; recent expected hotspot is `crates/argo-config/src/lib.rs` `pub use sections::{...}`).
6. Verify locally: `cargo clippy -p argo-cli -p argo-config --all-targets -- -D warnings` clean; `cargo test -p argo-cli middleware_csrf middleware_origin_check auth_magic_link` pass; `pnpm typecheck && pnpm test` clean in `web/`.
7. `git push --force-with-lease origin claude-claude-jwt-cookie-security-hardening`.
8. Comment on #1835: "✅ rebased + descoped to cookie+CSRF+Origin half. Revocation infra removed and queued as `mission-revocation-with-verify-middleware-v1x` (must land with verify-middleware enforcement). Final PR is ~500-600 LOC."

### Path B — fresh PR

Close #1835. Open `mission-cookie-and-csrf-hardening-v1` against current main, cherry-picking only the cookie+CSRF parts of #1835's history. Lower-risk if Path A's interactive-rebase surface is too complex.

## Acceptance criteria

- [ ] Cookie hardening (`__Host-session` + `__Host-csrf` with proper flags) lands; matches the spec the original PR set out.
- [ ] CSRF defense middleware (`middleware_csrf` double-submit) lands and is mounted in `routes.rs::dispatch_route_inner` before mutating routes.
- [ ] Origin allow-list middleware (`middleware_origin_check`) lands and is mounted.
- [ ] JWT `jti` claim lands on session JWTs (uuid v7).
- [ ] **NO `revoked_tokens` table, NO logout endpoint that writes to it.** That work is now in the sibling brief.
- [ ] SPA-side X-CSRF-Token attachment lands.
- [ ] `cargo clippy -p argo-cli -p argo-config --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli middleware_csrf middleware_origin_check auth_magic_link` pass.
- [ ] `pnpm typecheck && pnpm test && pnpm build` clean in `web/`.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] Every modified file < 500 LOC.

## Notes for the implementer

- The 2026-05-17 HOLD comment + the 2026-05-19 operator-lock + this 2026-05-20 reversal are all preserved on the PR thread for context. Read them; you're descoping per the most-recent operator instruction.
- The `auth_magic_link` jti emission was added in PR #1841 (MERGED). Keep your `jti` work additive — don't fight what's already on main.
- If `argo-config/src/lib.rs` `pub use sections::{...}` is in conflict: union of ALL re-exports (yours + the many that have landed since). This is the long-running hotspot for any rebase against main.

## Out of scope

- `revoked_tokens` table + logout endpoint + verify-middleware — owned by sibling brief `mission-revocation-with-verify-middleware-v1x`.
- The `revoked_tokens` jti-rejection acceptance criterion from the original brief — moved to the sibling.
- Any auth flow changes beyond the cookie + CSRF + Origin + jti scope.
