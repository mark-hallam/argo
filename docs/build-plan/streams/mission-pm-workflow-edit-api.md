# Stream P1-6 — workflow edit/publish gateway API

**Owner**: claude.
**Depends on**: mission-pm-workflow-persistence-schema MERGED.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-workflow-edit-api`.
**Source**: skycot-pm-worker REPORT.md §3.8 + §11 P1 #6.

## Scope

With the persistence layer in place (P1 #5), expose a gateway API that lets operators (and learning workers, gated by capability) create / publish / archive workflow definitions. Routes:

- `GET /api/v1/verticals/{vertical_id}/workflows` — list workflows + their head version.
- `GET /api/v1/verticals/{vertical_id}/workflows/{name}` — fetch head version of a workflow.
- `GET /api/v1/verticals/{vertical_id}/workflows/{name}/versions` — list every version (newest first).
- `GET /api/v1/verticals/{vertical_id}/workflows/{name}/versions/{id}` — fetch a specific version.
- `POST /api/v1/verticals/{vertical_id}/workflows/{name}` — create a new version (body = TOML). Parent_id defaults to current head.
- `PUT /api/v1/verticals/{vertical_id}/workflows/{name}/head` — set head to a specific version id.
- `DELETE /api/v1/verticals/{vertical_id}/workflows/{name}/versions/{id}` — archive a version (lifecycle_state → archived).

Every mutating route requires `workflow.edit` capability; reads require `workflow.read`. Both capabilities are NEW.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-capability/src/lib.rs` — narrowly scoped: add two new `Capability::ToolDispatch` glob constants `workflow.edit`, `workflow.read`. Default-role assignments: `owner` + `admin` get edit, `user` + `guest` get read. ≤ 30 LOC of additions.
- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` (NEW) — the 7 route handlers.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` — narrowly scoped: add the 4 GET dispatch arms. No other edits.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — narrowly scoped: add the POST arm + the PUT + DELETE arms. No other edits.
- `crates/argo-cli/tests/workflow_edit_api_smoke.rs` (NEW) — integration smoke covering create + set-head + list + archive round-trip with capability checks.

## Files you must NOT touch

- `crates/argo-store/src/workflow_definition.rs` — locked from P1 #5. Use its API as-is.
- `crates/argo-vertical/src/loader.rs` — loader continues to read files for now; the DB is a parallel store.
- Any other gateway route module.

## Acceptance criteria

- [ ] Two new capabilities: `workflow.edit`, `workflow.read`. Default roles per the scope section.
- [ ] All 7 routes serve correct status codes: 200 read-success, 201 create-success, 204 head-update + archive, 400 malformed TOML, 401 missing bearer (per existing `mission-gateway-bearer-auth` precedent), 403 capability denied, 404 missing vertical / workflow / version.
- [ ] TOML body in POST is parsed via `argo_vertical::workflow::parse_workflow`; parse error returns 400 with the parser's error message.
- [ ] Capability check fires BEFORE the DB call (defense-in-depth — a 403 should not leak whether the workflow exists).
- [ ] Audit rows emitted on every mutating route (`WorkflowDefinitionCreated`, `WorkflowHeadChanged`, `WorkflowVersionArchived` — three NEW `AuditKind` variants). Payload includes `(actor, tenant_id, vertical_id, name, version_id)`.
- [ ] Smoke test in `workflow_edit_api_smoke.rs` covers: (a) anonymous request → 401; (b) reader principal → 200 on GET, 403 on POST; (c) editor principal → 201 on POST, 200 on subsequent GET; (d) head-pointer update visible in `GET head`; (e) archive on the current head clears the head pointer.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-cli` passes (the smoke test runs the real argo binary end-to-end).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-workflow-edit-api`
- PR title: `gateway: workflow definition CRUD routes + capabilities (mission-pm-workflow-edit-api)`

## Notes for the implementer

- The route shapes mirror `routes_skills.rs` (or equivalent existing surface) — reuse the same JSON envelope, error mapping, and bearer-auth precedent. Don't invent a new convention.
- The three new `AuditKind` variants should land in this brief, not be deferred. They are small additions to `event.rs::KNOWN_KINDS`. Cross-reference with mission-pm-capability-audit-events (P0 #4) — both briefs add audit kinds; coordinate so the test that asserts `KNOWN_KINDS.len()` ticks up correctly. If P0 #4 lands first, this brief picks up `len() == 19` and bumps to `len() == 22`.
- Capability strings: `workflow.edit` and `workflow.read` follow the existing `audit.view.cross_tenant` precedent (dot-separated). They are `Capability::ToolDispatch { tool_name_glob }` values, not a new variant.
- The conversational author surface (P2 #10) will eventually POST through this API with the worker's own principal. Make sure the capability check works for non-human principals (the `Service` variant of `Principal`).

## Out of scope

- A CLI wrapper (`argo workflow create/edit/...`) — separate small follow-up; the gateway API is the contract.
- A WebSocket subscription for "workflow changed" — fine to add later, not in scope here.
- File-system mirror (writing the DB content back to `verticals/<id>/workflows/`) — deferred.
