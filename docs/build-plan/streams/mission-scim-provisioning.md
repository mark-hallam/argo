# Mission load-bearing wire — SCIM 2.0 provisioning endpoint

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #231. Enterprise IdPs (Okta / Azure AD / Google Workspace) expect SCIM 2.0 endpoints to provision/deprovision users automatically. Today: operators manually create operator records.

## Scope

NEW `argo-cli` gateway routes implementing SCIM 2.0 core schema:

1. **`GET /scim/v2/Users`** — list users; supports `?filter=userName eq "alice"`, `?startIndex`, `?count`
2. **`POST /scim/v2/Users`** — provision user (creates operator + maps SCIM `userName` → operator id)
3. **`GET /scim/v2/Users/<id>`** — read
4. **`PATCH /scim/v2/Users/<id>`** — partial update (active = false → deactivates)
5. **`DELETE /scim/v2/Users/<id>`** — deprovision (soft-delete; preserves audit-chain integrity)
6. **`GET /scim/v2/Groups`** + CRUD (optional in v1; behind feature flag if scope is too big)

Auth: SCIM endpoints require a separate bearer token (`ARGO_SCIM_BEARER_TOKEN`) distinct from `ARGO_GATEWAY_BEARER_TOKEN` so the IdP gets its own scoped credential.

This is NOT in scope: bulk operations (`/scim/v2/Bulk` — v2); password sync (we delegate to IdP); custom extension schemas (use canonical core schema only).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_scim_users.rs` (NEW, ≤ 480 LOC) — User CRUD
- `crates/argo-cli/src/handlers/gateway/routes_scim_users_tests.rs` (NEW, sibling, ≤ 480 LOC) — 10 tests
- `crates/argo-cli/src/handlers/gateway/routes_scim_groups.rs` (NEW, ≤ 350 LOC) — Group CRUD (optional)
- `crates/argo-cli/src/handlers/gateway/routes_scim_groups_tests.rs` (NEW, sibling, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` + `routes_dispatch_post.rs` + `routes_dispatch_patch.rs` (if exists) — narrowly scoped: SCIM route arms
- `crates/argo-cli/src/handlers/gateway/scim_auth.rs` (NEW, ≤ 120 LOC) — SCIM bearer auth gate
- `argo-config/src/sections.rs` — `[scim]` config section
- `crates/argo-store/migrations/NEXT__scim_provisioned_tombstones.sql` (NEW; check slot at claim time, likely 71+) — tracks deprovisioned-by-SCIM users
- `docs/skycot/RUNBOOK.md` — "SCIM setup" with per-IdP snippets

## Files you must NOT touch

- Existing operator-creation paths (`argo operator add`); the audit chain; non-SCIM auth gates

## Acceptance criteria

- [ ] All 5 User endpoints respond with SCIM 2.0 envelope shapes (per RFC 7643/7644)
- [ ] Filter `userName eq "..."` works
- [ ] Deactivation (`active = false`) prevents future auth; existing audit rows preserved
- [ ] Hard-delete handled as soft-delete with a `scim_provisioned_tombstones` row
- [ ] Auth: `ARGO_SCIM_BEARER_TOKEN` required; distinct from gateway bearer
- [ ] 10 unit tests + 1 integration test (provision → deactivate → reactivate)
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-scim-provisioning`
- PR title: `gateway: SCIM 2.0 user-provisioning endpoints (mission-scim-provisioning)`

## Notes

- SCIM is well-specified — read RFC 7644 §3 (Resources) BEFORE writing. The envelope shape is load-bearing (`schemas: ["urn:ietf:params:scim:schemas:core:2.0:User"]`).
- The separate bearer token is the key isolation pattern — if the IdP's credential leaks, the blast radius is provisioning only, not the full gateway.
- Soft-delete pattern: SCIM's contract says DELETE removes the resource; we satisfy the contract while preserving audit-chain references via tombstones.
