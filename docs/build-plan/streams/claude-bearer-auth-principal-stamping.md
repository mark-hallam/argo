# Bearer-auth principal stamping (Phase A follow-up to #946)

**Owner**: codex. **Depends on**: `mission-gateway-bearer-auth-get-routes`
MERGED.

**Authored against**: origin/main `ab1fbc40` on 2026-05-08.

## Conductor Rewrite

Rewritten on 2026-05-08 after worker validation found the original
schema premise stale. Current main has canonical table `audit_record`
(`crates/argo-store/src/migrations/v1.rs`) with no `principal` column;
there is no `audit_log` table. This brief must not add a migration.
Stamp the authenticated principal into `audit_record.payload_json`
instead.

## Why

#946 / #960 wired bearer auth across gateway POST and protected GET
routes. We accept the bearer but currently discard operator identity
metadata on gateway audit rows. Mission claim 1 needs attributable
worker/human actions. For this v1 follow-up, attribution lives in each
gateway audit row's JSON payload.

## Scope

- Parse optional `X-Argo-Principal` after bearer auth succeeds:
  - `worker:<uuid>`
  - `human:<email-or-handle>`
  - `system:<name>`
- Missing header maps to `anonymous`.
- Malformed principal returns `400 Bad Request` with
  `{"error":"bad principal"}`. This is not a 401 because bearer auth
  already succeeded.
- Add `"principal":"..."` into gateway approval audit payloads emitted
  by `POST /sessions/<id>/approve` and
  `POST /sessions/<id>/revoke-approval`.
- Keep the change payload-only. Do not add audit schema columns.

## Approach

1. **NEW** `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs`
   (<= 180 LOC):
   - `Principal` enum: `Worker(Uuid)`, `Human(String)`,
     `System(String)`, `Anonymous`
   - `parse_principal_header(req: &str) -> Result<Principal, &'static str>`
   - `principal_payload(req, payload)` helper that inserts the string
     principal into an existing JSON object
   - unit tests for each accepted variant, missing, malformed, empty,
     and oversized values
2. `routes_dispatch_post.rs`: after `check_bearer_auth(req)` passes,
   validate the principal header once and return the 400 envelope on
   malformed values.
3. `routes_session_approval.rs`: wrap both existing audit payloads
   with the helper before calling `AuditSink::write`.
4. Add a focused gateway smoke test file for the principal payload
   behavior. The old brief referenced nonexistent
   `gateway_auth_smoke.rs`; use a new file instead.

## Files You May Touch

- `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (only if a
  small shared helper is cleaner)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
- `crates/argo-cli/src/handlers/gateway/routes_session_approval.rs`
- `crates/argo-cli/src/handlers/gateway/mod.rs`
- `crates/argo-cli/tests/gateway_auth_principal_smoke.rs` (NEW)

## Files You Must NOT Touch

- `CLAUDE.md`
- `crates/argo-store/src/migrations/`
- `crates/argo-identity/`
- Non-gateway audit writers

## Acceptance Criteria

- [ ] `X-Argo-Principal: worker:<uuid>` parses as a worker principal.
- [ ] Missing header produces `principal = "anonymous"` in payload JSON.
- [ ] Malformed principal returns `400 Bad Request` with
      `{"error":"bad principal"}`.
- [ ] `POST /sessions/<id>/approve` audit payload contains
      `"principal":"worker:<uuid>"` when the header is present.
- [ ] `POST /sessions/<id>/revoke-approval` audit payload contains
      `"principal":"anonymous"` when the header is absent.
- [ ] No migration or audit schema change is added.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli gateway_auth_principal` passes.
- [ ] Every modified file is under 500 LOC.

## Branch + PR

- Branch: `codex-claude-bearer-auth-principal-stamping`
- PR title: `gateway: stamp bearer principal into audit payloads (claude-bearer-auth-principal-stamping)`
