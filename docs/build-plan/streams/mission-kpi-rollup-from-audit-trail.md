---
brief-id: mission-kpi-rollup-from-audit-trail
owner: claude
priority: P1
status: TODO
authored-by: claude conductor (scheduled-task T18-06Z, deepening pass)
authored-against: origin/main 5ba0c54cd on 2026-05-25
loc-estimate: 250-400 LOC (scheduler + rollup logic + sql queries + 5 tests + migration v81)
validate-before-claim: bash scripts/check-brief-freshness.sh mission-kpi-rollup-from-audit-trail
mission-claim: 2 (Delegation is the unit — KPIs are the worker's delegation contract)
demo-blocking: YES — Mariah's KPI tiles on /workers/{id} must reflect REAL audit data per no-theater amendment 2026-05-24
parallel-surface-check: |
  rg 'fn rollup|KpiRollup|kpi_rollup' crates/ → no hits (clean — no existing parallel surface)
  rg 'pub struct.*Scheduler' crates/argo-orchestrator/src/ → DutyScheduler only (sibling pattern OK)
---

# mission-kpi-rollup-from-audit-trail

**Authored against**: origin/main `5ba0c54cd` on `2026-05-25`.
**Validate before claim**: bash scripts/check-brief-freshness.sh mission-kpi-rollup-from-audit-trail

**Mission claim 2 enforcement.** `Worker::kpis: Option<serde_json::Value>` exists today (`crates/argo-identity/src/types.rs:305`) but is set to `None` at hire and never populated. The Mariah-card / worker-detail UI tiles bind to it. **The v1.0 demo MP4 cannot ship until those tiles show real numbers** — per operator amendment 2026-05-24 the demo must be real, not theatrical. This brief wires a periodic rollup that derives `worker.kpis` from the worker's audit_record trail and persists it through the existing `IdentityStore::update_charter` path (no new mutation primitive).

## Audit trail facts (verified 2026-05-25)

- `audit_record(seq, kind, payload_json, created_at)` is the source-of-truth row (`crates/argo-store/src/session_query.rs:303` and 4 other call-sites). Already tenant-scoped via `argo-store` tenant-bind sweep.
- `AuditKind::ApprovalAsked` payload: `{ action_id, worker_id, channel, target, risk_level, requested_at, expiry_ts }` (`crates/argo-audit/src/event_kind.rs:58-60`).
- `AuditKind::ApprovalReplied` payload: `{ action_id, channel, replier, reply_kind, reply_body, replied_at }` (event_kind.rs:69-72). **action_id correlates to ApprovalAsked → worker_id is recoverable by join.**
- `AuditKind::TurnStart` / `ToolCall` — payloads include `worker_id` directly.
- `IdentityStore::update_charter(worker_id, tenant_id, charter, kpis)` already exists and writes `kpi_json` (`crates/argo-identity/src/store.rs:374`). Use it; do not invent a parallel writer.

## Concrete deliverable

### 1. KPI JSON schema (new — pinned in this brief, no parallel definition)

```json
{
  "approvals_handled": 42,           // count(ApprovalReplied WHERE worker join)
  "auto_approval_rate": 0.83,        // count(auto_resolved) / count(ApprovalAsked + auto_resolved)
  "mean_response_time_seconds": 47,  // avg(ApprovalReplied.replied_at - ApprovalAsked.requested_at)
  "sessions_24h": 12,                // count(TurnStart in last 24h for this worker_id)
  "tool_calls_24h": 73,              // count(ToolCall in last 24h for this worker_id)
  "rollup_window_end_utc": "2026-05-25T18:00:00Z",
  "rollup_schema_version": 1
}
```

Schema version is bumped on additive field changes; consumers (Mariah card, /workers/{id}) read what they know and ignore the rest.

### 2. New module — `crates/argo-orchestrator/src/kpi_rollup.rs` (sibling to `duty_scheduler.rs`)

```rust
pub struct KpiRollupScheduler {
    store: Arc<OrchestratorStore>,
    identity_store: Arc<IdentityStore>,  // for update_charter
    audit_sink: Arc<dyn AuditSink>,
    interval: Duration,                   // default 300s (5 min)
}

impl KpiRollupScheduler {
    pub async fn tick(&self, now: DateTime<Utc>) -> ArgoResult<KpiRollupReport> {
        // 1. Enumerate (tenant_id, worker_id) tuples with audit rows since last_rollup_at
        // 2. For each, run 5 aggregation queries (one per KPI) — all tenant-bound
        // 3. Build KpiPayload JSON
        // 4. Persist via IdentityStore::update_charter(worker_id, tenant_id, None, Some(&kpi_json))
        // 5. Emit AuditKind::WorkerKpiRolled audit row per worker rolled
    }
}

pub fn spawn_kpi_rollup_scheduler(
    sched: Arc<KpiRollupScheduler>,
    shutdown: CancellationToken,
) -> JoinHandle<()> { ... }
```

Wire into `crates/argo-cli/src/handlers/gateway/boot_spawns.rs` alongside `spawn_duty_scheduler` (~line 172). Default `KPI_ROLLUP_INTERVAL_SECS=300`, overridable via `argo.toml` `[kpi_rollup]` section.

### 3. New AuditKind — `WorkerKpiRolled`

Add to `crates/argo-audit/src/event_kind.rs` after `ApprovalReplied`. Payload: `{ worker_id, tenant_id, kpis_json, rolled_at, rows_scanned }`. Update `event.rs` `label()` / `from_label()` / `all()`.

### 4. Migration v81 — add `worker.kpis_rolled_at TIMESTAMP NULL`

New file: `crates/argo-store/src/migrations/v81.rs`. Additive (NULL default), no backfill needed. Register in `migrations/mod.rs` line 79 (after `mod v80;`) and `registry_compliance.rs` (after the v80 block at line 126). Latest registered slot is 80 (verified by `bash scripts/check-migration-slots.sh`); 81 is the next free slot.

### 5. Tenant-isolation: load-bearing

Every aggregation query MUST bind `tenant_id` in the WHERE clause. The scheduler enumerates `(tenant_id, worker_id)` tuples, never `worker_id` alone. Use existing `argo-store` tenant-bound query helpers (mirror the pattern at `crates/argo-store/src/session_query.rs:303`).

### 6. Regression tests (in `crates/argo-orchestrator/tests/kpi_rollup_test.rs`)

1. **`rollup_increments_approvals_handled_after_replied`** — seed 3 ApprovalAsked+ApprovalReplied pairs for worker_A, tick, assert `worker_A.kpis["approvals_handled"] == 3`.
2. **`rollup_tenant_isolated`** — same WorkerId text across two TenantIds, distinct audit volumes; assert each tenant's worker gets its OWN KPI roll, no cross-tenant bleed.
3. **`rollup_emits_audit_row_per_worker_per_tick`** — 5 workers with activity, tick, assert 5 × `WorkerKpiRolled` rows.
4. **`rollup_24h_window_drops_stale_rows`** — seed ToolCall row dated 25h ago + ToolCall dated 1h ago; assert `tool_calls_24h == 1`.
5. **`rollup_skipped_if_no_audit_activity_since_last_rollup`** — first tick produces rows, second tick with no new audit activity produces zero new `WorkerKpiRolled` rows (uses `kpis_rolled_at` watermark).

### 7. Wire un-ignore (none expected) and verify against gap manifest

After merge, update `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` Claim-2 KPI row from 🟡 P2 to 🟢 SHIPPED with PR # cite.

## Out of scope (explicit deferrals)

- Streaming / real-time KPI updates (this is batch rollup; 5-min granularity is enough for demo).
- KPI display UI changes — the Mariah card / `/workers/{id}` tiles already bind to `worker.kpis`; this brief makes them non-null. Visual polish stays out.
- KPI alerting / threshold breach (P3 future).
- Cross-worker KPI aggregations (team-headline already covers fleet-level).

## Acceptance gates

- [ ] `cargo test -p argo-orchestrator kpi_rollup` — 5/5 pass.
- [ ] `cargo test --workspace --no-fail-fast` — green.
- [ ] `bash scripts/pre-merge-gate.sh` — green (LOC budget + clippy + migration-slot validator + tenant-bind sweep).
- [ ] Manual: `cargo run -p argo-cli -- gateway start` then trigger 3 approvals on a demo worker, wait 6 min, `curl /api/v1/workers/{id}` returns non-null `kpis` block with `approvals_handled >= 3`.
- [ ] After merge, gap-manifest row updated (PR body lists the line change).

## Why not Option B (per-write incremental rollup)

Considered: increment `worker.kpis` inside the audit sink on each ApprovalReplied. Rejected because (a) audit sink is a hot path — 5 SQL writes per audit row breaks the chain-hash latency budget, (b) makes audit-record the source-of-truth + denormalised copy a derived view, (c) recovering from a corrupted rollup requires recomputing from audit anyway, so batch is closer to the steady-state pattern.

## Validate before claim

```bash
bash scripts/check-brief-freshness.sh mission-kpi-rollup-from-audit-trail
bash scripts/check-migration-slots.sh  # confirm 81 is still free
rg 'pub struct.*KpiRollup' crates/      # confirm zero hits before authoring
```
