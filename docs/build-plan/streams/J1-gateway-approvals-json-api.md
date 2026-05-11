# Stream J1 — Gateway approvals JSON API (replaces HTML `/inbox` with JSON `/api/v1/approvals`)

**Owner**: claude.
**Depends on**: none — extends the existing `argo-cli/src/handlers/gateway/routes_channels_webinbox.rs` (HTML inbox stays untouched).
**Authored against**: origin/main `8a55f631` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J1-gateway-approvals-json-api`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 1 of 18 — unblocks frontend E1-a from mock fallback to real data.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Auth | Bearer token (existing `ARGO_GATEWAY_BEARER_TOKEN`) | SSO + per-tenant key rotation | `mission-enterprise-auth-sso` |
| RBAC | Tenant-scoped; viewer sees only their tenant's approvals | Per-role visibility (delegator vs operator vs auditor); per-action visibility for sensitive types | `mission-enterprise-approvals-rbac` |
| Pagination | Cursor-based, 50 per page | Same | n/a |
| PII redaction | This brief returns raw data; client-side E1-e redaction is defense-in-depth | Server-side first fence (mandatory) — see J2 | `mission-enterprise-approvals-pii-server-fence` (= J2) |
| Audit | Every list/decide/undo emits an audit row (uses existing audit-row infra) | Same | n/a |
| Rate limit | None (relies on existing gateway global rate limits) | Per-tenant approval-API rate limit | `mission-enterprise-api-rate-limit` |

v1 is acceptable: this brief is a JSON wrapper over
the existing approval state machine. The state
machine itself is already audit-logged + tenant-
scoped from prior shipped work.

## Why

The current gateway `/inbox` route serves HTML
(per `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`).
E1-a's frontend brief consumes a JSON API at
`/api/v1/approvals` with cursor pagination; while
that endpoint doesn't exist, E1-a falls back to
mock data. This brief lands the real JSON endpoint
so production frontend gets real data.

## Scope

- 3 JSON endpoints, all under `/api/v1/approvals`:
  - `GET /api/v1/approvals?cursor=<c>&limit=<n>` →
    `{ items: ApprovalRow[], next_cursor?: string }`.
  - `POST /api/v1/approvals/<id>/decide` body
    `{ decision: "approve" | "refuse" }` →
    `{ ok: true, approval_id, decision }`.
  - `POST /api/v1/approvals/<id>/undo` →
    `{ ok: true, approval_id }`.
- Re-uses the existing approval state machine
  (`crates/argo-channels/src/webinbox.rs` +
  related approval-action code).
- Bearer-auth required (uses E0's existing
  bearer-auth precedent merged via #946).
- Tenant scoping: only the caller's tenant's
  approvals returned.
- 50-row default page; max 100; client supplies
  cursor token from prior response.
- HTML `/inbox` route stays unchanged (legacy
  consumers).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `crates/argo-cli/src/handlers/gateway/mod.rs`
    (register new route module)
  - `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
    (POST dispatch chain — narrowly add 2 arms)

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs`
  — NEW. ≤ 400 LOC. The 3 endpoint handlers.
- `crates/argo-cli/src/handlers/gateway/routes_approvals_json_tests.rs`
  — NEW. ≥ 12 unit tests. ≤ 450 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly scoped: 1-line `pub mod` addition.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly scoped: 1 arm for the GET. ≤ 200 LOC
  after.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
  — narrowly scoped: 2 arms (decide + undo).
  ≤ 250 LOC after.
- `docs/skycot/RUNBOOK.md` — narrowly scoped:
  add §"Approvals JSON API" with endpoints +
  curl examples.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`
  — locked. HTML inbox is preserved.
- `crates/argo-channels/src/webinbox.rs` —
  read-only consumer.
- The approval state machine — read-only.
- Any other gateway route file.

## Acceptance criteria

- [ ] `GET /api/v1/approvals` returns 200 with JSON
      body `{ items: [...], next_cursor: <opt> }`.
- [ ] Default page size 50; `?limit=` accepts
      1-100; out-of-range returns 400 with envelope
      `{ error: { code: "bad_request", message: "..." } }`.
- [ ] Cursor pagination: `?cursor=<token>` resumes
      from prior page; missing cursor starts fresh.
- [ ] `POST /api/v1/approvals/<id>/decide` body
      `{ decision: "approve" }` returns
      `{ ok: true, approval_id: <id>, decision: "approve" }`
      AND mutates the approval state.
- [ ] `POST /api/v1/approvals/<id>/decide` body
      `{ decision: "refuse" }` returns ok + mutates.
- [ ] `POST /api/v1/approvals/<id>/undo` returns ok
      + reverses the decision (within the existing
      undo window enforced by the state machine).
- [ ] `POST .../decide` for an already-decided
      approval returns 409 Conflict with envelope.
- [ ] `POST .../undo` outside the undo window
      returns 410 Gone with envelope.
- [ ] All endpoints require `Authorization: Bearer
      <token>` (existing pattern).
- [ ] All endpoints are tenant-scoped (validated
      against `principal.tenant_id`).
- [ ] All POSTs emit an audit row via existing
      audit infrastructure.
- [ ] 12 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] `cargo test -p argo-cli` passes.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J1-gateway-approvals-json-api`
- PR title: `gateway: JSON approvals API (/api/v1/approvals) (J1)`

## Notes for the implementer

- **`ApprovalRow` JSON shape** (matches the frontend
  `ApprovalItem` from `web/lib/sample-approvals.ts`):
  ```json
  {
    "id": "ap_01H9Z7",
    "worker": { "id": "...", "name": "...", "avatar_url": null },
    "verb": "wants to send this listing",
    "detail": "...",
    "consequence_if_yes": "...",
    "consequence_if_no": "...",
    "reason": "...",
    "channel": "slack",
    "received_at": "2026-05-12T14:32:00Z",
    "decision": null
  }
  ```
  Use snake_case in JSON; frontend already maps
  to camelCase via TS.

- **Cursor design.** Opaque base64-encoded
  `(timestamp, id)` tuple. Server-only; client
  passes back as-is. No PII in the cursor.

- **State machine reuse.** The existing approval
  state machine in `argo-channels::webinbox::*`
  already enforces:
  - State transitions (pending → approved/refused
    → undone).
  - Undo window (30s default).
  - Audit emission.
  Don't reimplement — call into the existing API.

- **Error envelope.** Matches the convention
  established by `web/lib/api/types.ts` (E0):
  ```json
  { "error": { "code": "approval_not_found", "message": "..." } }
  ```
  Standard codes: `bad_request`, `unauthorized`,
  `forbidden`, `approval_not_found`,
  `already_decided`, `undo_window_expired`,
  `internal_error`.

- **Tenant scoping is mandatory.** Every query
  filters `WHERE tenant_id = $1` from the bearer-
  auth's resolved principal. Missing tenant_id
  fails closed with 401.

- **Don't return PII unredacted.** J2 (server-side
  PII fence) ships next; until it merges, frontend
  defense-in-depth applies. This brief returns
  raw data per its v1 posture; J2 wraps it.

- **Idempotency on POST.** If a client sends the
  same `decide` twice (network retry), the second
  returns the same response as the first (no error)
  as long as the decision matches. Different
  decision → 409. Standard idempotency.

- **Test plan — 12 unit tests:**
  1. GET happy path (returns items + cursor).
  2. GET with cursor (resumes correctly).
  3. GET with `limit=100` (max).
  4. GET with `limit=200` (over-max → 400).
  5. GET tenant isolation (no cross-tenant rows).
  6. POST decide approve happy path.
  7. POST decide refuse happy path.
  8. POST decide already-decided (409).
  9. POST undo happy path.
  10. POST undo outside window (410).
  11. Missing bearer (401).
  12. Audit row written per POST.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-approvals-rbac` — per-role
  approval visibility.
- `mission-enterprise-api-rate-limit` — per-tenant
  rate limiting.

Sibling brief in this batch:

- `J2-gateway-approvals-pii-redaction-server-side`
  — wraps this endpoint's response with server-
  side PII redaction.
