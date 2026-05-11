# Stream J8 — Gateway worker overview endpoint (`GET /api/v1/workers/<id>/overview`)

**Owner**: claude. **Depends on**: J7 MERGED.
**Authored against**: origin/main `f179f2a0` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J8-gateway-worker-overview-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 8 of 18 — unblocks frontend E4-b from mock fallback.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| KPI selection | Vertical-default 4 KPIs per E3-c table | Operator-customizable per-worker KPI selection | `mission-enterprise-kpi-customization` (named in E4-b) |
| Sparkline data window | 7-day | Configurable per-tenant retention; sparkline window scoped accordingly | `mission-enterprise-kpi-retention` (named in E4-b) |
| Recent-activity scope | Last 5 actions for this worker | Same | n/a |
| Charter source | Embedded JSON on worker row | Voice profile in its own table with version history | `mission-enterprise-voice-profile-versioning` (named in J5) |
| Audit | View emits audit row | Same | n/a |

v1 is acceptable: the Overview tab is read-heavy +
mostly aggregation of existing data.

## Why

E4-b's Overview tab calls
`GET /api/v1/workers/<id>/overview` to render the 4
cards (WhatSheDoes / Duties / KPIs / RecentActivity).
Until this endpoint exists, E4-b falls back to mock
data. This brief lands the bundled response.

## Scope

- 1 JSON endpoint:
  - `GET /api/v1/workers/<id>/overview` →
    `200 { charter, duties, kpis, recent_activity }`.
- Tenant-scoped + permission-gated (re-uses J7's
  permission resolution).
- KPIs computed from the worker's vertical KPI
  table (hardcoded in E3-c's
  `argo-team-headline::kpi_table` for v1).
- Per-KPI 7-day sparkline data: vector of 7
  numbers, last-7-days-by-day, in workspace tz.
- Recent activity: last 5 audit rows for the
  worker.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `routes_dispatch_get.rs`
  (narrowly add 1 arm).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_overview.rs`
  — NEW. ≤ 350 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_worker_overview_tests.rs`
  — NEW. ≥ 10 tests. ≤ 400 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1-line `pub mod` add.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly: 1 arm for the path. ≤ 280 LOC after.

## Files you must NOT touch

- `crates/argo-team-headline/**` — read-only;
  KPI table consumed via its existing public API.
- `crates/argo-identity/**` — read-only.
- Other Track-J handler files.

## Acceptance criteria

- [ ] `GET /api/v1/workers/<id>/overview` returns
      200 + JSON `{ charter, duties[], kpis[],
      recent_activity[] }`.
- [ ] Charter shape:
      ```json
      {
        "text": "...",
        "version": 1,
        "last_edited_at": "<ISO>",
        "last_edited_by": "<principal_id>"
      }
      ```
- [ ] Duties shape: array of
      ```json
      {
        "id": "...",
        "label": "...",
        "schedule": { "kind": "cron", "spec": "0 6 * * *" },
        "description": "...",
        "enabled": true,
        "next_fire_at": "<ISO>"
      }
      ```
- [ ] KPIs shape: 4 items (or 3 if vertical only
      defines 3):
      ```json
      {
        "key": "replies_sent",
        "label": "Replies sent",
        "value": "128",
        "trend": { "direction": "up", "delta_pct": 12.5 },
        "sparkline_7d": [104, 112, 98, 145, 120, 132, 128]
      }
      ```
- [ ] Recent activity: array of 5
      ActivityRow-shaped items (timestamp + verb +
      status + cost).
- [ ] Cross-tenant access → 403.
- [ ] Unknown worker id → 404.
- [ ] Archived worker → 410.
- [ ] Bearer auth required (401 otherwise).
- [ ] Permissions resolved via J7's helper;
      response includes `permissions.can_edit_charter`
      etc. (re-shared with worker-detail).
- [ ] Audit row written per view (event_type:
      `worker.overview.viewed`).
- [ ] Response time budget: p99 < 300ms for a
      worker with 7-day audit history.
- [ ] 10 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J8-gateway-worker-overview-endpoint`
- PR title: `gateway: GET /api/v1/workers/<id>/overview endpoint (J8)`

## Notes for the implementer

- **KPI computation.** Reuse
  `argo-team-headline::kpi_table` to identify
  which KPIs apply to the worker's vertical.
  For each KPI:
  - Run the aggregation query (count / sum /
    average) over the audit-row window.
  - Compute the previous-period value for the
    same window length to derive the trend.
  - Compute daily values for the sparkline (7
    buckets by workspace day).

- **Sparkline bucketing.** Use workspace timezone
  to bucket audit rows into 7 day-aligned
  intervals. Workers in different time zones
  produce sparklines that align to their
  workspace's day boundaries.

- **Trend direction logic:**
  ```rust
  fn trend(curr: f64, prev: f64) -> Trend {
      let pct = if prev > 0.0 {
          (curr - prev) / prev * 100.0
      } else {
          0.0
      };
      let direction = if pct.abs() < 2.0 {
          Direction::Flat
      } else if pct > 0.0 {
          Direction::Up
      } else {
          Direction::Down
      };
      Trend { direction, delta_pct: pct }
  }
  ```

- **`direction_means_good`** — front-end DeltaChip
  consumes this via the KPI metadata. For
  `replies_sent`, up = good. For
  `avg_response_time`, down = good. Encode in
  the KPI table.

- **N+1 prevention.** Single audit-row query
  pulls the data for all 4 KPIs + the 5 recent-
  activity rows. Don't fire one query per KPI.

- **Recent activity scope.** Last 5 actions
  regardless of action_type. Frontend uses this
  as the bridge to the Activity tab (E4-c) which
  shows the full timeline.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-kpi-customization` (named).
- `mission-enterprise-kpi-retention` (named).
