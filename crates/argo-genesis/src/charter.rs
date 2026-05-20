//! Structured [`WorkerCharter`] — the typed output of JD-to-charter
//! synthesis (`mission-audit-genesis-jd-llm-summarizer`).
//!
//! Before this brief, [`crate::job_description::genesis_from_job_description`]
//! passed the raw JD text straight through to
//! [`crate::pipeline::plan_with`]; clean test JDs worked, real
//! customer JDs (messy PDF extracts, multi-role boilerplate) didn't.
//! The new flow runs the JD through a
//! [`crate::synthesizer::Synthesizer`] and stores the structured
//! charter on [`crate::GenesisDraft`] so downstream consumers keep
//! reading [`Self::mission`] as a `String` while picking up
//! responsibilities / tool hints / escalation contact / vertical
//! hint as opt-in fields.
//!
//! `source_jd_hash` is SHA-256 of the (pre-processed) raw JD. Not
//! used yet — a future cache brief keys on the hash to dedupe
//! identical JDs without re-prompting.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Validation bounds on a [`WorkerCharter`]. The LLM-driven
/// [`crate::synthesizer::LlmSynthesizer`] enforces these on every
/// parsed response; the stub path skips validation (identity
/// charter is by definition unstructured).
pub const MIN_RESPONSIBILITIES: usize = 1;
/// Upper bound on the responsibility count. Above this, the model
/// is usually pasting individual bullet points instead of the
/// 3-5 high-level statements we want.
pub const MAX_RESPONSIBILITIES: usize = 7;
/// Upper bound on the `tools_hint` count. We only want the top few.
pub const MAX_TOOLS_HINT: usize = 5;

/// Field-level validation error surfaced by [`WorkerCharter::validate`].
/// Each variant identifies the offending field so the caller can
/// produce a clear `ArgoError::Schema` message.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CharterValidationError {
    /// `mission` field was empty / whitespace-only.
    #[error("worker charter mission is empty")]
    EmptyMission,
    /// `responsibilities` had fewer than [`MIN_RESPONSIBILITIES`]
    /// or more than [`MAX_RESPONSIBILITIES`] entries.
    #[error(
        "worker charter responsibilities count {count} is outside [{min}, {max}]",
        min = MIN_RESPONSIBILITIES,
        max = MAX_RESPONSIBILITIES,
    )]
    ResponsibilitiesOutOfRange {
        /// Observed count.
        count: usize,
    },
    /// `tools_hint` had more than [`MAX_TOOLS_HINT`] entries.
    #[error(
        "worker charter tools_hint count {count} exceeds max {max}",
        max = MAX_TOOLS_HINT,
    )]
    TooManyTools {
        /// Observed count.
        count: usize,
    },
}

/// Structured charter produced by a [`crate::synthesizer::Synthesizer`].
///
/// `mission` is a 1-paragraph statement of what the worker exists
/// to do. The other fields are best-effort hints. `source_jd_hash`
/// is SHA-256 of the JD text the charter was synthesized from
/// (lowercased hex) — used in a future cache brief, emitted here
/// so the field is stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerCharter {
    /// One-paragraph mission statement.
    pub mission: String,
    /// 1-7 high-level responsibilities.
    #[serde(default)]
    pub responsibilities: Vec<String>,
    /// Up to 5 tool / system hints (e.g., `"Salesforce"`, `"Slack"`).
    #[serde(default)]
    pub tools_hint: Vec<String>,
    /// Optional named escalation contact (role name or email).
    #[serde(default)]
    pub escalation_contact: Option<String>,
    /// Optional vertical / archetype hint slug.
    #[serde(default)]
    pub vertical_hint: Option<String>,
    /// SHA-256 (lowercase hex) of the raw JD text this charter was
    /// synthesized from. Stable across calls — same JD ⇒ same hash.
    pub source_jd_hash: String,
}

impl WorkerCharter {
    /// SHA-256 hex of `raw` — the value stamped onto
    /// [`Self::source_jd_hash`]. Public so callers / future cache
    /// briefs can compute the same hash without re-running synthesis.
    #[must_use]
    pub fn sha256_jd(raw: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Identity charter — `mission` is the raw JD, all other fields
    /// empty. Used by [`crate::synthesizer::StubSynthesizer`] and as
    /// the fail-soft fallback when the LLM path is wrapped via
    /// [`crate::synthesizer::synthesize_or_stub`].
    #[must_use]
    pub fn from_identity(raw: &str) -> Self {
        Self {
            mission: raw.to_string(),
            responsibilities: Vec::new(),
            tools_hint: Vec::new(),
            escalation_contact: None,
            vertical_hint: None,
            source_jd_hash: Self::sha256_jd(raw),
        }
    }

    /// Enforce field-level bounds. The stub path skips this (its
    /// charter is unstructured by design); the LLM path calls it on
    /// every parsed response so a malformed model output surfaces as
    /// a [`CharterValidationError`] instead of silently propagating.
    ///
    /// # Errors
    /// Returns the first failing rule encountered (mission first,
    /// then responsibility count, then tool count).
    pub fn validate(&self) -> Result<(), CharterValidationError> {
        if self.mission.trim().is_empty() {
            return Err(CharterValidationError::EmptyMission);
        }
        let count = self.responsibilities.len();
        if !(MIN_RESPONSIBILITIES..=MAX_RESPONSIBILITIES).contains(&count) {
            return Err(CharterValidationError::ResponsibilitiesOutOfRange { count });
        }
        let tcount = self.tools_hint.len();
        if tcount > MAX_TOOLS_HINT {
            return Err(CharterValidationError::TooManyTools { count: tcount });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_charter_round_trips_raw_jd() {
        let charter = WorkerCharter::from_identity("hello");
        assert_eq!(charter.mission, "hello");
        assert!(charter.responsibilities.is_empty());
        assert_eq!(charter.source_jd_hash.len(), 64); // SHA-256 hex
    }

    #[test]
    fn sha256_jd_is_stable() {
        let a = WorkerCharter::sha256_jd("be my SDR");
        let b = WorkerCharter::sha256_jd("be my SDR");
        assert_eq!(a, b);
        let c = WorkerCharter::sha256_jd("be my SDR.");
        assert_ne!(a, c);
    }

    #[test]
    fn empty_mission_rejects() {
        let mut c = WorkerCharter::from_identity("non-empty");
        c.mission = "   ".into();
        c.responsibilities = vec!["x".into()];
        assert_eq!(c.validate(), Err(CharterValidationError::EmptyMission));
    }

    #[test]
    fn too_many_responsibilities_rejects() {
        let mut c = WorkerCharter::from_identity("x");
        c.responsibilities = (0..MAX_RESPONSIBILITIES + 1).map(|i| i.to_string()).collect();
        assert_eq!(
            c.validate(),
            Err(CharterValidationError::ResponsibilitiesOutOfRange {
                count: MAX_RESPONSIBILITIES + 1
            })
        );
    }

    #[test]
    fn zero_responsibilities_rejects() {
        let mut c = WorkerCharter::from_identity("x");
        c.responsibilities = vec![];
        assert_eq!(
            c.validate(),
            Err(CharterValidationError::ResponsibilitiesOutOfRange { count: 0 })
        );
    }

    #[test]
    fn too_many_tools_rejects() {
        let mut c = WorkerCharter::from_identity("x");
        c.responsibilities = vec!["r".into()];
        c.tools_hint = (0..MAX_TOOLS_HINT + 1).map(|i| i.to_string()).collect();
        assert_eq!(
            c.validate(),
            Err(CharterValidationError::TooManyTools {
                count: MAX_TOOLS_HINT + 1
            })
        );
    }

    #[test]
    fn valid_charter_validates() {
        let mut c = WorkerCharter::from_identity("non-empty");
        c.responsibilities = vec!["r1".into(), "r2".into()];
        c.tools_hint = vec!["Slack".into()];
        c.escalation_contact = Some("ops@example.com".into());
        c.vertical_hint = Some("sales_sdr".into());
        assert!(c.validate().is_ok());
    }
}
