//! External-judge adjudication types (Amendment 2).
use crate::ids::{JudgeRecordId, SessionId};
use crate::routing::ProviderModel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What did the judge evaluate?
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
