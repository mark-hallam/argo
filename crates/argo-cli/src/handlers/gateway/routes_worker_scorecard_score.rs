//! Pure scoring + audit-actual resolution for the worker goal-vs-actual
//! scorecard ([`super::routes_worker_scorecard`]).
//!
//! The scorecard joins a vertical's declarative KPI **targets**
//! (`VerticalManifest::all_kpis`) with **actuals** measured from the
//! tenant-scoped audit trail. Two source kinds are honestly
//! measurable today:
//!
//! - [`ScoreSource::AuditCount`] — count of audit rows of a fixed
//!   `kind` over the window (e.g. `assistant_message` for a
//!   "messages handled" count target).
//! - [`ScoreSource::ResponseLatencyMs`] — median of the per-row
//!   `latency_ms` payload field for any `DurationMs` latency target.
//!
//! Everything else resolves to [`ScoreSource::Unmeasured`]. **NO
//! THEATER**: an unmeasured target renders "target set — measurement
//! not wired", never a fabricated actual. `score` only ever runs on a
//! real observed value.

use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use argo_vertical::{KpiComparator, KpiDefinition, KpiKind};
use sqlx::Row;

/// Where the scorecard measured a KPI's actual value FROM, or that no
/// countable audit source is wired for it yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScoreSource {
    /// Count of audit rows whose `kind` equals this string.
    AuditCount(&'static str),
    /// Median of the `latency_ms` payload field across audit rows
    /// that carry it (milliseconds).
    ResponseLatencyMs,
    /// No countable audit source wired yet — renders the honest
    /// "measurement not wired" note, never a number.
    Unmeasured,
}

impl ScoreSource {
    /// Stable identifier surfaced in the JSON so the SPA (and an
    /// auditor) can see exactly what was measured.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ScoreSource::AuditCount(kind) => kind,
            ScoreSource::ResponseLatencyMs => "response_latency_ms",
            ScoreSource::Unmeasured => "unmeasured",
        }
    }
}

/// Classify a KPI definition into the audit source the scorecard can
/// honestly measure it from. Matching is deterministic — by `kind`
/// then by keyword in the lower-cased `name`. Anything unrecognised
/// is [`ScoreSource::Unmeasured`] on purpose; we never guess a source.
pub(crate) fn classify_source(kpi: &KpiDefinition) -> ScoreSource {
    if kpi.kind == KpiKind::DurationMs {
        // Every duration target is a response-latency target measured
        // from the `latency_ms` payload field.
        return ScoreSource::ResponseLatencyMs;
    }
    if kpi.kind == KpiKind::Count {
        let n = kpi.name.to_ascii_lowercase();
        if n.contains("message") || n.contains("reply") || n.contains("response") {
            return ScoreSource::AuditCount("assistant_message");
        }
        if n.contains("escalat") {
            return ScoreSource::AuditCount("escalation.required");
        }
        if n.contains("tool") || n.contains("action") || n.contains("task") {
            return ScoreSource::AuditCount("tool_call");
        }
    }
    ScoreSource::Unmeasured
}

/// Epsilon used for an `Eq` comparator. Coarse enough that a boolean
/// expressed as `1.0`/`0.0` (the `required_audit_chain_intact`
/// legacy mapping) and small integer counts compare cleanly.
const EQ_EPSILON: f64 = 0.5;

/// Score a measured `actual` against a `threshold` under `comparator`.
///
/// Returns `(attainment_pct, met)` where `attainment_pct` is rounded
/// to one decimal. Attainment is intentionally **uncapped** above
/// 100 — beating a target reads as "133%" (the acceptance scenario
/// "respond < 60s" observed at 45s renders `met / 133.3%`).
pub(crate) fn score(actual: f64, threshold: f64, comparator: KpiComparator) -> (f64, bool) {
    let pct = match comparator {
        KpiComparator::Gte => {
            if threshold > 0.0 {
                actual / threshold * 100.0
            } else if actual > 0.0 {
                100.0
            } else {
                0.0
            }
        }
        KpiComparator::Lte => {
            if actual > 0.0 {
                threshold / actual * 100.0
            } else {
                // Zero-latency / zero-cost is as good as it gets.
                100.0
            }
        }
        KpiComparator::Eq => {
            if (actual - threshold).abs() < EQ_EPSILON {
                100.0
            } else {
                0.0
            }
        }
    };
    let met = match comparator {
        KpiComparator::Gte => actual >= threshold,
        KpiComparator::Lte => actual <= threshold,
        KpiComparator::Eq => (actual - threshold).abs() < EQ_EPSILON,
    };
    ((pct * 10.0).round() / 10.0, met)
}

/// `"met"` when the target is hit, `"at_risk"` otherwise.
pub(crate) fn status_label(met: bool) -> &'static str {
    if met {
        "met"
    } else {
        "at_risk"
    }
}

/// Human-readable rendering of a numeric value under a KPI kind.
/// Mirrors `routes_worker_detail::format_kpi`'s unit suffixes so the
/// scorecard and the worker-detail KPI chip agree.
pub(crate) fn format_value(kind: KpiKind, v: f64) -> String {
    match kind {
        KpiKind::Percentage => format!("{}%", trim(v)),
        KpiKind::DurationMs => format!("{}ms", trim(v)),
        KpiKind::Count => format!("{}", v.round() as i64),
        KpiKind::Ratio => format!("{v:.2}"),
        KpiKind::Score => trim(v),
    }
}

/// Comparator rendered as the operator-facing inequality symbol.
pub(crate) fn comparator_symbol(c: KpiComparator) -> &'static str {
    match c {
        KpiComparator::Gte => "≥",
        KpiComparator::Lte => "≤",
        KpiComparator::Eq => "=",
    }
}

/// Drop a trailing `.0` so `60.0` renders as `60`, but keep genuine
/// fractional values intact.
fn trim(v: f64) -> String {
    if (v.fract()).abs() < f64::EPSILON {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Resolve the observed actual for a KPI from the tenant-scoped audit
/// trail since `since_micros`. Returns `None` when the source is
/// `Unmeasured` OR when a latency target has no rows carrying a
/// `latency_ms` field — both render "measurement not wired".
///
/// An `AuditCount` source returns `Some(0.0)` when there are simply no
/// matching rows: zero actions is a real, measured zero, not a gap.
pub(crate) async fn resolve_actual(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: argo_identity::WorkerId,
    source: ScoreSource,
    since_micros: i64,
) -> ArgoResult<Option<f64>> {
    match source {
        ScoreSource::Unmeasured => Ok(None),
        ScoreSource::AuditCount(kind) => {
            let count = audit_count(store, tenant, worker_id, kind, since_micros).await?;
            Ok(Some(count as f64))
        }
        ScoreSource::ResponseLatencyMs => {
            let samples = latency_samples(store, tenant, worker_id, since_micros).await?;
            Ok(median(&samples))
        }
    }
}

async fn audit_count(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: argo_identity::WorkerId,
    kind: &str,
    since_micros: i64,
) -> ArgoResult<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS n \
         FROM audit_record a JOIN session s ON s.id = a.session_id \
         WHERE s.tenant_id = ? AND s.worker_id = ? AND a.kind = ? AND a.created_at >= ?",
    )
    .bind(tenant.0.as_bytes().to_vec())
    .bind(worker_id.0.as_bytes().to_vec())
    .bind(kind)
    .bind(since_micros)
    .fetch_one(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("scorecard count: {e}")))?;
    Ok(row.try_get("n").unwrap_or(0))
}

async fn latency_samples(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: argo_identity::WorkerId,
    since_micros: i64,
) -> ArgoResult<Vec<f64>> {
    // CAST to REAL so an integer `latency_ms` (e.g. `45000`) decodes as
    // `f64`; without the cast SQLite returns integer affinity and the
    // `f64` decode silently drops every sample.
    let rows = sqlx::query(
        "SELECT CAST(json_extract(a.payload_json, '$.latency_ms') AS REAL) AS lat \
         FROM audit_record a JOIN session s ON s.id = a.session_id \
         WHERE s.tenant_id = ? AND s.worker_id = ? AND a.created_at >= ? \
           AND json_extract(a.payload_json, '$.latency_ms') IS NOT NULL",
    )
    .bind(tenant.0.as_bytes().to_vec())
    .bind(worker_id.0.as_bytes().to_vec())
    .bind(since_micros)
    .fetch_all(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("scorecard latency: {e}")))?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<f64, _>("lat").ok())
        .collect())
}

/// Median of a sample set, or `None` when empty (so a latency target
/// with no observations renders "measurement not wired").
pub(crate) fn median(samples: &[f64]) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        Some(sorted[mid])
    } else {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    }
}
