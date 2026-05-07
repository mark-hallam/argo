# Bearer-auth principal stamping (Phase A follow-up to #946)

**Owner**: claude. **Depends on**: `mission-gateway-bearer-auth-get-routes` MERGED
(creates `routes_auth.rs`).

## Why

#946 wired `check_bearer_auth` across POST routes and a sibling stream is
extending it to GET routes via `routes_auth.rs`. We accept the bearer but
discard everything we *learn* from it — every audit row reads
`principal=anonymous` even when the request was authenticated. Mission
claim 1 ("teammate, not tool. A worker has persistent identity, voice,
memory") demands every action be attributable. This brief adds principal
stamping: when `X-Argo-Principal` accompanies a valid bearer, parse it and
write it onto the audit row that the request produces.

## Scope

- Parse `X-Argo-Principal: worker:<uuid>` / `human:<email>` / `system:<name>`
  out of the request after bearer succeeds.
- Pass the parsed `Principal` through the dispatch path so the audit row
  gets stamped instead of the default `anonymous`.
- Schema: extend the audit row writer to accept an optional principal
  string (column already exists in `audit_log`; verify before adding).
- Reject malformed principal values with 400 (not 401 — bearer was good).

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs`
   (~120 LOC):
   - `Principal` enum: `Worker(WorkerId)` / `Human(String)` / `System(String)` / `Anonymous`
   - `pub fn parse_principal_header(req: &str) -> Result<Principal, &'static str>`
   - 8 unit tests in same file (well-formed each variant, malformed, missing,
     empty, oversized).
2. **Modify** `routes_auth.rs` (created by parent brief) — narrowly:
   on bearer-auth success, also call `parse_principal_header` and pass
   the result through the request context tuple (existing pattern in
   `routes_dispatch_post.rs`).
3. **Modify** audit row writer site(s) — pass `principal.as_str()` into
   the existing column rather than hard-coded `"anonymous"`.
4. Add 1 integration test in `crates/argo-cli/tests/gateway_auth_smoke.rs`
   covering: header present → audit row has principal; header absent →
   audit row has `anonymous`; malformed → 400.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (narrow edit — 1 call site)
- `crates/argo-cli/src/handlers/gateway/mod.rs` (1 pub mod line)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` (narrow — thread principal arg)
- Audit row writer site(s) the request path actually reaches (≤ 2 files;
  prefer the smallest possible diff)
- `crates/argo-cli/tests/gateway_auth_smoke.rs` (existing — add 3 cases)

## Files you must NOT touch

- The `argo-store` migrations directory — `audit_log.principal` column
  already exists; verify with `grep -n principal crates/argo-store/migrations/*audit*`
  before assuming. If it does NOT exist, STOP and surface to conductor;
  do not add migration in this brief.
- `crates/argo-identity/` types
- Any non-audit-row writer

## Acceptance criteria

- [ ] `X-Argo-Principal: worker:<uuid>` parses to `Principal::Worker(_)`
- [ ] Malformed principal → 400 with body `{"error":"bad principal"}`
- [ ] Missing header → `Principal::Anonymous` (current behaviour preserved)
- [ ] Audit row produced by an authenticated POST has `principal` column
      populated, not the literal string `"anonymous"`
- [ ] 8 unit tests + 3 integration cases
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-bearer-auth-principal-stamping`
- PR title: `gateway: stamp principal from X-Argo-Principal onto audit rows (claude-bearer-auth-principal-stamping)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies to gateway handler files.
- This brief is intentionally PURE READ + STAMP. No new audit kinds, no
  policy changes, no rate-limit logic. If you find yourself touching
  more than ~5 files, stop and re-scope.
- Block start: read #946 in full before modifying. The pattern there is
  load-bearing.
