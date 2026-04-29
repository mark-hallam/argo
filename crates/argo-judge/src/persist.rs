//! Persistence helper: write a [`argo_types::JudgeRecord`] to the
//! `judge_record` table (§7 schema).
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, JudgeRecord, JudgeSubject, JudgeVerdict};

/// Write `record` to the `judge_record` table.
pub async fn persist_judge_record(store: &SessionStore, record: &JudgeRecord) -> ArgoResult<()> {
    let (verdict_label, reason) = verdict_columns(&record.verdict);
    let (subject_kind, subject_ref) = subject_columns(&record.subject);
    let evidence_json = serde_json::to_string(
        &record
            .evidence_hashes
            .iter()
            .map(hex::encode)
            .collect::<Vec<_>>(),
    )
    .map_err(|e| ArgoError::Schema(format!("evidence hashes serialize: {e}")))?;
    sqlx::query(
        "INSERT INTO judge_record (id, session_id, subject_kind, subject_ref, verdict, \
         reason, judge_provider, judge_model, rubric_id, rubric_version, \
         evidence_hashes, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.id.0.as_bytes().as_slice())
    .bind(record.session_id.0.as_bytes().as_slice())
    .bind(subject_kind)
    .bind(subject_ref)
    .bind(verdict_label)
    .bind(reason)
    .bind(&record.judge_model.provider)
    .bind(&record.judge_model.model)
    .bind(&record.rubric_id)
    .bind(&record.rubric_version)
    .bind(evidence_json)
    .bind(record.created_at.timestamp_micros())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("persist_judge_record: {e}")))?;
    Ok(())
}

fn verdict_columns(v: &JudgeVerdict) -> (&'static str, Option<&str>) {
    match v {
        JudgeVerdict::Success => ("success", None),
        JudgeVerdict::Partial { reason } => ("partial", Some(reason.as_str())),
        JudgeVerdict::Failure { reason } => ("failure", Some(reason.as_str())),
    }
}

fn subject_columns(s: &JudgeSubject) -> (&'static str, String) {
    match s {
        JudgeSubject::SkillCreate { name, hash } => {
            ("skill_create", format!("{}@{}", name, hex::encode(hash)))
        }
        JudgeSubject::SkillRefine {
            name,
            version_from,
            version_to_hash,
        } => (
            "skill_refine",
            format!("{name}:{version_from}->{}", hex::encode(version_to_hash)),
        ),
        JudgeSubject::MemoryWrite { entry_id } => ("memory_write", entry_id.to_string()),
        JudgeSubject::DelegateOutcome { delegate_session } => {
            ("delegate_outcome", delegate_session.0.to_string())
        }
    }
}
