# sprint-ws1-3-auto-provision-tenant-on-verify

**Owner**: claude or codex.
**Depends on**: #2676 (MERGED — JWT tenant resolver). Complements WS1-2 (#2682, writes → user tenant).
**Authored against**: origin/main `d699a56c8` on `2026-06-04` (verified: `provision_for_user` @ `crates/argo-identity/src/tenant_provision.rs:86`; verify handler @ `crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs`; provision handler @ `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs:52`).
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-3-auto-provision-tenant-on-verify`.

## Why

The keystone (#2676) resolves a signed-in user to their own tenant *from the JWT* — but a
freshly-verified user who has NOT yet provisioned a tenant has no `tenant_id` to resolve, so
the resolver falls back to the cwd/demo tenant. Today `auth_magic_link_verify.rs` returns
`needs_onboarding: true` and relies on the SPA to call `POST /api/v1/onboarding/provision`
before any data page — but if the SPA ever reads a data route first (or a user deep-links),
they momentarily see the shared demo tenant. Closing this guarantees every authenticated
session maps to the user's OWN tenant from the first request.

## What

Pick ONE of these (decide + justify in the PR body; option A is preferred):

- **A — provision-on-verify:** when `auth_magic_link_verify` succeeds for a user with no
  tenant, call `provision_for_user` right there so the minted `__Host-session` JWT already
  carries the real `tenant_id`. Idempotent (re-verify must not duplicate).
- **B — hard-gate the SPA:** make every authenticated data route 401/redirect-to-onboarding
  until the user has a provisioned tenant, so a data read can never resolve to cwd.

Re-verify the verify-handler + JWT-mint flow at claim time (`rg 'needs_onboarding|provision_for_user|mint' crates/argo-cli/src/handlers/gateway/auth_magic_link*`).

## May-touch
- `crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs` (+ the JWT mint path) OR the data-route gate, depending on A vs B.
- `crates/argo-identity/src/tenant_provision.rs` only if a helper tweak is needed (it's already idempotent).
- `crates/argo-cli/tests/` — a test proving a freshly-verified user's first data read resolves to THEIR tenant, not cwd/demo.
- `docs/build-plan/BUILD_STATUS.md` (ledger flip).

## Acceptance criteria
- [ ] A user who verifies and has no prior tenant ends up with a provisioned tenant whose `tenant_id` is in their session JWT (option A) — OR cannot reach any data route until provisioned (option B).
- [ ] Integration test: a freshly-verified user's first `/api/v1/team` (or any migrated route) resolves to their own (empty) tenant, never the cwd/demo tenant.
- [ ] Idempotent: verifying twice does not create two tenants / error.
- [ ] `cargo clippy` clean on touched crates; tests GREEN (report pass lines).
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`.

## Out of scope
- The fresh-tenant baseline seed (WS2-1) and the write-path migration (WS1-2, #2682).

## Parallel-surface check
- `rg 'fn provision_for_user' crates/` — reuse the existing idempotent provision; no parallel provisioner.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS1-3).
