//! Data types for `worker_okr` — enums, row types, the `--kr` parser,
//! and the per-period progress shapes. Split out of `worker_okr.rs` to
//! keep that module under the 500-LOC ceiling; SQL persistence and the
//! scoring math stay in the parent. Re-exported via `pub use` so
//! `argo_store::worker_okr::Comparison` (etc.) continue to resolve.

use serde::{Deserialize, Serialize};

use argo_types::{ArgoError, ArgoResult};

/// Comparison operator for a key result. The operator-facing tokens are
/// `gte`, `lte`, `eq` to keep the `--kr` CLI parser unambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    /// `actual >= target` is the success criterion.
    Gte,
    /// `actual <= target` is the success criterion.
    Lte,
    /// `actual == target` is the success criterion (within `f64::EPSILON`).
    Eq,
}

impl Comparison {
    /// Parse the operator-facing token used in `--kr <kpi>:<op>:<target>:<weight>`.
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "gte" => Ok(Self::Gte),
            "lte" => Ok(Self::Lte),
            "eq" => Ok(Self::Eq),
            _ => Err(ArgoError::Provider(format!(
                "invalid comparison '{s}': expected gte|lte|eq"
            ))),
        }
    }

    /// String form persisted into `worker_key_result.comparison`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gte => "gte",
            Self::Lte => "lte",
            Self::Eq => "eq",
        }
    }
}

/// Lifecycle of a `worker_objective` row. Persisted as the lowercase
/// string form on the `status` column so future variants land additively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    /// In-flight; the default for `set`.
    Active,
    /// Hit. Set by the operator after a successful period close.
    Achieved,
    /// Closed without success.
    Missed,
    /// Withdrawn — counted out of the period scoreboard entirely.
    Cancelled,
}

impl ObjectiveStatus {
    /// String form persisted into `worker_objective.status`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Achieved => "achieved",
            Self::Missed => "missed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Inverse of [`Self::as_str`] — used when reading rows back from
    /// SQLite. Unknown values fall through to [`Self::Active`] so a
    /// future enum variant doesn't silently break existing callers.
    pub fn parse(s: &str) -> Self {
        match s {
            "achieved" => Self::Achieved,
            "missed" => Self::Missed,
            "cancelled" => Self::Cancelled,
            _ => Self::Active,
        }
    }
}

/// One key-result spec, as parsed from `--kr kpi:op:target:weight`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyResultSpec {
    /// KPI name to read out of `worker_kpi_value`.
    pub kpi_name: String,
    /// Comparison operator.
    pub comparison: Comparison,
    /// Numeric target the comparison is evaluated against.
    pub target_value: f64,
    /// Relative weight in the period score. Must be `> 0`.
    pub weight: f64,
}

impl KeyResultSpec {
    /// Parse the `kpi:op:target:weight` token used by `argo worker okr
    /// set --kr ...`. Mirrors the brief's contract exactly: four
    /// colon-separated fields, weight strictly positive.
    pub fn parse(token: &str) -> ArgoResult<Self> {
        let parts: Vec<&str> = token.split(':').collect();
        if parts.len() != 4 {
            return Err(ArgoError::Provider(format!(
                "invalid --kr '{token}': expected kpi:op:target:weight"
            )));
        }
        let kpi_name = parts[0].trim().to_string();
        if kpi_name.is_empty() {
            return Err(ArgoError::Provider(format!(
                "invalid --kr '{token}': kpi name must not be empty"
            )));
        }
        let comparison = Comparison::parse(parts[1].trim())?;
        let target_value: f64 = parts[2].trim().parse().map_err(|e| {
            ArgoError::Schema(format!("invalid --kr '{token}': target parse: {e}"))
        })?;
        if !target_value.is_finite() {
            return Err(ArgoError::Provider(format!(
                "invalid --kr '{token}': target must be finite"
            )));
        }
        let weight: f64 = parts[3].trim().parse().map_err(|e| {
            ArgoError::Schema(format!("invalid --kr '{token}': weight parse: {e}"))
        })?;
        if !weight.is_finite() || weight <= 0.0 {
            return Err(ArgoError::Provider(format!(
                "invalid --kr '{token}': weight must be > 0"
            )));
        }
        Ok(Self { kpi_name, comparison, target_value, weight })
    }
}

/// One persisted objective row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveRow {
    /// Auto-increment id.
    pub id: i64,
    /// Worker id this objective belongs to.
    pub worker_id: String,
    /// Tenant boundary.
    pub tenant_id: String,
    /// Period token (`YYYYQ[1-4]`).
    pub period: String,
    /// Operator-facing title.
    pub title: String,
    /// Optional long-form description.
    pub description: Option<String>,
    /// Current lifecycle state.
    pub status: ObjectiveStatus,
    /// Micros since epoch when the row was inserted.
    pub created_at: i64,
}

/// One persisted key-result row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyResultRow {
    /// Auto-increment id.
    pub id: i64,
    /// Owning objective.
    pub objective_id: i64,
    /// KPI name read out of `worker_kpi_value`.
    pub kpi_name: String,
    /// Numeric target.
    pub target_value: f64,
    /// Comparison operator.
    pub comparison: Comparison,
    /// Relative weight in the period score (`> 0`).
    pub weight: f64,
}

/// Per-KR slice of a [`ProgressReport`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyResultProgress {
    /// The KR row.
    pub key_result: KeyResultRow,
    /// Most-recent observed value, if any. `None` ⇒ the KPI has never
    /// been recorded for the (worker, tenant) pair.
    pub actual_value: Option<f64>,
    /// `[0.0, 1.0]` achievement ratio per the brief formula. `0.0` when
    /// no observation exists.
    pub achievement: f64,
    /// `true` when the comparison itself passes against `actual_value`.
    pub achieved: bool,
}

/// One objective's full scorecard for a period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressReport {
    /// The owning objective row.
    pub objective: ObjectiveRow,
    /// Per-KR detail in insertion order.
    pub key_results: Vec<KeyResultProgress>,
    /// Weighted score in `[0.0, 1.0]`. `0.0` when the objective has no
    /// key-results.
    pub weighted_score: f64,
}
