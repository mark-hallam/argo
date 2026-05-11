//! Replicate DTOs.
//!
//! Public types ([`Prediction`], [`ModelInfo`], [`AccountInfo`]) are the
//! caller-facing shape. Wire structs mirror Replicate's JSON envelopes and
//! convert at the serde boundary via [`From`] impls so callers never see
//! wire oddities (e.g. snake-case status strings, optional nested objects).

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Opaque Replicate prediction id.
///
/// Returned by [`crate::ReplicateConnector::start_prediction`] and
/// embedded in every [`Prediction`]. Treated as a black-box string —
/// the connector does not parse or interpret it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PredictionId(pub String);

impl PredictionId {
    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for PredictionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for PredictionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Replicate prediction lifecycle states.
///
/// `Starting` and `Processing` are the two non-terminal states; the
/// polling loop continues while either is observed. The three terminal
/// states (`Succeeded`, `Failed`, `Canceled`) exit the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionStatus {
    /// The prediction has been accepted and is being scheduled.
    Starting,
    /// The model is actively running.
    Processing,
    /// Terminal: completed successfully; `output` populated.
    Succeeded,
    /// Terminal: the model raised an error; `error` populated.
    Failed,
    /// Terminal: the operator (or a cancel-prediction call) stopped it.
    Canceled,
}

impl PredictionStatus {
    /// True if the status is one of the three terminal states.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Canceled)
    }
}

impl std::str::FromStr for PredictionStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "starting" => Ok(Self::Starting),
            "processing" => Ok(Self::Processing),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "canceled" | "cancelled" => Ok(Self::Canceled),
            other => Err(format!("unknown prediction status: {other}")),
        }
    }
}

/// One Replicate prediction.
///
/// Status is one of [`PredictionStatus`]'s five variants. `output` shape
/// is model-defined — see crate-level docs for why it is intentionally
/// [`serde_json::Value`].
#[derive(Debug, Clone, PartialEq)]
pub struct Prediction {
    /// Opaque Replicate id.
    pub id: PredictionId,
    /// Model identifier — either `"owner/name"` or a version id.
    pub model: String,
    /// Lifecycle state.
    pub status: PredictionStatus,
    /// Populated on `Succeeded`. Shape is model-defined.
    pub output: Option<serde_json::Value>,
    /// Populated on `Failed`. Free-form upstream message.
    pub error: Option<String>,
    /// Per-call timing / token counts; surface for spend attribution.
    pub metrics: Option<PredictionMetrics>,
    /// RFC3339 timestamp; `None` until the prediction starts running.
    pub started_at: Option<DateTime<Utc>>,
    /// RFC3339 timestamp; populated on any terminal state.
    pub completed_at: Option<DateTime<Utc>>,
}

/// Per-prediction billing-relevant metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct PredictionMetrics {
    /// Seconds spent inside the model invocation (the billing duration).
    pub predict_time: Option<f64>,
    /// End-to-end wall-clock duration, including queueing.
    pub total_time: Option<f64>,
    /// LLM-specific: prompt tokens.
    pub input_token_count: Option<u64>,
    /// LLM-specific: completion tokens.
    pub output_token_count: Option<u64>,
}

/// Replicate model registry record.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    /// User or org that publishes the model.
    pub owner: String,
    /// Model name within the owner namespace.
    pub name: String,
    /// Human-readable description (markdown).
    pub description: String,
    /// Example input the model registry suggests; shape is model-defined.
    pub default_example: Option<serde_json::Value>,
    /// Opaque latest-version id, e.g. for `"version": ...` create-bodies.
    pub latest_version_id: String,
    /// Lifetime run count published by Replicate (popularity hint).
    pub run_count: u64,
    /// Cover image URL, if any.
    pub cover_image_url: Option<String>,
    /// License URL, if published — verticals can refuse restrictive licenses.
    pub license_url: Option<String>,
}

/// Replicate account info — caller's tenancy + billing identity.
///
/// Returned (and memoized) by [`crate::ReplicateConnector::account`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInfo {
    /// Account kind — Replicate returns this on the `type` field, which is
    /// a Rust reserved keyword so it's exposed here as `type_`.
    pub type_: String,
    /// Account username (URL slug).
    pub username: String,
    /// Display name.
    pub name: String,
}

// -- wire shapes -------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct PredictionWire {
    pub id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub status: String,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
    #[serde(default)]
    pub metrics: Option<MetricsWire>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MetricsWire {
    #[serde(default)]
    pub predict_time: Option<f64>,
    #[serde(default)]
    pub total_time: Option<f64>,
    #[serde(default)]
    pub input_token_count: Option<u64>,
    #[serde(default)]
    pub output_token_count: Option<u64>,
}

impl From<MetricsWire> for PredictionMetrics {
    fn from(w: MetricsWire) -> Self {
        Self {
            predict_time: w.predict_time,
            total_time: w.total_time,
            input_token_count: w.input_token_count,
            output_token_count: w.output_token_count,
        }
    }
}

impl PredictionWire {
    pub(crate) fn into_prediction(self) -> Result<Prediction, String> {
        let status: PredictionStatus = self.status.parse()?;
        let model = self
            .model
            .or(self.version)
            .unwrap_or_else(|| "unknown".to_string());
        let error = self.error.and_then(|v| match v {
            serde_json::Value::String(s) => Some(s),
            serde_json::Value::Null => None,
            other => Some(other.to_string()),
        });
        Ok(Prediction {
            id: PredictionId(self.id),
            model,
            status,
            output: self.output,
            error,
            metrics: self.metrics.map(Into::into),
            started_at: self.started_at,
            completed_at: self.completed_at,
        })
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelInfoWire {
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_example: Option<serde_json::Value>,
    #[serde(default)]
    pub latest_version: Option<LatestVersionWire>,
    #[serde(default)]
    pub run_count: Option<u64>,
    #[serde(default)]
    pub cover_image_url: Option<String>,
    #[serde(default)]
    pub license_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LatestVersionWire {
    pub id: String,
}

impl From<ModelInfoWire> for ModelInfo {
    fn from(w: ModelInfoWire) -> Self {
        Self {
            owner: w.owner,
            name: w.name,
            description: w.description.unwrap_or_default(),
            default_example: w.default_example,
            latest_version_id: w.latest_version.map(|v| v.id).unwrap_or_default(),
            run_count: w.run_count.unwrap_or(0),
            cover_image_url: w.cover_image_url,
            license_url: w.license_url,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AccountInfoWire {
    #[serde(rename = "type")]
    pub type_: String,
    pub username: String,
    #[serde(default)]
    pub name: Option<String>,
}

impl From<AccountInfoWire> for AccountInfo {
    fn from(w: AccountInfoWire) -> Self {
        Self {
            type_: w.type_,
            username: w.username.clone(),
            name: w.name.unwrap_or(w.username),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn status_parses_all_terminal_variants() {
        assert_eq!(
            PredictionStatus::from_str("starting").unwrap(),
            PredictionStatus::Starting
        );
        assert_eq!(
            PredictionStatus::from_str("processing").unwrap(),
            PredictionStatus::Processing
        );
        assert_eq!(
            PredictionStatus::from_str("succeeded").unwrap(),
            PredictionStatus::Succeeded
        );
        assert_eq!(
            PredictionStatus::from_str("failed").unwrap(),
            PredictionStatus::Failed
        );
        assert_eq!(
            PredictionStatus::from_str("canceled").unwrap(),
            PredictionStatus::Canceled
        );
        // Replicate sometimes spells it British-style.
        assert_eq!(
            PredictionStatus::from_str("cancelled").unwrap(),
            PredictionStatus::Canceled
        );
        assert!(PredictionStatus::from_str("garbage").is_err());
    }

    #[test]
    fn terminal_check_matches_state_machine() {
        assert!(!PredictionStatus::Starting.is_terminal());
        assert!(!PredictionStatus::Processing.is_terminal());
        assert!(PredictionStatus::Succeeded.is_terminal());
        assert!(PredictionStatus::Failed.is_terminal());
        assert!(PredictionStatus::Canceled.is_terminal());
    }

    #[test]
    fn account_info_wire_falls_back_name_to_username() {
        let wire = AccountInfoWire {
            type_: "user".into(),
            username: "ada".into(),
            name: None,
        };
        let info: AccountInfo = wire.into();
        assert_eq!(info.name, "ada");
    }
}
