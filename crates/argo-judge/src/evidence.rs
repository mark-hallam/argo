//! Evidence input passed to [`crate::Judge::adjudicate`].
use argo_types::{ToolCallRequest, ToolCallResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// What artifact the agent is asking the judge to bless.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProposedArtifact {
    /// Brand-new skill creation.
    SkillCreate {
        /// Skill name.
        name: String,
        /// Full body (frontmatter + Markdown).
        body: String,
    },
    /// Refining an existing skill.
    SkillRefine {
        /// Skill name.
        name: String,
        /// Full new body.
        body: String,
    },
    /// Memory write (rare; usually skipped).
    MemoryWrite {
        /// Entry id (target row).
        entry_id: Uuid,
        /// Body being written.
        body: String,
    },
    /// Subagent delegation outcome.
    DelegateOutcome {
        /// Free-form summary of the child's output.
        summary: String,
    },
    /// No artifact under review (judge runs on the turn alone).
    None,
}

/// One optional ground-truth check the agent loop / harness has already
/// computed (e.g. "test suite passed", "type-check is clean").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthCheck {
    /// Stable id (e.g. `"cargo_test"`).
    pub id: String,
    /// Did the check pass?
    pub passed: bool,
    /// Human-readable note shown to the judge.
    pub note: String,
}

/// All the input the judge model gets to evaluate one decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeEvidence {
    /// What the user asked.
    pub task_statement: String,
    /// Final assistant output (what the agent produced).
    pub agent_output: String,
    /// Tool calls the assistant emitted during the turn.
    pub tool_calls: Vec<ToolCallRequest>,
    /// Tool results returned (1:1 with `tool_calls` by `id`).
    pub tool_results: Vec<ToolCallResult>,
    /// What's being proposed for blessing.
    pub proposed_artifact: ProposedArtifact,
    /// Ground-truth checks (already evaluated).
    pub ground_truth: Vec<GroundTruthCheck>,
}

impl JudgeEvidence {
    /// SHA-256 over each evidence component, in canonical-JSON form.
    /// Persisted alongside every `JudgeRecord` so post-hoc audits can
    /// confirm the judge saw exactly what we said it did.
    #[must_use]
    pub fn evidence_hashes(&self) -> Vec<[u8; 32]> {
        let parts: Vec<serde_json::Value> = vec![
            serde_json::json!({"task_statement": self.task_statement}),
            serde_json::json!({"agent_output": self.agent_output}),
            serde_json::json!({"tool_calls": self.tool_calls}),
            serde_json::json!({"tool_results": self.tool_results}),
            serde_json::json!({"proposed_artifact": self.proposed_artifact}),
            serde_json::json!({"ground_truth": self.ground_truth}),
        ];
        parts.iter().map(canonical_sha256).collect()
    }
}

fn canonical_sha256(v: &serde_json::Value) -> [u8; 32] {
    let canonical = serde_json::to_string(v).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(canonical.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}
