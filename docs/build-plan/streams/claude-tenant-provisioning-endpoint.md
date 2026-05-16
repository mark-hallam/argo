# claude-tenant-provisioning-endpoint — Provision tenant + seed workspace from signup (Wave 2 P0)

**Owner**: claude. **Depends on**: `claude-magic-link-auth-backend` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-tenant-provisioning-endpoint`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Claim 1 + Claim 2:** Tenant = the multi-tenant boundary that makes "your teammate" coherent. Provisioning is where Claim 1 starts being real for a customer.
- **Self-serve readiness:** without this, magic-link auth dead-ends.

## Scope

P0. New gateway endpoint `POST /api/v1/onboarding/provision`. Authenticated (requires JWT from magic-link verify). Body: `{ "workspace_name": "...", "company_name"?: "..." }`.

On success:
1. Create new `tenant_id` (UUID v7) in `tenant` table.
2. Link `user_account` to `tenant` in `user_tenant` (role=owner).
3. Create `workspace` row inside that tenant.
4. Seed default config: copy `[provider]` block from a "v1-defaults.toml" template (Anthropic via `ANTHROPIC_API_KEY` env, OpenAI optional).
5. Pre-install gtm-engineer vertical pack into the workspace.
6. Return `{ "tenant_id": "...", "workspace_id": "...", "next_step": "/onboarding/connect-provider" }`.

Idempotency: if user already has a tenant, return existing tenant_id with the `next_step` reflecting their current onboarding stage.

Audit: every provisioning event emits an AuditKind::TenantProvisioned audit row (or equivalent).

LOC: ~600.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs` (NEW) — endpoint handler. ≤200 LOC.
- `crates/argo-cli/src/handlers/gateway/onboarding_provision_tests.rs` (NEW) — integration tests (success + idempotent + audit emission). ≤120 LOC.
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: mount route. ≤10 modified lines.
- `crates/argo-identity/src/tenant_provision.rs` (NEW) — `TenantProvision::create(user_id, workspace_name)` — orchestrates tenant + workspace + user_tenant rows. ≤180 LOC.
- `crates/argo-identity/src/lib.rs` — `pub mod tenant_provision;`. ≤5 modified lines.
- `crates/argo-store/migrations/<next>__tenant_provisioned_audit.sql` (NEW) — additive audit kind. ≤10 LOC.
- `crates/argo-audit/src/event.rs` — narrowly: add `AuditKind::TenantProvisioned` variant. ≤20 modified lines.
- `crates/argo-cli/src/handlers/gateway/v1-defaults.toml` (NEW) — default provider/workspace config template. ≤40 LOC.

## Files you must NOT touch

- `crates/argo-identity/src/policy.rs` — out of scope. Existing policy resolution applies to provisioned tenants automatically.
- Magic-link auth code — F-04-shipped, just consume the JWT.
- Vertical packs themselves — pre-installation is a config copy, not a vertical-pack edit.

## Acceptance criteria

- [ ] `POST /api/v1/onboarding/provision` requires valid JWT; returns 401 without.
- [ ] First call from a new user creates tenant + workspace + user_tenant link.
- [ ] Idempotent: second call from same user returns existing tenant.
- [ ] gtm-engineer vertical pack pre-installed (verify via `WorkspaceStore::list_verticals`).
- [ ] Audit row emitted with `AuditKind::TenantProvisioned`.
- [ ] `next_step` field reflects onboarding progress correctly.
- [ ] `cargo test -p argo-cli -p argo-identity -p argo-audit onboarding_provision` passes.
- [ ] `cargo clippy -p argo-cli -p argo-identity -p argo-audit --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-tenant-provisioning-endpoint`
- PR title: `feat(gateway): tenant provisioning + workspace seed (claude-tenant-provisioning-endpoint)`

## Notes for the implementer

- Use UUID v7 for `tenant_id` (existing pattern; check codebase for `Uuid::new_v7`).
- `v1-defaults.toml` template is the seed config — operators can override post-onboarding. Don't bake operator-specific keys; the template should reference env vars (e.g., `ANTHROPIC_API_KEY`).
- Pre-installing gtm-engineer means copying the vertical's manifest into the new workspace's verticals dir. Use `argo-vertical::install` or equivalent.
- After this brief lands, the onboarding wizard (claude-onboarding-wizard) has a target backend.
