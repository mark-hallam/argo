# web-workflow-per-workflow-runs-table — replace hardcoded "82 runs / 94%" with real data

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-per-workflow-runs-table`.
**Priority**: P2 — wave-15 user-journey audit caught the `/workflows/<v>/<n>`
detail page displaying literal `82` runs and `94%` success rate
**hardcoded into the JSX**. Demo buyer screenshots the page and asks "is
that real?" — operator has to admit no.
**Sizing**: ~350-450 LOC across gateway endpoint (workflow runs query) +
web swap-in + 6 tests + sample-data deterministic fixture.

---

## Why this brief exists

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx` has:

```tsx
<div style={...}>82</div>
<div>Runs in the last 30 days</div>

<div>94%</div>
<div>Success rate</div>
```

These are **JSX literals**, not props, not hardcoded fixtures — actual
inline numbers that show on EVERY workflow page regardless of whether
that workflow has been run 0 times or 10,000 times.

The `workflow_run` table already exists (migration 18, since the original
v0.x cli). The query is trivial. This brief swaps the literals for real
counts.

---

## Pre-flight verification

```bash
# 1. Confirm the literals are still on origin/main.
rg "^\s*82\s*$\|^\s*94%\s*$" web/app/\(app\)/workflows/

# 2. Confirm the workflow_run table exists.
sqlite3 ~/.argo/store.db ".schema workflow_run"

# 3. Confirm no /api/v1/workflows/<v>/<name>/runs endpoint exists yet.
rg "workflow.*runs.*api\|/runs " crates/argo-cli/src/handlers/gateway/
```

---

## Phase 1 — Gateway endpoint

### Path

```
GET /api/v1/verticals/<vertical_id>/workflows/<name>/runs[?days=30&limit=100]
```

Returns:
```json
{
  "vertical_id": "pr_checker",
  "workflow_name": "standard_review",
  "window_days": 30,
  "run_count": 82,
  "success_rate": 0.94,
  "avg_duration_ms": 4321,
  "p95_duration_ms": 9876,
  "last_run_at": "2026-05-20T14:32:00Z",
  "runs": [
    { "id": "...", "started_at": "...", "finished_at": "...", "ok": true, "duration_ms": 1234 },
    ...
  ]
}
```

### Handler

**File**: `crates/argo-cli/src/handlers/gateway/routes_workflow_runs.rs` (new)

```rust
async fn list_runs_response(
    vertical_id: &str,
    name: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store { Some(s) => s, None => return store_unavailable() };
    let tenant = match resolve_workspace_tenant(store).await { /* ... */ };

    // Parse query params.
    let days = query_param(req, "days").unwrap_or("30").parse::<i64>().unwrap_or(30);
    let limit = query_param(req, "limit").unwrap_or("100").parse::<i64>().unwrap_or(100);
    let window_micros = chrono::Duration::days(days).num_microseconds().unwrap_or(0);
    let cutoff = chrono::Utc::now().timestamp_micros() - window_micros;

    // Workflow id: workflow_run.workflow_id is the (vertical_id, name) key
    // joined via workflow_definition. Or stored directly — verify schema.
    // For v1, assume `workflow_id = "<vertical_id>/<name>"`.
    let workflow_key = format!("{vertical_id}/{name}");

    // 1. Aggregate stats.
    let (run_count, success_count, sum_duration, max_duration): (i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT COUNT(*), \
                    SUM(CASE WHEN ok = 1 THEN 1 ELSE 0 END), \
                    COALESCE(SUM(finished_at - started_at), 0), \
                    COALESCE(MAX(finished_at - started_at), 0) \
             FROM workflow_run \
             WHERE workflow_id = ? AND started_at >= ?",
        )
        .bind(&workflow_key)
        .bind(cutoff)
        .fetch_one(store.pool()).await
        .map_err(|e| ArgoError::Provider(format!("agg: {e}")))?;

    let success_rate = if run_count > 0 {
        success_count as f64 / run_count as f64
    } else { 0.0 };
    let avg_duration_ms = if run_count > 0 {
        sum_duration / run_count / 1000
    } else { 0 };

    // 2. p95 (cheap approximation: sort by duration, take the 95th percentile row).
    let durations: Vec<i64> = sqlx::query_scalar(
        "SELECT (finished_at - started_at) FROM workflow_run \
         WHERE workflow_id = ? AND started_at >= ? AND finished_at IS NOT NULL \
         ORDER BY (finished_at - started_at) ASC",
    )
    .bind(&workflow_key).bind(cutoff)
    .fetch_all(store.pool()).await
    .unwrap_or_default();
    let p95_duration_ms = if !durations.is_empty() {
        let idx = (durations.len() as f64 * 0.95) as usize;
        durations.get(idx.min(durations.len() - 1)).copied().unwrap_or(0) / 1000
    } else { 0 };

    // 3. Last run.
    let last_run_at: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(started_at) FROM workflow_run WHERE workflow_id = ?",
    )
    .bind(&workflow_key).fetch_one(store.pool()).await.ok().flatten();

    // 4. Recent N runs.
    let rows: Vec<RunRow> = sqlx::query_as(
        "SELECT id, started_at, finished_at, ok \
         FROM workflow_run \
         WHERE workflow_id = ? AND started_at >= ? \
         ORDER BY started_at DESC \
         LIMIT ?",
    )
    .bind(&workflow_key).bind(cutoff).bind(limit)
    .fetch_all(store.pool()).await
    .unwrap_or_default();

    let body = json!({
        "vertical_id": vertical_id,
        "workflow_name": name,
        "window_days": days,
        "run_count": run_count,
        "success_rate": success_rate,
        "avg_duration_ms": avg_duration_ms,
        "p95_duration_ms": p95_duration_ms,
        "last_run_at": last_run_at.map(epoch_micros_to_iso),
        "runs": rows.into_iter().map(|r| json!({
            "id": r.id,
            "started_at": epoch_micros_to_iso(r.started_at),
            "finished_at": r.finished_at.map(epoch_micros_to_iso),
            "ok": r.ok.unwrap_or(false),
            "duration_ms": r.finished_at.and_then(|f| Some((f - r.started_at) / 1000)),
        })).collect::<Vec<_>>(),
    });
    ("200 OK", body.to_string())
}
```

### Dispatch wiring

Add to the GET dispatch arm in `routes.rs`:

```rust
if let Some((vertical_id, name)) = parse_workflow_runs_path(req) {
    return Some(routes_workflow_runs::list_runs_response(
        vertical_id, name, req, store
    ).await);
}
```

`parse_workflow_runs_path` matches:
```
GET /api/v1/verticals/<vertical_id>/workflows/<name>/runs[?...]
```

---

## Phase 2 — Web API client

**File**: `web/lib/api/workflow-runs.ts` (new)

```ts
export interface WorkflowRunsSummary {
  vertical_id: string;
  workflow_name: string;
  window_days: number;
  run_count: number;
  success_rate: number;             // 0.0 - 1.0
  avg_duration_ms: number;
  p95_duration_ms: number;
  last_run_at: string | null;
  runs: WorkflowRun[];
}

export interface WorkflowRun {
  id: string;
  started_at: string;
  finished_at: string | null;
  ok: boolean;
  duration_ms: number | null;
}

export async function fetchWorkflowRuns(args: {
  verticalId: string;
  name: string;
  days?: number;
  limit?: number;
}): Promise<WorkflowRunsSummary> {
  const params = new URLSearchParams();
  if (args.days) params.set("days", String(args.days));
  if (args.limit) params.set("limit", String(args.limit));
  const qs = params.toString();
  return apiGet<WorkflowRunsSummary>(
    `/api/v1/verticals/${args.verticalId}/workflows/${args.name}/runs${qs ? "?" + qs : ""}`
  );
}
```

---

## Phase 3 — Web page swap

**File**: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`

Replace the hardcoded `82` and `94%` divs:

```diff
+ const runsQuery = useQuery({
+   queryKey: ["workflow-runs", verticalId, name],
+   queryFn: () => fetchWorkflowRuns({ verticalId, name, days: 30 }),
+   staleTime: 30_000,
+ });
+ const stats = runsQuery.data;

  <div className="stats-grid">
    <div className="stat-cell">
-     <div style={...}>82</div>
+     <div style={...}>
+       {runsQuery.isPending ? <Skeleton width={48} height={36} /> : stats?.run_count ?? 0}
+     </div>
      <div>Runs in the last 30 days</div>
    </div>
    <div className="stat-cell">
-     <div>94%</div>
+     <div>
+       {runsQuery.isPending ? <Skeleton width={48} height={36} /> : `${Math.round((stats?.success_rate ?? 0) * 100)}%`}
+     </div>
      <div>Success rate</div>
    </div>
  </div>
```

Add 2 more cells (the new data the gateway returns):

```tsx
<div className="stat-cell">
  <div>{stats ? formatDuration(stats.avg_duration_ms) : "—"}</div>
  <div>Avg duration</div>
</div>
<div className="stat-cell">
  <div>{stats?.last_run_at ? relativeTime(stats.last_run_at) : "Never run"}</div>
  <div>Last run</div>
</div>
```

### Empty state

When `run_count === 0`:
- The "82" cell shows "0".
- The success-rate cell shows "—" not "0%" (zero of zero is not zero
  percent — it's undefined).
- Add a friendly "This workflow hasn't been run yet. Test it to see your
  first stats." nudge below the stat grid.

---

## Phase 4 — Recent runs table

Below the stats grid, the page should show the recent runs list (the
`runs: [...]` array from the response). Each row:

```tsx
<table>
  <thead>
    <tr><th>Run ID</th><th>Started</th><th>Duration</th><th>Status</th></tr>
  </thead>
  <tbody>
    {stats?.runs.map((r) => (
      <tr key={r.id}>
        <td><HashCell hash={r.id} /></td>         <!-- reuse from web-audit-feed-per-row-hash -->
        <td>{relativeTime(r.started_at)}</td>
        <td>{r.duration_ms ? formatDuration(r.duration_ms) : "Running…"}</td>
        <td><StatusPill tone={r.ok ? "success" : "destructive"}>
          {r.ok ? "Succeeded" : "Failed"}
        </StatusPill></td>
      </tr>
    ))}
  </tbody>
</table>
```

---

## Phase 5 — Tests

### Backend

**File**: `crates/argo-cli/src/handlers/gateway/routes_workflow_runs_tests.rs` (new):
1. `list_runs_returns_aggregates_and_recent_rows`
2. `list_runs_empty_workflow_returns_zero_counts`
3. `list_runs_respects_days_filter`
4. `list_runs_respects_limit`
5. `success_rate_computed_correctly`
6. `p95_duration_within_top_5pct_bucket`

### Frontend

**File**: `web/app/(app)/workflows/[verticalId]/[name]/page.test.tsx` (new):
7. `renders_run_count_from_gateway`
8. `renders_skeleton_while_loading`
9. `renders_empty_state_when_zero_runs`

---

## Phase 6 — Sample data fixture for dev

**File**: `web/lib/sample-workflow-runs.ts` (new)

For dev mode (no gateway), the API client falls back to a deterministic
sample fixture so the workflow page still looks real:

```ts
export const SAMPLE_WORKFLOW_RUNS_SUMMARY: WorkflowRunsSummary = {
  vertical_id: "pr_checker",
  workflow_name: "standard_review",
  window_days: 30,
  run_count: 47,                    // varied per workflow via deterministic hash
  success_rate: 0.91,
  avg_duration_ms: 4321,
  p95_duration_ms: 12543,
  last_run_at: "2026-05-20T14:32:00Z",
  runs: [/* 10 deterministic sample rows */],
};
```

The `apiGet` 404 fallback uses this. In strict-prod mode (per the wave-2
audit), the fallback is OFF and a real error surfaces.

---

## may-touch

- `crates/argo-cli/src/handlers/gateway/routes_workflow_runs.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes_workflow_runs_tests.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes.rs` — add GET dispatch arm.
- `web/lib/api/workflow-runs.ts` (new).
- `web/lib/sample-workflow-runs.ts` (new).
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` — replace hardcoded literals; add stats query.
- `web/app/(app)/workflows/[verticalId]/[name]/page.test.tsx` (new or extend).

## must-NOT-touch

- `workflow_run` table schema (migration 18) — already correct.
- The workflow builder page (`/build`) — separate concern.
- The workflow version-history rollback flow (`web-workflow-rollback-feature`).
- Other hardcoded literals in other pages — those are sibling cleanup briefs.

---

## Acceptance criteria

- [ ] `GET /api/v1/verticals/<v>/workflows/<n>/runs` returns 200 with the documented envelope.
- [ ] Empty workflow returns `{ run_count: 0, success_rate: 0, runs: [] }` without errors.
- [ ] `days` query param filters to that window.
- [ ] `limit` query param caps the recent-runs array length.
- [ ] Web `/workflows/<v>/<n>` page shows real `run_count` (no hardcoded 82).
- [ ] Web shows real `success_rate` (no hardcoded 94%).
- [ ] Web adds `avg duration` + `last run` cells.
- [ ] Web shows recent runs table with run IDs (using `<HashCell>`) + duration + status.
- [ ] Empty state shows friendly "hasn't been run yet" nudge.
- [ ] In strict-prod mode, no sample-data fallback (per wave-2 audit).
- [ ] 9 tests pass.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

---

## Gotchas

1. **`workflow_run.workflow_id` is a string key**: the convention is
   `"<vertical_id>/<name>"` (verify with `sqlite3 ~/.argo/store.db "SELECT
   DISTINCT workflow_id FROM workflow_run LIMIT 5"`). If the actual
   convention differs (some rows use UUIDs from `workflow_definition.id`),
   the brief needs adjustment — match what's actually in the table.
2. **Time units in the table are MICROS** (`started_at INTEGER`).
   Duration calculations are micros → ms requires `/1000`.
3. **P95 calculation is approximation**: for a workflow with 10,000 runs
   in 30 days, this query LOADS ALL durations into memory. For v1 that's
   acceptable (workflow runs are bounded); v2 should use SQLite's
   `percentile_disc()` if available or an approximation algorithm.
4. **Demo data seeding**: a fresh demo workspace has zero runs. To make
   the demo look populated, the operator should seed `workflow_run` rows
   via `argo workflow seed-demo` (sibling brief, not in scope here) OR
   the dev-mode sample fallback handles it.
5. **Run-row click navigates to drill-in**: each row in the recent-runs
   table should link to `/workflows/<v>/<n>/runs/<run_id>` for the
   full run detail. That page is a sibling brief
   (`web-workflow-run-detail-page`, not yet authored). For v1, the brief
   may leave links pointing to "#" with a TODO.
6. **`HashCell` dependency**: this brief depends on `web-audit-feed-per-row-hash`
   landing first (which adds the `<HashCell>` component). If that brief
   isn't merged, use a simpler inline `<code>{r.id.slice(0, 8)}…</code>`
   stopgap.
7. **`success_rate` for 0 runs**: don't return `NaN` or `0.0` — the
   frontend renders "—" for the dashes case. Document: `success_rate`
   is `null` when `run_count === 0`.

Actually: the brief above returns `0.0` for empty. Update the JSON to use
`null` for empty:

```diff
- "success_rate": success_rate,
+ "success_rate": if run_count > 0 { json!(success_rate) } else { json!(null) },
```

and the web client handles `null` → "—" display.

---

## References

- Audit: `docs/skycot/audit/user-journey/2026-05-20-wave-15-final-verdict.md` (the hardcoded-literal finding)
- Schema: `crates/argo-store/migrations/18__workflow_runs.sql`
- Sibling for HashCell: `web-audit-feed-per-row-hash`
- Wave-2 isStrictProduction: `web/lib/api/approvals.ts` (the fallback gate)
- Page: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`
