# Stream J10 — Gateway worker memory endpoints (`GET /memory` + facts CRUD)

**Owner**: claude. **Depends on**: J7 MERGED; existing `mission-memory-per-worker-scope` MERGED (memory namespace primitive).
**Authored against**: origin/main `0604e7ff` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J10-gateway-worker-memory-endpoints`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 10 of 18 — unblocks frontend E4-d from mock fallback.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Fact taxonomy | Free-text per row | Operator-defined fact-category schema | `mission-enterprise-memory-fact-categories` (named in E4-d) |
| PII classification per fact | None at v1 | Per-fact sensitivity classification (low/med/high) — server-side LLM-classified | `mission-enterprise-memory-pii-classification` (named in E4-d) |
| Cross-worker share | Not exposed | Explicit operator-granted cross-worker share | `mission-enterprise-cross-worker-memory-share` (named in E4-d) |
| Append-only | DELETE soft-deletes with audit row | Hard-archive after 90-day retention; recoverable for the window | `mission-enterprise-memory-retention` |
| Conversation thread privacy | Worker + counterparty handle only; no message bodies | Operator-readable threads gated by tenant policy | `mission-enterprise-conversation-thread-rbac` (named in E4-d) |

v1 is acceptable: facts are workspace-scoped + tenant-
isolated; mutations are audit-logged via the existing
memory primitive.

## Why

E4-d's Memory tab calls multiple endpoints under
`/api/v1/workers/<id>/memory/*`. Until they exist,
E4-d falls back to mock data. This brief lands all
4 endpoints in one batch (they share the same
auth + tenant model + audit pattern).

## Scope

- 4 JSON endpoints:
  - `GET /api/v1/workers/<id>/memory` →
    `{ facts[], active_conversations[], recent_decisions[], voice_profile, namespace_id }`.
  - `POST /api/v1/workers/<id>/memory/facts` body
    `{ text }` → `201 { fact_id, text, created_at }`.
  - `PUT /api/v1/workers/<id>/memory/facts/<fact_id>` body
    `{ text }` → `200 { fact_id, text, edited_at }`.
  - `DELETE /api/v1/workers/<id>/memory/facts/<fact_id>`
    → `204 No Content`.
- All endpoints tenant-scoped + worker-scoped.
- J2 redaction applied to fact text + conversation
  snippets per the requesting Principal.
- DELETE is soft-delete (audit-logged tombstone).
- Each mutation emits an audit row with the
  delta (old → new for edits).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  `routes_dispatch_get.rs` (1 arm) +
  `routes_dispatch_post.rs` (1 arm) +
  `routes_dispatch_put.rs` (1 arm — verify file
  exists; if not, NEW + 1 arm) +
  `routes_dispatch_delete.rs` (same).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_memory.rs`
  — NEW. All 4 handlers. ≤ 450 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_worker_memory_tests.rs`
  — NEW. ≥ 12 tests. ≤ 450 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1 line.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly: 1 arm.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
  — narrowly: 1 arm.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_put.rs`
  (NEW if absent) — 1 arm. ≤ 200 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_delete.rs`
  (NEW if absent) — 1 arm. ≤ 200 LOC.

## Files you must NOT touch

- `crates/argo-memory/**` — read-only; consumed
  via existing `SkillStore`-equivalent memory
  primitive.
- Other Track-J handler files.

## Acceptance criteria

- [ ] `GET /api/v1/workers/<id>/memory` returns
      200 + bundled JSON shape (4 sections).
- [ ] `GET` response includes the worker's
      namespace_id (visible to operators only;
      hidden from delegators).
- [ ] `POST .../facts` with valid body inserts a
      fact row + emits audit row + returns 201.
- [ ] `PUT .../facts/<fact_id>` updates the fact +
      audit-logs old→new delta. Original fact ID
      preserved.
- [ ] `DELETE .../facts/<fact_id>` soft-deletes
      (sets deleted_at; row stays in DB) + audit-
      logs.
- [ ] Cross-tenant or cross-worker fact access →
      404 (don't leak existence).
- [ ] J2 redaction applied to fact text + active-
      conversation snippets.
- [ ] Empty fact text (POST/PUT) → 400.
- [ ] Fact text > 1000 chars → 400 with limit
      message.
- [ ] Bearer auth required.
- [ ] 12 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J10-gateway-worker-memory-endpoints`
- PR title: `gateway: worker memory CRUD endpoints (J10)`

## Notes for the implementer

- **Memory namespace primitive.** The existing
  `argo-memory` crate exposes per-worker namespaces
  (per `mission-memory-per-worker-scope` MERGED).
  Use the existing CRUD; this brief is HTTP
  wiring.

- **Soft-delete semantics.** DELETE sets
  `deleted_at = NOW()`. Future GETs filter
  `WHERE deleted_at IS NULL`. Audit log records
  the deletion. After 90 days a sweep job
  hard-deletes (out of scope here).

- **Active conversations.** Read from the
  worker's conversation-thread table (already
  exists for the active-channel state). Return:
  ```json
  {
    "counterparty_handle": "@aroha",
    "topic_snippet": "Bauer Vapor X5 inquiry",
    "last_msg_at": "<ISO>",
    "msg_count": 3,
    "latest_action_id": "act_01H..."
  }
  ```
  Latest_action_id lets the frontend route to the
  audit drill-in for the latest action.

- **Recent decisions.** Pull from audit_record
  where `outcome ∈ ("approved", "refused")` AND
  `decision_meta.material = true` for the worker
  in the last 14 days. Top 10.

- **Voice profile.** Currently embedded JSON on
  the worker row (from J5). Return read-only here;
  edits route through the retrain endpoint (J12).

- **Audit-row content.** For DELETE: include the
  fact text being deleted so operators can
  reconstruct what was there. For PUT: include
  both old and new text. For POST: include the
  new text.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-memory-fact-categories`
  (named).
- `mission-enterprise-memory-pii-classification`
  (named).
- `mission-enterprise-cross-worker-memory-share`
  (named).
- `mission-enterprise-memory-retention` —
  hard-archive after 90 days.
- `mission-enterprise-conversation-thread-rbac`
  (named).
