# web-gateway-endpoints-worker-settings-patch — STATUS: 4/4 SHIPPED, 2 follow-ups remain

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-worker-settings-patch`.
**Priority**: P2 — the 4 originally-scoped PATCH handlers SHIPPED. Two
follow-ups remain: (1) move from audit-row-derived reads to a real
`worker_settings` table so reads don't scan `audit_record`; (2) add the
5th `approval-policy` PATCH section deferred from the original brief.
**Sizing**: ~500-700 LOC across new `worker_settings` table + migration +
read-path swap + approval-policy PATCH + tests.

---

## Status — 4/4 PATCH endpoints SHIPPED

| Endpoint | Status | Landed via |
|----------|--------|------------|
| `PATCH /channels` | ✅ SHIPPED | `routes_worker_settings_read::patch_channels` |
| `PATCH /quiet-hours` | ✅ SHIPPED | `routes_worker_settings_read::patch_quiet_hours` |
| `PATCH /budget` | ✅ SHIPPED | `routes_worker_settings_read::patch_budget` |
| `PATCH /autonomy` | ✅ SHIPPED | `routes_worker_settings_read::patch_autonomy` |
| `GET /settings` | ✅ SHIPPED | `routes_worker_settings_read::get_settings` |
| `POST /pause` / `POST /resume` / `POST /archive` / `DELETE /` | ✅ SHIPPED | `routes_worker_settings_danger::*` |

The v1 implementation persists settings as **audit-row markers** (`worker.<facet>.changed`)
rather than to a dedicated `worker_settings` table, then derives the
"current" value at read time by reading the most recent audit row per facet.
This works but has 3 weaknesses:

1. **Read scan cost**: every `GET /settings` walks `audit_record` filtered to
   `worker.*.changed` kinds. For a worker with 1,000 setting-changes over its
   lifetime, this is a 1,000-row scan per GET.
2. **No referential integrity**: settings live in audit rows whose payload
   structure can drift without a compile-time check.
3. **No bulk read**: cohort-wide queries ("show me all workers with conservative
   autonomy") require a full audit_record scan.

This follow-up brief addresses (1)+(2)+(3) by adding a real `worker_settings`
table that the PATCH handlers ALSO write to (keep audit emission — they're
the chain-of-truth), and the GET handler reads from instead of the audit scan.

---

## Follow-up A — persist to dedicated `worker_settings` table

### Phase A.1 — Migration

**File**: `crates/argo-store/migrations/NNN__worker_settings.sql` (next free slot)

```sql
-- Stream `web-gateway-endpoints-worker-settings-patch` — explicit
-- worker-settings persistence. Settings are also audit-row-emitted
-- (chain of truth), but the read path now hits this table instead of
-- scanning audit_record. Single row per (tenant, worker).
--
-- Slot rationale: filename slot N (current highest + 1). Versioned
-- slot vNN (current highest + 1).

CREATE TABLE IF NOT EXISTS worker_settings (
    tenant_id            TEXT      NOT NULL,
    worker_id            TEXT      NOT NULL,
    channels_json        TEXT      NOT NULL DEFAULT '{}',
    quiet_hours_json     TEXT      NOT NULL DEFAULT '{}',
    budget_json          TEXT      NOT NULL DEFAULT '{}',
    autonomy_level       TEXT      NOT NULL DEFAULT 'balanced'
                              CHECK(autonomy_level IN ('conservative','balanced','aggressive')),
    approval_policy_json TEXT      NOT NULL DEFAULT '{}',
    updated_at           INTEGER   NOT NULL,
    PRIMARY KEY (tenant_id, worker_id)
);

CREATE INDEX IF NOT EXISTS idx_worker_settings_autonomy
    ON worker_settings(tenant_id, autonomy_level);
```

The CHECK constraint on `autonomy_level` matches the existing handler's
validation (`conservative` / `balanced` / `aggressive`). JSON columns let
the 4 nested-shape facets evolve without further migrations.

### Phase A.2 — Store module

**File**: `crates/argo-store/src/worker_settings.rs` (new) — ~150 LOC.

```rust
//! Per-worker settings row — single row per (tenant, worker). PATCH
//! handlers in argo-cli/gateway write here AS WELL AS emitting an
//! audit row (the chain is the source of truth; this table is the
//! fast read cache).

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerSettings {
    pub tenant_id: TenantId,
    pub worker_id: String,
    pub channels: Value,             // { slack, im, email, web_inbox }
    pub quiet_hours: Value,           // { start_local, end_local, timezone }
    pub budget: Value,                // { monthly_usd_cap, alert_at_pct }
    pub autonomy_level: String,
    pub approval_policy: Value,       // see Follow-up B
    pub updated_at_micros: i64,
}

pub async fn get_or_default(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
) -> ArgoResult<WorkerSettings> {
    let row = sqlx::query(
        "SELECT channels_json, quiet_hours_json, budget_json, autonomy_level, \
                approval_policy_json, updated_at \
         FROM worker_settings WHERE tenant_id = ? AND worker_id = ?",
    )
    .bind(tenant.to_string()).bind(worker_id)
    .fetch_optional(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("get_or_default: {e}")))?;
    if let Some(row) = row {
        Ok(WorkerSettings {
            tenant_id: tenant.clone(),
            worker_id: worker_id.to_string(),
            channels: parse_json_col(&row, "channels_json")?,
            quiet_hours: parse_json_col(&row, "quiet_hours_json")?,
            budget: parse_json_col(&row, "budget_json")?,
            autonomy_level: row.try_get("autonomy_level").unwrap_or_else(|_| "balanced".into()),
            approval_policy: parse_json_col(&row, "approval_policy_json")?,
            updated_at_micros: row.try_get("updated_at").unwrap_or(0),
        })
    } else {
        // No row yet → return defaults.
        Ok(default_settings(tenant.clone(), worker_id.to_string()))
    }
}

pub async fn upsert_facet(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
    facet: SettingsFacet,
    value: &Value,
) -> ArgoResult<()> {
    let col = facet.column_name();
    let now = chrono::Utc::now().timestamp_micros();
    // Upsert with COALESCE for non-target columns so we don't trash
    // existing facets.
    let sql = format!(
        "INSERT INTO worker_settings \
         (tenant_id, worker_id, {col}, updated_at) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(tenant_id, worker_id) DO UPDATE SET \
         {col} = excluded.{col}, updated_at = excluded.updated_at"
    );
    sqlx::query(&sql)
        .bind(tenant.to_string())
        .bind(worker_id)
        .bind(value.to_string())
        .bind(now)
        .execute(store.pool()).await
        .map_err(|e| ArgoError::Provider(format!("upsert_facet: {e}")))?;
    Ok(())
}

pub enum SettingsFacet { Channels, QuietHours, Budget, Autonomy, ApprovalPolicy }
impl SettingsFacet {
    fn column_name(&self) -> &'static str {
        match self {
            Self::Channels => "channels_json",
            Self::QuietHours => "quiet_hours_json",
            Self::Budget => "budget_json",
            Self::Autonomy => "autonomy_level",
            Self::ApprovalPolicy => "approval_policy_json",
        }
    }
}
```

### Phase A.3 — Wire into existing PATCH handlers

Each of the 4 shipped handlers in `routes_worker_settings_read.rs` already
emits an audit row. Add a `worker_settings::upsert_facet(...)` call
immediately before the audit-emit line so the table stays in sync.

```diff
  // In patch_channels (and the 3 siblings):
  validate_body(...)?;
+ argo_store::worker_settings::upsert_facet(
+     store, &tenant, worker_id,
+     SettingsFacet::Channels,
+     &channels_value,
+ ).await?;
  sink.emit(AuditKind::WorkerChannelsChanged, payload).await?;
  // ... existing 200 OK return ...
```

### Phase A.4 — Switch GET to fast read

`get_settings` currently scans `audit_record` for the latest row per facet.
Replace with one call:

```rust
let settings = argo_store::worker_settings::get_or_default(
    store, &tenant, worker_id
).await?;
("200 OK", serialize(&settings).to_string())
```

### Phase A.5 — Backfill migration (one-time)

For workspaces that already have audit-row settings but no `worker_settings`
row, run a one-time backfill at boot:

**File**: `crates/argo-store/src/worker_settings_backfill.rs` (new)

Walks `worker` table, for each worker reads the latest `worker.<facet>.changed`
audit row per facet, writes into `worker_settings`. Idempotent.

Run once at gateway boot, gated by a boolean flag stored in a
`migration_marker` row so it doesn't re-run on subsequent boots.

---

## Follow-up B — add `PATCH /api/v1/workers/<id>/settings/approval-policy`

### Phase B.1 — Payload schema

Body:
```json
{
  "require_approval_above_usd": 50,
  "auto_approve_below_usd": 5,
  "approver_channel": "slack",
  "escalation_after_minutes": 30,
  "auto_reject_on_timeout": false
}
```

All fields optional — handler treats omitted fields as "keep current value."
Validation:
- `require_approval_above_usd >= auto_approve_below_usd`
- `escalation_after_minutes > 0`
- `approver_channel ∈ { "slack", "email", "im", "voice", "web_inbox" }`

### Phase B.2 — Handler

Add to `routes_worker_settings_read.rs`:

```rust
pub(crate) async fn patch_approval_policy(
    worker_id: &str,
    body: &str,
    req: &str,
    store: &SessionStore,
    sink: &dyn AuditSink,
) -> (&'static str, String) {
    let principal = principal_from_header(req).unwrap_or_else(|| "anonymous".into());
    let tenant = match helpers::resolve_workspace_tenant(store).await { /* ... */ };
    if let Err(r) = helpers::cross_tenant_guard(store, &tenant, worker_id).await { return r; }

    let opts: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return helpers::bad_request(&format!("invalid body: {e}")),
    };
    if let Err(msg) = validate_approval_policy(&opts) {
        return helpers::bad_request(&msg);
    }

    let current = argo_store::worker_settings::get_or_default(store, &tenant, worker_id).await
        .unwrap_or_default();
    // Merge new fields into current (preserves omitted fields).
    let mut merged = current.approval_policy.clone();
    for (k, v) in opts.as_object().unwrap_or(&serde_json::Map::new()) {
        merged[k] = v.clone();
    }
    argo_store::worker_settings::upsert_facet(
        store, &tenant, worker_id,
        SettingsFacet::ApprovalPolicy,
        &merged,
    ).await?;

    sink.emit(AuditKind::WorkerApprovalPolicyChanged, json!({
        "actor": principal,
        "tenant_id": tenant.to_string(),
        "worker_id": worker_id,
        "new_value": merged,
    })).await?;

    ("200 OK", json!({ "ok": true, "approval_policy": merged }).to_string())
}
```

### Phase B.3 — Audit variant

Add `AuditKind::WorkerApprovalPolicyChanged` (4-site recipe). Label:
`"worker.approval_policy.changed"`.

### Phase B.4 — Tests

Add 4 tests to `routes_worker_settings_tests.rs`:
1. `patch_approval_policy_round_trips`
2. `patch_approval_policy_validates_threshold_ordering`
3. `patch_approval_policy_validates_channel_enum`
4. `patch_approval_policy_partial_update_preserves_other_fields`

---

## may-touch

- `crates/argo-store/migrations/NNN__worker_settings.sql` (new).
- `crates/argo-store/src/migrations/vNN.rs` (new).
- `crates/argo-store/src/worker_settings.rs` (new) + tests file.
- `crates/argo-store/src/worker_settings_backfill.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes_worker_settings_read.rs` — wire `upsert_facet` into existing 4 handlers; add `patch_approval_policy`.
- `crates/argo-cli/src/handlers/gateway/routes_worker_settings.rs` — re-export `patch_approval_policy`.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` (or wherever PATCH is dispatched) — add the new arm.
- `crates/argo-audit/src/event.rs` — add `WorkerApprovalPolicyChanged` variant.
- `crates/argo-cli/src/handlers/gateway/boot_serve.rs` — wire backfill on boot.
- `crates/argo-cli/src/handlers/gateway/routes_worker_settings_tests.rs` — add 4 new tests.
- `web/lib/api/worker-settings.ts` — add `updateApprovalPolicy(workerId, policy)` API client function.

## must-NOT-touch

- The 4 shipped PATCH handler signatures — extend internally only.
- Audit row format for the existing 4 facets — keep emitting same payloads.
- `worker` table — `worker.channels` keeps its existing column for back-compat.
- `routes_worker_settings_danger.rs` (pause/resume/archive/delete) — separate concern.

---

## Acceptance criteria

- [ ] `worker_settings` table exists after migration.
- [ ] All 4 existing PATCH handlers ALSO write to `worker_settings` (verified by `SELECT * FROM worker_settings` after PATCH).
- [ ] `GET /settings` reads from `worker_settings` (no longer scans `audit_record` for the read path).
- [ ] Backfill runs once on boot, idempotently — second boot is a no-op.
- [ ] `PATCH /api/v1/workers/<id>/settings/approval-policy` returns 200 + `{ ok, approval_policy }`.
- [ ] Invalid policy (auto_approve >= require_approval) returns 400.
- [ ] Partial PATCH preserves omitted fields.
- [ ] Audit row `kind: "worker.approval_policy.changed"` is emitted.
- [ ] Cross-tenant attempt returns 404.
- [ ] `cargo clippy -p argo-store -p argo-audit -p argo-cli --all-targets -- -D warnings` clean.
- [ ] 4 new tests pass (approval-policy) + 4 new store-table tests pass.

---

## Gotchas

1. **Backward compatibility**: workspaces that have NO `worker_settings`
   row should still get sensible defaults from `GET /settings`. The
   `get_or_default` helper returns defaults for missing rows — don't
   delete that fallback.
2. **`worker.channels` lives on the `worker` table** in v1; the new
   `channels_json` column on `worker_settings` is the new source of truth.
   The brief's PATCH handler should write to BOTH for one release, then a
   follow-up brief drops the old column. v1 dual-write is intentional.
3. **Migration ordering**: the migration must run BEFORE the backfill. The
   versioned migration runner guarantees this — the backfill is run
   AT BOOT, after `SessionStore::migrate` returns.
4. **Backfill idempotency**: the `migration_marker` table check ensures
   the backfill doesn't re-run. If a worker's settings have been changed
   AFTER backfill, re-running would clobber. Don't.
5. **JSON merge semantics on partial PATCH**: the brief uses shallow merge
   (object-level), not deep merge. Operator PATCH'ing `{ approver_channel: "email" }`
   updates only that field; nested objects (none yet, but for future-
   proofing) are replaced wholesale. Document this in the OpenAPI spec.
6. **CHECK constraint on `autonomy_level`**: this is the only column with a
   DB-level enum check. JSON columns can hold anything; the handler is
   responsible for validating shapes.

---

## References

- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`
- Audit: `docs/skycot/audit/forms/2026-05-20-wave-4-forms.md`
- Shipped substrate: `crates/argo-cli/src/handlers/gateway/routes_worker_settings*.rs`
- Sibling: `web-form-validation-zod-sweep` (client side)
- Audit kind pattern: `crates/argo-audit/src/event.rs` (search `WorkerChannelsChanged`)
