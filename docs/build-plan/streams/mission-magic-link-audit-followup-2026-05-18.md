# mission-magic-link-audit-followup-2026-05-18 — magic-link P1 findings from 2026-05-18 audit

**Owner**: claude. **Sizing hint**: 3 narrow fixes in the same module.
**Depends on**: `claude-rebase-pr-1796` MERGED (which it is, via #1823).
**Authored against**: origin/main `bc3841a4` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-magic-link-audit-followup-2026-05-18`.
**Source**: 2026-05-18 audit P1 findings #3 + #4 + #5 — three discrete gaps in the magic-link path landed by #1823. Recovery brief explicitly said "do not expand scope"; worker correctly resisted. These are the deferred follow-ups.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — multi-tenant correctness (#3) and brute-force resistance (#4).
- **Observability** (#2) — auth flow must emit audit rows (#5); "every action is on this page" is the demo-script promise.

## Scope

P1. Three discrete fixes, all in the same `auth_magic_link.rs` module:

### Fix 1: `tenants.pop()` → `.into_iter().next()` (binds NEWEST, not oldest)

`crates/argo-cli/src/handlers/gateway/auth_magic_link.rs:149` currently:

```rust
Ok(mut tenants) => tenants.pop().map(|t| t.tenant_id.to_string()),
```

`list_tenants_for_user` returns rows `ORDER BY created_at DESC` (`crates/argo-identity/src/user_tenant_store.rs:67`). `.pop()` returns the LAST element = the OLDEST tenant. For multi-tenant users this binds the JWT to the wrong tenant.

Fix:

```rust
Ok(tenants) => tenants.into_iter().next().map(|t| t.tenant_id.to_string()),
```

Add a unit test: insert user with 2 tenants (older + newer), call verify, assert returned tenant_id is the NEWER one.

### Fix 2: Rate-limit on `/verify` endpoint

`auth_magic_link.rs:121-169` currently has zero IP throttling. The 32-byte token space is practically un-brute-forceable but online enumeration + DoS is still a real surface.

Add: 60/min/IP sliding window guard on `/verify`. Reuse the same `DashMap<String, Vec<Instant>>` pattern as signup (`auth_magic_link.rs:294-303`) — a separate `VERIFY_BUCKETS` map; do NOT collide with signup buckets.

### Fix 3: Audit emission on successful verify

`auth_magic_link.rs` currently emits zero audit rows on success — only `tracing::warn!`. Add:

```rust
audit_sink.emit(AuditEvent::AuthSucceeded {
    method: "magic_link",
    user_id,
    tenant_id,
    source_ip,
}).await?;
```

at the success point post-tenant-resolution (around line 151). Requires extending `argo_audit::event::AuditKind` if `AuthSucceeded` doesn't exist yet — check first (`grep "AuthSucceeded\\|AuthEvent" crates/argo-audit/`). If absent, add as a new variant (mirrors the pattern in #1819).

Also emit on FAILURE (wrong token, expired token, already-used token) for forensic visibility — `AuditEvent::AuthFailed { method, reason, source_ip }`.

LOC: ~200 (fixes + tests + possible new audit variant).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/auth_magic_link.rs` — fixes 1, 2, 3. ≤80 modified LOC.
- `crates/argo-cli/src/handlers/gateway/auth_magic_link_tests.rs` — add 3 new tests. ≤80 modified LOC.
- `crates/argo-audit/src/event.rs` — add `AuthSucceeded` + `AuthFailed` variants IF needed (verify by grep first). ≤30 modified LOC.
- `crates/argo-audit/src/sink.rs` — minor wiring if a new variant requires schema fields. ≤10 modified LOC.

## Files you must NOT touch

- The migration slot 115 schema (`magic_link_tokens.sql`) — fixes are runtime-only.
- The `user_tenant_store.rs` query — fix is at the call-site, not in the store.
- The SPA verify route (`web/app/api/auth/verify/route.ts`) — gateway-only fix.
- Other audit emit sites — don't drift this brief's scope to a global audit-coverage sweep.

## Acceptance criteria

- [ ] `tenants.into_iter().next()` returns the NEWEST tenant; unit test verifies 2-tenant case.
- [ ] POST `/api/v1/auth/verify` rate-limits at 60/min/IP; 61st attempt within 60s returns 429.
- [ ] Successful verify emits `AuthSucceeded` audit row with `(user_id, tenant_id, source_ip)`.
- [ ] Failed verify (3 failure modes: wrong token, expired, already-used) emits `AuthFailed` audit row.
- [ ] `cargo test -p argo-cli auth_magic_link` all pass.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-magic-link-audit-followup-2026-05-18`
- Title: `fix(gateway): magic-link audit follow-up — tenant pick + verify rate-limit + audit emission (mission-magic-link-audit-followup-2026-05-18)`
- Worker verification: crate-scoped clippy + tests; manual smoke against the 3 cases.

## Hidden constraints

- **Don't break tonight's demo path.** Single-user signup-then-verify on a single tenant still works after these fixes (the demo only has one tenant, so `.pop()` vs `.into_iter().next()` is irrelevant in practice).
- **Audit variant addition follows #1819 pattern.** If new `AuthSucceeded` / `AuthFailed` variants are added, ensure the migration registry + chain-hash computation are not impacted (`AuditKind` enum-only changes per #1819 precedent).
- **Per-IP buckets for /verify must NOT share with signup buckets.** Different policies; different DashMap.
- **Closes 2026-05-18 audit P1 #3, #4, #5.**
