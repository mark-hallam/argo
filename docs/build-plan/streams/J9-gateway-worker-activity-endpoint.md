# Stream J9 — Gateway worker activity endpoint (`GET /api/v1/workers/<id>/activity`)

**Owner**: claude. **Depends on**: J7 MERGED.
**Authored against**: origin/main `e6aa4ca9` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J9-gateway-worker-activity-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 9 of 18 — unblocks frontend E4-c from mock fallback.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Pagination | Cursor-based, 50 per page | Same | n/a |
| Filter dimensions | 3 dims: time range / action type / status | Full audit-timeline parity (+ channel + approver + cost band + data classification) | `mission-enterprise-activity-filter-parity` (named in E4-c) |
| PII fence | J2 applies (server-side) | Same | n/a |
| Audit | Per-view emission | Same | n/a |

v1 is acceptable: this is a worker-scoped slice of
J14's full audit feed; same shape, narrower filter
default.

## Why

E4-c calls `GET /api/v1/workers/<id>/activity` to
render the per-worker timeline. Until this endpoint
exists, E4-c falls back to mock data. This brief
unblocks production rendering.

## Scope

- 1 JSON endpoint:
  - `GET /api/v1/workers/<id>/activity?range=&type=&status=&cursor=`
    → `200 { items: ActivityRow[], next_cursor?: string }`.
- Tenant-scoped + worker-id-filter against audit_record.
- J2 redaction applied per row.
- Default 50 rows; max 100 via `?limit=`.
- Audit row per view.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `routes_dispatch_get.rs` (1 arm).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_activity.rs`
  — NEW. ≤ 280 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_worker_activity_tests.rs`
  — NEW. ≥ 8 tests. ≤ 350 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1 line.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly: 1 arm. ≤ 310 LOC after.

## Files you must NOT touch

- `crates/argo-redact/**` — read-only (consumed
  from J2).
- Other Track-J handler files.

## Acceptance criteria

- [ ] `GET /api/v1/workers/<id>/activity` returns
      200 + `{ items: [...], next_cursor }`.
- [ ] Filter params:
      - `?range=1h|today|yesterday|7d|30d|qtd|lq`
        (default: `7d`).
      - `?type=message_sent|...` (default: all).
      - `?status=succeeded|failed|refused|pending|autonomous`
        (default: all).
      - `?cursor=<token>`.
      - `?limit=1..100` (default: 50).
- [ ] Each item is an ActivityRow (matching the
      shared TS contract):
      ```json
      {
        "id": "act_01H...",
        "timestamp": "<ISO>",
        "worker": { "id": "...", "name": "..." },
        "action_type": "...",
        "verb": "...",
        "status": "...",
        "approver": { "id": "...", "name": "..." } | null,
        "channel": "...",
        "cost": { "usd": 0.012 } | null,
        "data_classification": "..."
      }
      ```
- [ ] J2 server-side PII redaction applied to each
      item's `verb`, `approver.name`,
      `data_classification` per the Principal's
      permissions.
- [ ] Cross-tenant access → 403.
- [ ] Unknown worker id → 404.
- [ ] Archived worker: returns 410 (consistent with
      J7/J8 behavior).
- [ ] Bearer auth required.
- [ ] Cursor token opaque; client passes it back
      unchanged.
- [ ] Audit row per view (`worker.activity.viewed`).
- [ ] 8 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J9-gateway-worker-activity-endpoint`
- PR title: `gateway: GET /api/v1/workers/<id>/activity endpoint (J9)`

## Notes for the implementer

- **Cursor shape.** Opaque base64-encoded
  `(timestamp_micros, audit_record_id)` tuple.
  Server-only; client passes back as-is. Same
  encoding as J1 cursors for consistency.

- **Filter SQL:**
  ```sql
  SELECT * FROM audit_record
  WHERE tenant_id = $1
    AND worker_id = $2
    AND created_at > $3
    AND ($4::text IS NULL OR action_type = $4)
    AND ($5::text IS NULL OR outcome_status = $5)
    AND (created_at, id) < ($cursor_ts, $cursor_id)
  ORDER BY created_at DESC, id DESC
  LIMIT $6;
  ```

- **Time range parsing.** Convert the symbolic
  range to a concrete cutoff using workspace
  timezone. `today` means "since 00:00 in tenant
  tz today".

- **Redaction in the loop.** For each row:
  ```rust
  let row = redact_audit_row(row, &principal);
  ```
  Same helper as J2's `redact_approval_row`,
  scoped to audit-row fields. Adds the row to
  the redaction-emission audit batch.

- **N+1 prevention.** Approver names are
  resolved in batch: collect distinct
  approver_ids from the page, single
  `WHERE id = ANY($1)` query against
  `identity_principal` table.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-activity-filter-parity`
  (already named).
