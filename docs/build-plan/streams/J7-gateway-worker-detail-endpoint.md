# Stream J7 — Gateway worker detail endpoint (`GET /api/v1/workers/<id>`)

**Owner**: claude. **Depends on**: J5 MERGED (workers exist); J6 MERGED (shared sort/serialization helpers).
**Authored against**: origin/main `de5b1ab0` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J7-gateway-worker-detail-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 7 of 18 — unblocks frontend E4-a from mock fallback.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Permissions resolution | Server computes per-Principal `permissions` field on the response | Same; v1 ships the contract | n/a |
| Cross-tenant access | 403 strict (same-tenant only) | Operator may switch tenant context with explicit grant | `mission-enterprise-cross-tenant-access` (already named E4-a) |
| Field-level RBAC | All workers in tenant return all fields | Per-field RBAC: e.g. delegators don't see `worker_id_full` or `provenance_hashes` | `mission-enterprise-worker-field-rbac` |
| Soft-deleted workers | 404 on archived workers | Per-permission "view archived" returns the row + flag | `mission-enterprise-archived-worker-view` |
| Audit | Each detail-view emits an audit row (so we know who looked at whose worker, when) | Same | n/a |

v1 is acceptable: detail endpoint is read-only, the
hard fence is tenant_id; field-level RBAC is hardening.

## Why

E4-a's frontend brief calls `GET /api/v1/workers/<id>`
to render the worker profile page. Without this
endpoint, E4-a falls back to mock data. This brief
unblocks the production profile page.

## Scope

- 1 JSON endpoint:
  - `GET /api/v1/workers/<id>` →
    `200 { worker: WorkerDetail }`.
- Tenant-scoped from Principal.
- Returns the full `WorkerDetail` JSON shape (per
  E4-a's locked Worker type extended with department
  / team / kpiToday / permissions).
- Resolves `permissions.canEditCharter` etc. per
  the Principal's role.
- 404 on unknown id or cross-tenant id.
- 410 on archived workers (with `Retry-After` info
  hinting at the 30-day recovery window).
- Audit row written per view.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `routes_dispatch_get.rs` (narrowly
  add 1 arm).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_detail.rs`
  — NEW. ≤ 320 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_worker_detail_tests.rs`
  — NEW. ≥ 10 tests. ≤ 400 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1-line `pub mod` addition.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly: 1 arm for the path. ≤ 250 LOC after.

## Files you must NOT touch

- `crates/argo-identity/src/store.rs` — read-only.
- Other Track-J handler files.

## Acceptance criteria

- [ ] `GET /api/v1/workers/<id>` for a same-tenant
      worker returns 200 + worker JSON.
- [ ] WorkerDetail JSON includes (matching E4-a's
      type extension):
      ```json
      {
        "id": "...",
        "name": "...",
        "avatar_url": null,
        "role": { "tagline": "...", "vertical": "..." },
        "status": "...",
        "department": { ... } | null,
        "team": { ... } | null,
        "kpi_today": { "label": "...", "value": "..." } | null,
        "permissions": {
          "can_edit_charter": true,
          "can_edit_duties": true,
          "can_pause_resume": true,
          "can_archive": true,
          "can_delete": false,
          "can_view_audit": true,
          "can_view_settings": false
        },
        "headline": { ... } | null,
        "last_active_at": "...",
        "pending_approval_count": 0
      }
      ```
- [ ] Cross-tenant access → 403 with envelope.
- [ ] Unknown worker id → 404 with envelope.
- [ ] Archived worker → 410 with envelope (body
      includes `recoverable_until: <ISO>`).
- [ ] Bearer auth required (401 otherwise).
- [ ] Permissions field reflects Principal's role:
      - Delegator-of-this-worker: `can_edit_charter
        = true`, `can_delete = false`,
        `can_view_settings = false`.
      - Operator (tenant-admin): all true.
      - Auditor: only `can_view_audit = true`;
        edit perms false; `can_view_settings = false`.
- [ ] Audit row written per view (event_type:
      `worker.detail.viewed`).
- [ ] 10 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J7-gateway-worker-detail-endpoint`
- PR title: `gateway: GET /api/v1/workers/<id> detail endpoint (J7)`

## Notes for the implementer

- **Permissions resolution function.** Single
  function `resolve_permissions(principal, worker)
  -> Permissions`. Inputs:
  - Principal's role (delegator / operator /
    auditor / compliance-officer).
  - Is principal the hire-author of this worker?
  - Tenant equality already enforced upstream.
  Outputs the 7 boolean flags.

- **Audit row content.** Just metadata
  (`actor_id`, `worker_id`, `viewer_role`).
  Don't include the response body itself.

- **Archived workers.** Archive is a soft-delete
  with a 30-day recovery window. Return 410 (not
  404) so clients can distinguish "never existed"
  from "was here, soft-deleted". Body includes
  `recoverable_until` ISO timestamp.

- **kpi_today derivation.** Pull the most prominent
  KPI from the worker's vertical KPI table (the
  same one E3-c uses for the headline generator).
  Returns a single `{ label, value }` pair. If no
  KPIs are configured, return null.

- **Response time budget.** This is a hot path
  (every worker profile page load). Target p50 <
  50ms, p99 < 200ms. The KPI lookup + permissions
  resolution should both stay under 5ms each.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-cross-tenant-access` (already
  named).
- `mission-enterprise-worker-field-rbac` — per-
  field RBAC on the detail response.
- `mission-enterprise-archived-worker-view` —
  permission-gated view of archived workers.
