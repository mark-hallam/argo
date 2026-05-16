//! External-judge adjudication types (Amendment 2).
use crate::ids::{JudgeRecordId, SessionId};
use crate::routing::ProviderModel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What did the judge evaluate?
///
/// # Examples
///
/// ```
/// use argo_types::JudgeSubject;
/// let subject = JudgeSubject::SkillCreate {
///     name: "review-pr".into(),
///     hash: [0u8; 32],
/// };
/// assert!(serde_json::to_string(&subject).unwrap().contains("skill_create"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JudgeSubject {
    /// A new skill being created.
    SkillCreate {
        /// Name.
        name: String,
        /// Sha256 of the skill body.
        hash: [u8; 32],
    },
    /// An existing skill being refined.
    SkillRefine {
        /// Name.
        name: String,
        /// Version we're refining from.
        version_from: String,
        /// Sha256 of the proposed new body.
        version_to_hash: [u8; 32],
    },
    /// A memory write (optional adjudication).
    MemoryWrite {
        /// Memory entry id.
        entry_id: Uuid,
    },
    /// A delegate-task outcome.
    DelegateOutcome {
        /// Child session.
        delegate_session: SessionId,
    },
}

/// The judge's verdict.
///
/// # Examples
///
/// ```
/// use argo_types::JudgeVerdict;
/// let verdict = JudgeVerdict::Failure { reason: "no test coverage".into() };
/// let json = serde_json::to_string(&verdict).unwrap();
/// assert!(json.contains("failure"));
/// assert!(json.contains("no test coverage"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum JudgeVerdict {
    /// Accept.
    Success,
    /// Accept as draft only.
    Partial {
        /// Why partial.
        reason: String,
    },
    /// Reject.
    Failure {
        /// Why failed.
        reason: String,
    },
}

/// A persisted judge record.
///
/// # Examples
///
/// ```
/// use argo_types::{
///     JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict,
///     ProviderModel, SessionId,
/// };
/// let record = JudgeRecord {
///     id: JudgeRecordId::new(),
///     session_id: SessionId::new(),
///     subject: JudgeSubject::MemoryWrite { entry_id: uuid::Uuid::now_v7() },
///     verdict: JudgeVerdict::Success,
///     judge_model: ProviderModel { provider: "anthropic".into(), model: "claude-haiku".into() },
///     rubric_id: "memory.v1".into(),
///     rubric_version: "1.0.0".into(),
///     evidence_hashes: vec![[0u8; 32]],
///     created_at: chrono::Utc::now(),
/// };
/// assert_eq!(record.rubric_id, "memory.v1");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeRecord {
    /// ID.
    pub id: JudgeRecordId,
    /// Owning session.
    pub session_id: SessionId,
    /// What was judged.
    pub subject: JudgeSubject,
    /// The verdict.
    pub verdict: JudgeVerdict,
    /// Which model judged.
    pub judge_model: ProviderModel,
    /// Rubric ID.
    pub rubric_id: String,
    /// Rubric version.
    pub rubric_version: String,
    /// Sha256 hashes of inputs the judge actually received.
    pub evidence_hashes: Vec<[u8; 32]>,
    /// Creation time.
    pub created_at: DateTime<Utc>,
}
