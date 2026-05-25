# fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch

**Owner**: codex.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`.
**Source**: e2e-ux-audit (onboarding-hire-an-ai-teammate) findings F1 + F3 — `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md`.

🔴 **P0 DEMO-BLOCKER — Hired worker invisible everywhere in the SPA. The whole "I hired Maya and there she is on my team" narrative breaks the moment the operator leaves the onboarding wizard.**

## Root cause

`POST /api/v1/onboarding/first-hire` resolves the workspace tenant via the **principal email**:
- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:251` calls `resolve_workspace(store, &email)` → writes the new worker into the per-principal tenant (e.g. `human:demoer@skycot.com` → `019e613b-9ca2-...`).

But the read paths use a **CWD-derived tenant** (the gateway process's current-working-directory filename), ignoring the principal entirely:
- `crates/argo-cli/src/handlers/gateway/routes_team_list.rs:213` calls `resolve_workspace_tenant(store)` — no email
- `crates/argo-cli/src/handlers/gateway/billing.rs:191` — same pattern
- `crates/argo-cli/src/handlers/gateway/routes_cohorts.rs:207` — same pattern
- Likely affects `GET /api/v1/workers/<id>`, `GET /api/v1/workers/<id>/overview`, and any other reader that resolves a workspace tenant without a principal

So when the wizard's `first-hire` write lands in `<email>`'s tenant, the SPA's `/team` reads from the CWD-derived workspace tenant and the newly-hired worker is invisible. Cross-checked: requesting `?tenant_id=<correct-id>` returns 403 `cross-tenant query not permitted` — the cross-tenant override is correctly fenced, so the only fix is to align the two resolvers.

CWD-derived tenant is a dev convenience that has leaked into product behavior. The single principal-aware resolver pattern in `routes_onboarding::resolve_workspace` is the correct one.

## Curl evidence (from the audit)

```
curl -X POST http://127.0.0.1:7400/api/v1/onboarding/first-hire \
  -H 'X-Argo-Principal: human:smoke@skycot.com' \
  -H 'content-type: application/json' \
  -d '{"description":"Bob the SDR","vertical_id":"sales-sdr"}'
→ {"worker_id":"019e613c-abb5-7733-a945-6d8ae9584ffc","name":"Bob",...}

curl http://127.0.0.1:7400/api/v1/team -H 'X-Argo-Principal: human:smoke@skycot.com'
→ [{"id":"019e5dbb...","name":"Maya"}]   ← Bob is missing
```

## Findings this brief closes

- onboarding-hire-an-ai-teammate F1 [P0] — `first-hire` writes per-email tenant; `/api/v1/team` reads CWD tenant → new hire invisible
- onboarding-hire-an-ai-teammate F3 [P0] — `/workers/{newhire-id}` 404s on `/api/v1/workers/<id>` because the read path looks in the wrong tenant (downstream of F1)

## Fix recipe

1. **Add a single principal-aware tenant resolver** in `crates/argo-cli/src/handlers/gateway/workspace_tenant.rs` (or wherever `resolve_workspace_tenant` lives). Signature:
   ```rust
   pub fn resolve_workspace_tenant_for_principal(
       store: &Store,
       principal: &Principal,
   ) -> Result<TenantId, GatewayError>
   ```
   Behavior: if `principal` is a human email, mirror `routes_onboarding::resolve_workspace(&email)`. If principal is a system actor (bypass / cron / supervisor), fall back to the CWD-derived tenant (preserving today's behavior for non-user paths).

2. **Update all CWD-derived readers** to use the new resolver. Files to touch:
   - `crates/argo-cli/src/handlers/gateway/routes_team_list.rs:213`
   - `crates/argo-cli/src/handlers/gateway/billing.rs:191`
   - `crates/argo-cli/src/handlers/gateway/routes_cohorts.rs:207`
   - `crates/argo-cli/src/handlers/gateway/routes_workers_detail.rs` (verify; likely same pattern)
   - `crates/argo-cli/src/handlers/gateway/routes_workers_overview*.rs`
   - Any other handler grepped via: `rg "resolve_workspace_tenant\\(store\\)" crates/argo-cli/src/handlers/`

3. **Tighten the legacy resolver** to deny when called with a human principal (defense in depth — make the wrong call fail loudly during dev).

4. **Add an integration test** at `crates/argo-cli/tests/onboarding_first_hire_visible_on_team.rs`:
   - Mint a demo principal
   - POST first-hire with a description → capture `worker_id` + `name`
   - GET `/api/v1/team` with the same principal → assert the worker_id appears
   - GET `/api/v1/workers/<id>` → assert 200 with the same worker
   - GET `/api/v1/cohorts` → assert the new worker shows up under its vertical's cohort

## Acceptance criteria

- [ ] `routes_team_list`, `billing`, `routes_cohorts`, and the worker-detail/overview readers all resolve the tenant via the principal-aware function when a human principal is present.
- [ ] `curl -X POST /api/v1/onboarding/first-hire -H 'X-Argo-Principal: human:smoke@skycot.com' -d '<desc>'` followed by `curl /api/v1/team -H 'X-Argo-Principal: human:smoke@skycot.com'` shows the new worker in the team list.
- [ ] `curl /api/v1/workers/<new-worker-id> -H 'X-Argo-Principal: human:smoke@skycot.com'` returns 200 with the worker payload.
- [ ] New integration test `onboarding_first_hire_visible_on_team.rs` passes cold.
- [ ] Existing system/cron paths still resolve via CWD-derived tenant (no regression on supervisor or scheduled-task readers).
- [ ] Gateway schema bump only if new test fixtures require it; otherwise no migration.

## May-touch

- `crates/argo-cli/src/handlers/gateway/workspace_tenant.rs` (or wherever the resolver lives)
- `crates/argo-cli/src/handlers/gateway/routes_team_list.rs`
- `crates/argo-cli/src/handlers/gateway/billing.rs`
- `crates/argo-cli/src/handlers/gateway/routes_cohorts.rs`
- `crates/argo-cli/src/handlers/gateway/routes_workers_detail.rs` (verify)
- `crates/argo-cli/src/handlers/gateway/routes_workers_overview*.rs` (verify)
- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (read-only — pattern to mirror)
- `crates/argo-cli/tests/onboarding_first_hire_visible_on_team.rs` (NEW)

## Out of scope

- Worker-detail error boundary (separate brief: `fix-worker-detail-loading-forever-no-error-boundary`)
- Demo seed CLI for the 5-persona roster (separate brief: `fix-demo-seed-cli-for-real-roster` — already exists)
- Cross-tenant audit cap check in auth-bypass mode (separate brief: `fix-audit-cross-tenant-cap-bypass-mode-behavior`)

## Effort

~150-250 LOC across 5-7 gateway handlers + 1 integration test. ~3-4 hours. This is the single biggest unblock for the v1.0 onboarding narrative — ship first.
