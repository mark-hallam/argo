# fix-gateway-demo-mode-worker-permissions-grant-all

**Owner**: codex.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-demo-mode-worker-permissions-grant-all`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/scene-1-and-2-claim-1-and-2.md` F11 [P1].

🟠 **P1 — Every demo worker overview returns `permissions.can_*: false`. Every interactive control on the worker profile is disabled. Camera rolls, narrator says "watch us pause her", FreezeButton is greyed. Edit charter, edit duties, archive, view audit, view settings — all dead.**

## Root cause

`GET /api/v1/workers/<id>/overview` returns a `permissions` object computed by `crates/argo-acp/src/handlers/workers.rs` (look for `compute_permissions` or similar). On a demo-mint principal, the function returns:
```json
{
  "can_edit_charter": false,
  "can_edit_duties": false,
  "can_pause_resume": false,
  "can_archive": false,
  "can_delete": false,
  "can_view_audit": false,
  "can_view_settings": false
}
```

The SPA layout server-fetches these and gates buttons/tabs on them — `FreezeButton`, `DutiesCard` toggles, the schedule + edit-charter affordances. So in demo mode, the entire interaction surface is read-only.

This is correct enterprise behavior for tenants where the demo principal hasn't been assigned a role (defense-in-depth). But for the v1.0 demo MP4, the recording principal needs the delegator-equivalent role on every demo-tenant worker — otherwise Scene 2's "freeze her / edit charter" beats fall flat.

Curl evidence:
```
curl :7400/api/v1/workers/<id>/overview | jq '.permissions'
→ all false
```

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F11 [P1] — `permissions.can_*: false` blocks all interactions in demo mode

## Fix recipe

1. **Add a demo-mode permission override** in `crates/argo-acp/src/handlers/workers.rs` (or wherever `compute_permissions` lives):
   ```rust
   pub fn compute_permissions(principal: &Principal, worker: &Worker, demo_mode: bool) -> WorkerPermissions {
       if demo_mode && principal.is_demo() {
           return WorkerPermissions::all_true();
       }
       // existing role-based logic
   }
   ```
   `Principal::is_demo()` returns true when the principal was minted via `argo demo-mint` (existing flag on the principal record).

2. **`demo_mode` is sourced from gateway state** — already present in the AgentDeps state under `ARGO_DEMO_MODE=1`. Plumb it into `compute_permissions` callsites.

3. **Audit the override**: every demo-mode override emits an audit row `kind = permission.demo_override_granted` with `{principal_id, worker_id, permissions: ["can_edit_*", ...]}` so the override is visible in the chain and never hidden.

4. **Tests**:
   - `crates/argo-acp/src/handlers/workers.rs` inline test: demo principal + `demo_mode=true` → all-true permissions.
   - Non-demo principal + `demo_mode=true` → role-based behavior (no leakage to real users sharing a demo tenant).
   - Demo principal + `demo_mode=false` → original role-based behavior (production safety: prevents the override from ever firing in prod by accident).

5. **SPA-side**: no changes needed; the gated buttons / tabs / cards already read the permissions object and enable when true. Verify `FreezeButton.tsx`, `DutiesCard.tsx`, `WorkerOverviewContent.tsx` consume the permissions correctly post-fix.

## Acceptance criteria

- [ ] With `ARGO_DEMO_MODE=1` + demo-minted principal, `curl :7400/api/v1/workers/<id>/overview | jq '.permissions'` returns all-true.
- [ ] With `ARGO_DEMO_MODE=1` + non-demo principal (manual SSO sign-in into the demo tenant), permissions follow normal role-based logic.
- [ ] With `ARGO_DEMO_MODE=0` + demo-minted principal, permissions follow normal role-based logic (no override leakage).
- [ ] Every demo-mode override emits the `permission.demo_override_granted` audit row.
- [ ] FreezeButton + DutiesCard toggles render enabled on the demo worker detail page.
- [ ] Inline tests pass cold: `cargo test -p argo-acp compute_permissions_demo`.

## May-touch

- `crates/argo-acp/src/handlers/workers.rs` (compute_permissions branch)
- `crates/argo-acp/src/handlers/workers/permissions.rs` (if separate)
- `crates/argo-cli/src/handlers/gateway/routes_worker_overview.rs` (plumb demo_mode into call)
- `crates/argo-audit/src/kind.rs` (new `permission.demo_override_granted` kind — check if a split is needed; if file is near LOC budget, sequence behind the audit-kind split brief)
- Inline tests in the above

## Out of scope

- Permission UI flip for production tenants (separate program — v1.1 RBAC surface)
- Demo principal lifecycle (separate)
- Cross-tenant permission leakage tests (already pinned by `claim_6_live_cross_tenant`)

## Effort

~80-150 LOC across the permissions branch + audit kind + inline tests. ~3-4 hours. P1 because Scene 2 needs working buttons on camera; the demo-seed brief is upstream of this (need real workers seeded first to confirm permissions wire end-to-end).
