# sprint-ws1-1b — migrate cwd-resolver tranche 1 (billing, approval-bundles, governance)

**Owner**: claude or codex.
**Depends on**: WS1 keystone #2676 (resolver) + WS1-2 #2682 (write paths) — both MERGED. This is the documented WS1-1b long-tail follow-up.
**Authored against**: origin/main `993349e7a` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-1b-migrate-cwd-resolver-tranche-1-billing-approvals-governance`.

## Why (verified 2026-06-05)
The keystone (#2676) migrated the 6 core READ routes and WS1-2 (#2682) the write paths to the
canonical `resolve_tenant::resolve_tenant_for_request` (precedence: JWT `tnt` → explicit header →
cwd/anon fallback). But **18 route files still call the OLD cwd resolver** (`resolve_workspace_tenant(store)`,
which reads tenant from cwd/store, NOT the user's session JWT). On a fully-public multi-tenant launch
a signed-in user hitting those routes is resolved to the shared/cwd (demo) tenant, not their own —
a cross-tenant data exposure.

**Highest-sensitivity of the 18 (this tranche):**
- `crates/argo-cli/src/handlers/gateway/billing.rs` — local `resolve_workspace_tenant` at :191, called at :50 and :95. **Financial: a user's checkout/usage would be scoped to the wrong tenant.**
- `crates/argo-cli/src/handlers/gateway/routes_approval_bundles.rs` — local copy at :283, called :55/:90. Approval data.
- `crates/argo-cli/src/handlers/gateway/routes_governance.rs` — 1 call site. Governance/policy data.

## Task (follow the keystone pattern in #2676 / #2682 — do NOT invent a new approach)
For each of the 3 files:
1. Thread the request into the handler if not already available, and replace `resolve_workspace_tenant(store)`
   with `resolve_tenant_for_request(req, store)` (the canonical resolver). Remove the local duplicate
   `resolve_workspace_tenant` fn once its last caller is migrated (kill the parallel copy).
2. **Per-route check (avoid premise-false):** confirm the route serves USER-scoped data (→ JWT tenant is
   correct). If a route is genuinely admin/system and SHOULD stay cwd, do NOT migrate it — document why
   in the PR body and leave it. Grep the handler's data reads to confirm.
3. Preserve behaviour otherwise (same response shapes, same error handling).

## Verify
- A cross-tenant regression test per migrated route (mirror WS1-4 #2696 / `tests/tenant_resolve_from_session_jwt.rs`):
  tenant A's session reads ONLY A's data on the route, never B's, never cwd/demo; the JWT wins over an
  `X-Argo-Tenant-Id: B` header on an A session. The test must FAIL if the route regresses to cwd.
- `cargo test -p argo-cli -- --test-threads=1` GREEN (serial). clippy clean on touched files
  (boundary-note the pre-existing `await_holding_lock` in `routes_settings_credentials_tests.rs`, untouched).

## Follow-up (NOT this brief — note in PR body)
~15 more cwd-resolver files remain (notifications, retrain, worker-activity, duties, worker-skills,
cohorts, worker-policies, workspace-settings, gtm-draft-followup, marketing-draft, onboarding-helpers) =
WS1-1b tranches 2+. Also: the `SKYCOT_AUTH_JWT_SECRET` fail-closed boot-guard appears absent (only a
test-doc mention) = a separate WS1-1c security brief.

## PR
Title: `sprint(ws1-1b) — tenant — migrate billing/approvals/governance off cwd resolver to JWT`
Keep ≤ LOC ceiling; if it grows, ship billing.rs alone (it's the financial one) and split the rest.
