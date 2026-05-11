# Stream J14 — Gateway audit feed endpoint (`GET /api/v1/audit`)

**Owner**: claude. **Depends on**: J2 MERGED (server-side redaction); J9 MERGED (shared activity-row shape).
**Authored against**: origin/main `351d9efc` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J14-gateway-audit-feed-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 14 of 18 — unblocks frontend E5-a from mock fallback.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Filter dimensions | 7 dims per E5-b (worker / channel / action_type / status / approver / cost_band / classification) | Same | n/a |
| Full-text search | LIKE-based substring match on `verb` + worker.name + action_id | Trigram indexed full-text search per tenant | `mission-enterprise-audit-full-text-search` (named) |
| Cross-tenant view | Single-tenant strict | Operator with cross-tenant grant may filter across tenants | `mission-enterprise-cross-tenant-audit` (named) |
| RBAC field filtering | J7's permission model + J2 redaction | Per-row visibility filtering (some action types hidden from delegators) | `mission-enterprise-audit-row-rbac` |
| Pagination | Cursor-based, 50/page, max 200 | Same | n/a |
| Cost field visibility | Always returned in v1 | Hidden from delegators unless they own the worker | `mission-enterprise-cost-rbac` |

v1 is acceptable: the audit feed is read-only over
the existing audit_record table; J2's redaction
fence handles PII. Cross-tenant + full-text +
row-level RBAC are hardening.

## Why

E5-a's frontend brief calls `GET /api/v1/audit`
with various filter params. Until this endpoint
exists, E5-a falls back to mock data. This brief
unblocks the production audit timeline.

## Scope

- 1 JSON endpoint:
  - `GET /api/v1/audit?range=&q=&worker=&channel=&action_type=&status=&approver=&cost_band=&classification=&cursor=&limit=`
    → `200 { items: AuditRow[], next_cursor?: string }`.
- Tenant-scoped from Principal.
- Filter composition: AND across dimensions; multi-
  value within a dimension is OR (e.g.
  `?status=succeeded,refused`).
- J2 redaction applied per row.
- Default 50 rows; max 200 via `?limit=`.
- Cursor token opaque + base64-encoded.
- Audit row written per query.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `routes_dispatch_get.rs`
  (1 arm).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs`
  — NEW. ≤ 380 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs`
  — NEW. ≥ 14 tests (one per filter dim + multi-
  value composition + RBAC + pagination + empty).
  ≤ 450 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1 line.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly: 1 arm. ≤ 340 LOC after.

## Files you must NOT touch

- `crates/argo-audit/**` (or wherever audit_record
  is queried from) — read-only.
- `crates/argo-redact/**` — read-only (J2's
  output).
- Other Track-J handler files.

## Acceptance criteria

- [ ] `GET /api/v1/audit` returns 200 + `{ items,
      next_cursor }`.
- [ ] All 7 filter dims accept comma-separated
      values; missing → no filter.
- [ ] `?q=` performs case-insensitive substring
      match against `verb || worker_name ||
      action_id`. Empty string ignored.
- [ ] `?range=` accepts the same symbolic values
      as J9 (1h/today/yesterday/7d/30d/qtd/lq) +
      custom `{ from, to }` via JSON-encoded
      query (or two params `?from=&to=`).
- [ ] J2 redaction applied per row.
- [ ] Cursor pagination via `?cursor=<token>`;
      response `next_cursor` null on last page.
- [ ] `?limit=` accepts 1-200; default 50; out-of-
      range → 400.
- [ ] Cross-tenant access → 403.
- [ ] Bearer auth required.
- [ ] Audit row written per query (`audit.feed.queried`)
      with filter summary metadata.
- [ ] Response time budget: p99 < 500ms for a
      tenant with 1M audit rows + indexed
      filtering.
- [ ] 14 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J14-gateway-audit-feed-endpoint`
- PR title: `gateway: GET /api/v1/audit feed endpoint (J14)`

## Notes for the implementer

- **Index requirements.** Ensure indexes exist on
  `audit_record(tenant_id, created_at DESC,
  worker_id)` and
  `audit_record(tenant_id, created_at DESC,
  action_type, outcome_status)`. If missing, this
  brief adds them via a NEW migration.

- **Filter SQL pattern.**
  ```sql
  SELECT * FROM audit_record
  WHERE tenant_id = $1
    AND created_at > $2
    AND ($3::text[] IS NULL OR worker_id = ANY($3))
    AND ($4::text[] IS NULL OR channel = ANY($4))
    AND ($5::text[] IS NULL OR action_type = ANY($5))
    AND ($6::text[] IS NULL OR outcome_status = ANY($6))
    AND ($7::text[] IS NULL OR approver_id = ANY($7))
    AND ($8::int IS NULL OR cost_band(cost_usd) = $8)
    AND ($9::text[] IS NULL OR data_classification = ANY($9))
    AND (verb ILIKE $10 OR worker_name ILIKE $10 OR id::text ILIKE $10)
    AND (created_at, id) < ($cursor_ts, $cursor_id)
  ORDER BY created_at DESC, id DESC
  LIMIT $limit;
  ```

- **Cost band mapping.** Reuse the function from
  E5-b (frontend) on the server:
  ```rust
  enum CostBand { Free, Lt001, From001To10, Gt10 }
  fn cost_band(cost: Option<f64>) -> CostBand { ... }
  ```

- **`?q=` performance.** With 1M+ rows, ILIKE
  becomes slow. v1 acceptable as long as it's
  combined with another filter that narrows
  significantly. v1.5 adds trigram index +
  full-text search (named brief).

- **Cursor encoding.** `base64(timestamp_micros
  || record_id)`. Total length ~30 chars.

- **Audit query telemetry.** Each query emits an
  audit row with filter summary (which dims were
  used, row count returned). Helps operators
  see "who looked at what" over time.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-audit-full-text-search`
  (named).
- `mission-enterprise-cross-tenant-audit`
  (named).
- `mission-enterprise-audit-row-rbac` — per-row
  visibility filtering.
- `mission-enterprise-cost-rbac` — cost field
  hidden from delegators by default.
