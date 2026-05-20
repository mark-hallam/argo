# web-worker-trust-promotion-engine — make trust grade actually move

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-worker-trust-promotion-engine`.
**Priority**: P1 — wave-15 user-journey audit found the trust ratchet is a
**static observatory**: the grade is set once at hire, displayed forever, and
never changes. The mission claim "trust ratchet earns trust over time" is
unbacked.
**Sizing**: ~1100-1400 LOC across a new `argo-trust-engine` crate + audit (1
new kind) + identity (1 new method) + cli spawn wiring + 1 gateway-readable
recompute endpoint + 12 unit tests + 1 cross-tenant negative test + 1 E2E.

---

## Why this brief exists

Wave-15 audit (`docs/skycot/audit/user-journey/2026-05-20-wave-15-final-verdict.md`):

> **Trust ratchet is a STATIC OBSERVATORY** — `worker.trust_grade` is set
> at hire time and never updated. There is no promotion engine, no
> demotion path, no signal aggregator. The /trust page renders an
> elaborate UI explaining "what would unlock the next rung" but no code
> ever advances the rung. Marketing claim "your AI hires earn trust over
> time" is broken.

The substrate is rich:
- `trust_observation` table (cross-tenant quality observations) — v50 / migration 54.
- `trust_streak` view — counts consecutive `approval.granted` rows per worker+tool.
- `worker.trust_grade` u8 column — write surface via
  `IdentityStore::set_worker_trust_grade` exists, but nothing CALLS it
  outside of test setup and the hire-time default.
- `AuditKind::ApprovalGranted` / `ApprovalRevoked` / `ApprovalReplied` /
  `EscalationRequired` / `RbacDeny` — the input signal stream.

What's missing: a periodic engine that reads the signals, computes a new grade,
and writes it back. This brief delivers that engine.

---

## Pre-flight verification

```bash
# 1. Confirm the read substrate is still where the brief expects.
rg "pub async fn set_worker_trust_grade" crates/argo-identity/src/store_trust_grade.rs
rg "pub async fn streaks_for_worker" crates/argo-store/src/trust_streak.rs
rg "AuditKind::ApprovalGranted|AuditKind::ApprovalReplied" crates/argo-audit/src/event.rs

# 2. Confirm no parallel promotion engine has been authored.
rg "trust_promotion|promote_worker_trust|TrustPromotionEngine" crates/

# 3. Confirm the spawn site for forge_cron (the pattern this brief copies).
rg "spawn_forge_cron" crates/argo-cli/src/handlers/gateway/boot_serve.rs

# 4. Confirm /trust page is currently read-only (so no UI conflict).
rg "trust_grade.*useState|set.*TrustGrade" web/app/\(app\)/workers/
```

If any of these fail unexpectedly, stop and reconcile.

---

## Architecture

### New crate: `argo-trust-engine`

A standalone crate keeps the promotion logic isolated from `argo-skill` (where
the grade enum lives) and from `argo-identity` (where the write API lives), so
the engine can evolve without churning either. Follows the
`argo-forge` precedent (separate crate that owns its cron config + tick body
and depends on `argo-store` + `argo-types`).

```
crates/argo-trust-engine/
├── Cargo.toml
├── src/
│   ├── lib.rs              # public surface (TrustPromotionConfig, spawn fn)
│   ├── cron.rs             # tick loop + scheduler glue (mirrors argo-forge/src/cron.rs)
│   ├── scoring.rs          # signal aggregation + score computation
│   ├── ratchet.rs          # score → TrustGrade mapping + hysteresis logic
│   └── recompute.rs        # one-shot recompute for a single worker (gateway uses this)
└── tests/
    ├── scoring_tests.rs
    ├── ratchet_tests.rs
    └── flap_resistance_tests.rs
```

LOC budget — each file targets ~150-300 LOC. `cron.rs` is closest to copy-
modify of `argo-forge/src/cron.rs`; `scoring.rs` + `ratchet.rs` are new.

### Signal sources (read-only)

The engine consumes 5 signal streams (all already populated, none new):

| Signal | Source | Direction | Weight (initial) |
|--------|--------|-----------|------------------|
| Approval streak length | `trust_streak_event` view | +score per row | +0.5 per consecutive granted |
| Approval rate (30d) | `audit_record` filtered to `ApprovalGranted` / `ApprovalReplied` | +score for granted, −score for rejected | +1.0 / −2.0 |
| Escalation count (30d) | `audit_record` filtered to `EscalationRequired` | −score | −1.0 per row |
| RBAC denial count (30d) | `audit_record` filtered to `RbacDeny` | −score | −0.5 per row |
| Cross-tenant quality avg | `trust_observation` filtered to source = worker's tenant, last 90d | +score linear in mean | +5.0 × mean(quality) |

These weights are NOT operator-tunable in v1 — they live as named consts in
`scoring.rs` so future briefs can flip to env-vars without API churn.

### Score → grade mapping (with hysteresis)

The 5 trust grades are (from `argo_skill::trust::TrustGrade`):

| u8 | Grade | Promote-from-below threshold | Demote-from-above threshold |
|----|-------|-------------------------------|------------------------------|
| 0  | LearnedUnreviewed (default at hire) | — | — |
| 1  | PublishedCommunity            | score ≥ 2.0  | score < 1.0  |
| 2  | Tenant                        | score ≥ 5.0  | score < 3.5  |
| 3  | PublishedVerified             | score ≥ 10.0 | score < 8.0  |
| 4  | Builtin (max trust)           | score ≥ 20.0 | score < 17.0 |

**Hysteresis** = the gap between promote-up and demote-back thresholds.
Without it, a worker hovering at score 1.99 / 2.01 flaps grades every tick.
The 1.0-wide gap means one bad day at grade 2 (Tenant) drops the score from
~5.5 to ~4.0 but does NOT demote — the demote-below threshold is 3.5.

**Promotion ladder rule** — the engine moves AT MOST one rung per tick, so a
worker that swings score from 0.5 to 12.0 in one window still climbs
gradually. This makes the journey visible in `/audit` (3 separate
`TrustGradeChanged` rows over 3 ticks, not one mystery jump).

### Read-side stays unchanged

`GET /api/v1/workers/<id>/trust-grade` already serves `worker.trust_grade` from
the column. Once the engine writes back, that endpoint returns the fresh grade
on the next poll. **No web changes needed for grade display** — the existing
`useWorkerTrustGrade()` hook will see the new values automatically.

What DOES need a small web change: removing the disclaimer text
"Trust grade is derived from observed behavior, not operator-tunable" needs
the second clause kept (still not operator-tunable) but the first clause
becomes load-bearing instead of aspirational. The /trust page is updated to
show "Recomputed every 30 min" and surface the **last_promoted_at /
last_demoted_at** timestamps.

---

## Phase 1 — `argo-trust-engine` crate skeleton

### 1a. Cargo manifest

**File**: `crates/argo-trust-engine/Cargo.toml` (new)

```toml
[package]
name = "argo-trust-engine"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
argo-audit.workspace = true
argo-identity.workspace = true
argo-skill.workspace = true
argo-store.workspace = true
argo-types.workspace = true
chrono.workspace = true
serde.workspace = true
serde_json.workspace = true
sqlx.workspace = true
tokio = { workspace = true, features = ["rt", "macros", "time"] }
tracing.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["rt", "macros", "test-util", "time"] }
```

Add to workspace members in `Cargo.toml` (root) — alpha-sorted between
`argo-types` and the next crate.

### 1b. Public surface

**File**: `crates/argo-trust-engine/src/lib.rs`

```rust
//! Periodic trust-grade recomputation for workers.
//!
//! Reads 5 signal streams (approval streak, approval rate, escalation
//! count, RBAC denial count, cross-tenant quality avg), aggregates a
//! score per worker, maps the score to a [`argo_skill::TrustGrade`]
//! with hysteresis, and writes the result back via
//! [`argo_identity::IdentityStore::set_worker_trust_grade`].
//!
//! Spawn the cron loop with [`spawn_trust_engine_cron`]; one-shot
//! recompute is available via [`recompute::recompute_worker`].

pub mod cron;
pub mod ratchet;
pub mod recompute;
pub mod scoring;

pub use cron::{spawn_trust_engine_cron, TrustEngineConfig};
pub use ratchet::{ratchet_grade, RatchetDecision};
pub use scoring::{compute_score, ScoreBreakdown};
```

### 1c. Cron loop

**File**: `crates/argo-trust-engine/src/cron.rs`

Copy-modify `crates/argo-forge/src/cron.rs`. Replace:
- `ForgeCronConfig` → `TrustEngineConfig`
- `DEFAULT_INTERVAL_SECS = 3_600` → `DEFAULT_INTERVAL_SECS = 1_800` (30 min)
- env-var prefix `ARGO_FORGE_CRON_*` → `ARGO_TRUST_ENGINE_*`
- The `tick` body calls `recompute::tick_all_workers(store).await?`

```rust
pub fn spawn_trust_engine_cron(
    store: Arc<SessionStore>,
    cfg: TrustEngineConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(cfg.interval_secs.max(1)));
        loop {
            ticker.tick().await;
            if let Err(e) = tick(Arc::clone(&store), &cfg).await {
                tracing::warn!(target: "trust_engine::cron", error = %e, "trust-engine tick failed");
            }
        }
    })
}

pub(crate) async fn tick(
    store: Arc<SessionStore>,
    cfg: &TrustEngineConfig,
) -> ArgoResult<usize> {
    if cfg.dry_run {
        return crate::recompute::tick_all_workers_dry_run(store, cfg).await;
    }
    crate::recompute::tick_all_workers(store, cfg).await
}
```

### 1d. Score computation

**File**: `crates/argo-trust-engine/src/scoring.rs`

```rust
//! Signal aggregation. All 5 signals are read read-only from `audit_record`
//! and `trust_observation`. No writes here.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

pub const WEIGHT_APPROVAL_STREAK: f32 = 0.5;
pub const WEIGHT_APPROVAL_GRANTED: f32 = 1.0;
pub const WEIGHT_APPROVAL_REJECTED: f32 = -2.0;
pub const WEIGHT_ESCALATION: f32 = -1.0;
pub const WEIGHT_RBAC_DENY: f32 = -0.5;
pub const WEIGHT_QUALITY_AVG: f32 = 5.0;

pub const WINDOW_DAYS_ACTIONS: i64 = 30;
pub const WINDOW_DAYS_QUALITY: i64 = 90;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub worker_id: String,
    pub streak_score: f32,
    pub approval_score: f32,
    pub escalation_score: f32,
    pub rbac_score: f32,
    pub quality_score: f32,
    pub total: f32,
    pub computed_at: chrono::DateTime<Utc>,
}

pub async fn compute_score(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
) -> ArgoResult<ScoreBreakdown> {
    let now = Utc::now();
    let action_cutoff = now - Duration::days(WINDOW_DAYS_ACTIONS);
    let quality_cutoff = now - Duration::days(WINDOW_DAYS_QUALITY);

    let streak = longest_streak(store, tenant, worker_id).await?;
    let (granted, rejected) = approval_counts(store, tenant, worker_id, action_cutoff).await?;
    let escalations = escalation_count(store, tenant, worker_id, action_cutoff).await?;
    let rbac_denies = rbac_deny_count(store, tenant, worker_id, action_cutoff).await?;
    let quality_avg = cross_tenant_quality_avg(store, tenant, quality_cutoff).await?;

    let streak_score = streak as f32 * WEIGHT_APPROVAL_STREAK;
    let approval_score = granted as f32 * WEIGHT_APPROVAL_GRANTED
        + rejected as f32 * WEIGHT_APPROVAL_REJECTED;
    let escalation_score = escalations as f32 * WEIGHT_ESCALATION;
    let rbac_score = rbac_denies as f32 * WEIGHT_RBAC_DENY;
    let quality_score = quality_avg * WEIGHT_QUALITY_AVG;

    Ok(ScoreBreakdown {
        worker_id: worker_id.to_string(),
        streak_score, approval_score, escalation_score, rbac_score, quality_score,
        total: streak_score + approval_score + escalation_score + rbac_score + quality_score,
        computed_at: now,
    })
}

// 5 private query helpers — each is a single sqlx::query_scalar against
// audit_record (filtered on action_type + actor = worker_id + ts > cutoff)
// or trust_observation. See trust_streak.rs for the audit-record query shape.
async fn longest_streak(...) -> ArgoResult<i64> { ... }
async fn approval_counts(...) -> ArgoResult<(i64, i64)> { ... }
async fn escalation_count(...) -> ArgoResult<i64> { ... }
async fn rbac_deny_count(...) -> ArgoResult<i64> { ... }
async fn cross_tenant_quality_avg(...) -> ArgoResult<f32> { ... }
```

### 1e. Ratchet logic

**File**: `crates/argo-trust-engine/src/ratchet.rs`

```rust
use argo_skill::TrustGrade;
use serde::{Deserialize, Serialize};

const PROMOTE_THRESHOLDS: [(TrustGrade, f32); 4] = [
    (TrustGrade::PublishedCommunity, 2.0),
    (TrustGrade::Tenant, 5.0),
    (TrustGrade::PublishedVerified, 10.0),
    (TrustGrade::Builtin, 20.0),
];

const DEMOTE_THRESHOLDS: [(TrustGrade, f32); 4] = [
    (TrustGrade::PublishedCommunity, 1.0),
    (TrustGrade::Tenant, 3.5),
    (TrustGrade::PublishedVerified, 8.0),
    (TrustGrade::Builtin, 17.0),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RatchetDecision {
    /// Score is within tolerance for the current grade — no change.
    Hold,
    /// Move up one rung.
    Promote { from: TrustGrade, to: TrustGrade },
    /// Move down one rung.
    Demote { from: TrustGrade, to: TrustGrade },
}

/// Given the current grade and the freshly-computed score, return the
/// single-step ratchet decision. Always at most one rung per call.
pub fn ratchet_grade(current: TrustGrade, score: f32) -> RatchetDecision {
    let current_u8 = grade_to_u8(current);
    // Try promote first — if score clears the next rung's promote threshold.
    if let Some(next) = u8_to_grade(current_u8.saturating_add(1)) {
        if let Some((_, threshold)) = PROMOTE_THRESHOLDS.iter()
            .find(|(g, _)| *g == next)
        {
            if score >= *threshold {
                return RatchetDecision::Promote { from: current, to: next };
            }
        }
    }
    // Else demote if score is below current grade's demote threshold.
    if let Some((_, demote_threshold)) = DEMOTE_THRESHOLDS.iter()
        .find(|(g, _)| *g == current)
    {
        if score < *demote_threshold {
            if let Some(prev) = u8_to_grade(current_u8.saturating_sub(1)) {
                return RatchetDecision::Demote { from: current, to: prev };
            }
        }
    }
    RatchetDecision::Hold
}

// u8 ↔ TrustGrade helpers — TrustGrade enum has Builtin=0..LearnedUnreviewed=4,
// but worker.trust_grade u8 is INVERTED (0=lowest, 4=highest). Use the same
// inverted u8 the gateway already uses (see routes_worker_trust_grade.rs).
fn grade_to_u8(g: TrustGrade) -> u8 { ... }
fn u8_to_grade(u: u8) -> Option<TrustGrade> { ... }
```

### 1f. Recompute driver

**File**: `crates/argo-trust-engine/src/recompute.rs`

```rust
//! Per-worker recompute + tick-all-workers driver.

use crate::{ratchet, scoring};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::IdentityStore;
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::json;
use std::sync::Arc;

/// Recompute one worker's grade. Returns the ratchet decision applied (or
/// `Hold` if no change). Caller is responsible for the AuditSink.
pub async fn recompute_worker(
    store: &SessionStore,
    sink: &dyn AuditSink,
    tenant: &TenantId,
    worker_id: &str,
    current_grade: TrustGrade,
) -> ArgoResult<ratchet::RatchetDecision> {
    let breakdown = scoring::compute_score(store, tenant, worker_id).await?;
    let decision = ratchet::ratchet_grade(current_grade, breakdown.total);
    match &decision {
        ratchet::RatchetDecision::Hold => Ok(decision),
        ratchet::RatchetDecision::Promote { from, to }
        | ratchet::RatchetDecision::Demote { from, to } => {
            // Write the new grade.
            let new_u8 = grade_to_u8(*to);
            let identities = IdentityStore::new(store);
            identities.set_worker_trust_grade(
                tenant.clone(),
                worker_id.parse().map_err(|e| ArgoError::Provider(format!("worker id: {e}")))?,
                new_u8,
            ).await?;
            // Emit audit row.
            let payload = json!({
                "worker_id": worker_id,
                "tenant_id": tenant.to_string(),
                "from_grade": format!("{from:?}"),
                "to_grade": format!("{to:?}"),
                "score_total": breakdown.total,
                "score_breakdown": breakdown,
                "decision": match &decision {
                    ratchet::RatchetDecision::Promote { .. } => "promote",
                    ratchet::RatchetDecision::Demote { .. } => "demote",
                    _ => "hold",  // unreachable
                },
            });
            sink.emit(AuditKind::TrustGradeChanged, payload).await
                .map_err(|e| ArgoError::Provider(format!("emit TrustGradeChanged: {e}")))?;
            Ok(decision)
        }
    }
}

/// Walk all (tenant, worker) pairs in `worker` table; recompute each.
/// Returns count of workers that actually changed grade.
pub(crate) async fn tick_all_workers(
    store: Arc<SessionStore>,
    cfg: &crate::TrustEngineConfig,
) -> ArgoResult<usize> {
    // SELECT tenant_id, id, trust_grade FROM worker; iterate; for each row
    // call recompute_worker(); tally changed.
}

pub(crate) async fn tick_all_workers_dry_run(
    store: Arc<SessionStore>,
    cfg: &crate::TrustEngineConfig,
) -> ArgoResult<usize> {
    // As above but skip the write + audit emit; tracing::info each decision.
}
```

---

## Phase 2 — argo-audit: add `TrustGradeChanged` variant

**File**: `crates/argo-audit/src/event.rs`

Same 4-site pattern as the rollback brief (enum body, `label()`, `from_label()`,
`ALL` array):

```rust
/// A worker's trust grade was advanced or rolled back by the periodic
/// `argo-trust-engine` (`web-worker-trust-promotion-engine`). Pairs with
/// `AuditKind::ApprovalGranted` / `EscalationRequired` as the input
/// signals — the trust engine reads those rows, computes a score,
/// and emits this row when the resulting ratchet decision moves a
/// rung. Payload: `{ worker_id, tenant_id, from_grade, to_grade,
/// score_total, score_breakdown: {streak, approval, escalation, rbac,
/// quality}, decision: "promote" | "demote" }`.
TrustGradeChanged,
```

Label string: `"trust_grade_changed"`.

---

## Phase 3 — argo-cli: spawn the engine on boot

**File**: `crates/argo-cli/src/handlers/gateway/boot_serve.rs`

Find the `spawn_forge_cron(...)` call. Add immediately below it:

```rust
let trust_cfg = argo_trust_engine::TrustEngineConfig::from_env();
let _trust_handle = argo_trust_engine::spawn_trust_engine_cron(
    Arc::clone(&store),
    trust_cfg,
);
```

Add `argo-trust-engine.workspace = true` to `crates/argo-cli/Cargo.toml`.

---

## Phase 4 — Gateway: on-demand recompute endpoint

For demo + future operator override, add a single endpoint:

```
POST /api/v1/workers/<id>/trust-grade/recompute
```

Body: empty. Calls `recompute::recompute_worker(...)` synchronously and
returns `{ decision, score_total, new_grade }`. Owner/admin role only via the
RBAC plumbing from `web-rbac-jwt-role-claim-and-gateway-enforce`.

**File**: `crates/argo-cli/src/handlers/gateway/routes_worker_trust_grade.rs`
(extend the existing file — already ~150 LOC, has room).

Add `recompute_response()` alongside the existing `show_response()`. Dispatch
arm goes into `routes_dispatch_post` (sibling file).

---

## Phase 5 — Web: surface the timestamps + remove the aspirational disclaimer

**File**: `web/app/(app)/workers/[workerId]/trust/page.tsx`

Two edits:

1. Replace the disclaimer:

   ```
   - Trust grade is derived from observed behavior, not operator-tunable.
   - The criteria above describe what would unlock the next rung once
   - the worker meets them.
   + Trust grade is recomputed every 30 minutes from observed approvals,
   + escalations, RBAC denials, and cross-tenant quality. Last
   + recomputed: {trustQuery.data.last_computed_at} ·
   + Last grade change: {trustQuery.data.last_grade_change_at ?? "—"}
   ```

2. Add a "Recompute now" button (admin/owner only — read `useViewerRole()` from
   `web/lib/hooks/use-viewer.ts`):

   ```tsx
   {viewerRole.canEdit && (
     <Button onClick={() => recomputeNow.mutate()}>
       Recompute trust grade
     </Button>
   )}
   ```

The `TrustGradeCard` component does NOT change — it already renders the
grade from the snapshot, so the next poll naturally picks up changes.

**File**: `web/lib/api/worker-trust.ts` (extend)

Add `recomputeTrustGrade(workerId)` calling the new POST endpoint.

---

## Phase 6 — Tests

### 6a. Unit tests — scoring (in `crates/argo-trust-engine/tests/scoring_tests.rs`)

1. `compute_score_empty_signals_returns_zero`
2. `compute_score_three_approvals_gives_positive`
3. `compute_score_one_rejection_overwhelms_streak`
4. `compute_score_cross_tenant_quality_weights_into_total`
5. `compute_score_filters_old_events_outside_window`

### 6b. Unit tests — ratchet (in `crates/argo-trust-engine/tests/ratchet_tests.rs`)

1. `ratchet_hold_at_default_grade`
2. `ratchet_promote_one_rung`
3. `ratchet_demote_on_low_score`
4. `ratchet_hysteresis_prevents_flap` (score oscillates 1.9 ↔ 2.1 → no demote
   back from grade 1 once promoted to 1, only demote at < 1.0)
5. `ratchet_promotes_at_most_one_rung_per_call` (score = 50 from grade 0 →
   one promote to grade 1, not all the way to grade 4)

### 6c. Integration test — cross-tenant negative

**File**: `crates/argo-trust-engine/tests/cross_tenant_isolation.rs`

Verifies: tenant A worker's score is not affected by tenant B's audit events.
Create approvals under both tenants; call `recompute_worker` for tenant A's
worker; assert score reflects only tenant A's rows.

### 6d. Flap-resistance integration test

**File**: `crates/argo-trust-engine/tests/flap_resistance_tests.rs`

Drive 100 ticks against a fixture signal stream that oscillates near the
threshold; assert the worker's grade changes at most 3 times across the run
(not every tick).

### 6e. E2E smoke (Playwright)

**File**: `web/tests/trust-promotion.spec.ts`

1. Sign in as test tenant owner.
2. Navigate to a worker that has 3+ recent `approval.granted` audit rows
   pre-seeded.
3. Note the current trust grade.
4. Click "Recompute trust grade".
5. Wait for toast / refresh.
6. Assert grade chip displays new grade.
7. Navigate to `/audit` → assert most recent row has
   `kind: trust_grade_changed`.

---

## may-touch

- **New crate**: `crates/argo-trust-engine/` (lib.rs, cron.rs, scoring.rs, ratchet.rs, recompute.rs + 4 tests files).
- `Cargo.toml` (workspace root) — add `argo-trust-engine` to members alpha-sorted.
- `crates/argo-cli/Cargo.toml` — add `argo-trust-engine.workspace = true`.
- `crates/argo-audit/src/event.rs` — add `TrustGradeChanged` variant (4 sites).
- `crates/argo-cli/src/handlers/gateway/boot_serve.rs` — spawn the cron.
- `crates/argo-cli/src/handlers/gateway/routes_worker_trust_grade.rs` — add `recompute_response`.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` (or equivalent) — wire POST arm.
- `web/app/(app)/workers/[workerId]/trust/page.tsx` — disclaimer rewrite + recompute button.
- `web/lib/api/worker-trust.ts` — add `recomputeTrustGrade` API client.
- `web/tests/trust-promotion.spec.ts` (new) — E2E.

## must-NOT-touch

- `argo_skill::trust::TrustGrade` enum — variants stay as-is.
- `worker.trust_grade` column type / migration — already exists.
- `IdentityStore::set_worker_trust_grade` — already correct.
- The 5 signal-source tables (`audit_record`, `trust_observation`,
  `trust_streak`) — engine is read-only against these.
- `TrustGradeCard` web component — renders from the snapshot, no change.
- Web `useWorkerTrustGrade` hook — already polls the gateway.

---

## Acceptance criteria

- [ ] `argo-trust-engine` crate compiles, all 5 modules under 300 LOC each.
- [ ] `cargo test -p argo-trust-engine` passes (12 unit tests + 2 integration tests).
- [ ] Cron loop starts on `argo gateway serve` boot (visible in tracing logs).
- [ ] `tick_all_workers` walks every `(tenant, worker)` row in the DB.
- [ ] `recompute_worker` writes `worker.trust_grade` via existing `IdentityStore` method (no parallel write path).
- [ ] `AuditKind::TrustGradeChanged` row emitted for every grade movement.
- [ ] Cross-tenant negative test passes: tenant A worker's score is not polluted by tenant B's events.
- [ ] `ratchet_grade` never promotes more than one rung per call.
- [ ] Flap-resistance test: 100 oscillating ticks → ≤3 grade changes.
- [ ] `POST /api/v1/workers/<id>/trust-grade/recompute` returns `{ decision, new_grade, score_total }`.
- [ ] Endpoint is RBAC-gated (admin/owner only — depends on `web-rbac-jwt-role-claim-and-gateway-enforce`).
- [ ] Web /trust page shows "Recomputed every 30 minutes" + last-change timestamps.
- [ ] Web "Recompute trust grade" button is visible for admin/owner, hidden for user/guest.
- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean in `web/`.
- [ ] Demo flow: hire a worker, run 3 `approve --grant` sessions, wait one cron tick (or click Recompute) → /audit shows `trust_grade_changed` row + /trust page shows the new grade.

---

## Gotchas

1. **u8 inversion**: `argo_skill::TrustGrade` has `Builtin=0` (top trust) at
   variant index 0 but `worker.trust_grade` u8 in storage uses
   `0 = LearnedUnreviewed` (bottom). Use the same conversion the existing
   gateway file `routes_worker_trust_grade.rs` uses — copy `grade_to_u8` /
   `u8_to_grade` from there into `ratchet.rs` as private helpers; do not
   introduce a new mapping.
2. **The default grade for a fresh hire is u8=0 = LearnedUnreviewed**, not
   the lowest TrustGrade enum variant. The ratchet should treat a brand-new
   worker (zero signals, score = 0.0) as `Hold` at LearnedUnreviewed, NOT
   demote-below (there's nothing below).
3. **The signal queries must filter by tenant_id AND by the worker_id as the
   actor**, not by tenant alone. Cross-tenant isolation depends on both
   filters. The cross-tenant test in 6c is non-optional.
4. **`audit_record.action_type` is a label string, not the enum**: when
   filtering for `ApprovalGranted`, the WHERE clause should be
   `action_type = 'approval.granted'` (the label), not the variant index.
5. **Don't write to the audit chain inside the per-row tick if `cfg.dry_run`
   is true**. The dry-run path is for operator inspection — it must not have
   side effects.
6. **Engine boot order**: spawn the cron AFTER `SessionStore::migrate` has
   completed; otherwise the first tick fails with "no such table". The
   forge_cron spawn site is already correctly placed; copy that ordering.
7. **CronJob.tenant_id**: this engine reads ACROSS tenants in one tick (it's
   admin-level, not tenant-scoped). Tenant-isolation lives inside
   `scoring::compute_score` which filters per worker's tenant. Do not try to
   add a tenant_id field to `TrustEngineConfig` — the engine is a singleton.
8. **CI flake risk**: `tokio::time::interval` first tick fires immediately,
   so an integration test that spawns the loop and reads grades after 1ms
   can race. Use the explicit `tick()` function (not the spawn) in tests.

---

## References

- Audit: `docs/skycot/audit/user-journey/2026-05-20-wave-15-final-verdict.md` (the static-observatory finding)
- Substrate: `crates/argo-store/src/trust_observation.rs`, `trust_streak.rs`
- Write API: `crates/argo-identity/src/store_trust_grade.rs`
- Cron pattern: `crates/argo-forge/src/cron.rs` (copy-modify template)
- Audit kind pattern: `crates/argo-audit/src/event.rs` — search `WorkflowDefinitionCreated`
- Existing read endpoint: `crates/argo-cli/src/handlers/gateway/routes_worker_trust_grade.rs`
- Existing UI: `web/app/(app)/workers/[workerId]/trust/page.tsx` + `web/components/skycot/worker/TrustGradeCard.tsx`
