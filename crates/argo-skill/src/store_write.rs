use super::SkillStore;
use crate::skill::{LifecycleState, Skill, SkillFrontmatter};
use argo_store::skill_provenance::{record_provenance, NewSkillProvenance, SkillProvenanceKind};
use argo_types::{ArgoError, ArgoResult, Lock, SessionId, SkillId};
use chrono::Utc;
use sha2::{Digest, Sha256};

impl SkillStore {
    pub(crate) async fn insert_row(
        &self,
        frontmatter: SkillFrontmatter,
        body: String,
        learned_in: Option<SessionId>,
        vertical_id: Option<&str>,
    ) -> ArgoResult<Skill> {
        self.insert_row_with_state(
            frontmatter,
            body,
            learned_in,
            vertical_id,
            LifecycleState::Active,
        )
        .await
    }

    pub(crate) async fn insert_row_with_state(
        &self,
        mut frontmatter: SkillFrontmatter,
        body: String,
        learned_in: Option<SessionId>,
        vertical_id: Option<&str>,
        state: LifecycleState,
    ) -> ArgoResult<Skill> {
        let id = SkillId::new();
        let created_at = Utc::now();
        let hash = sha256(&body);
        frontmatter.lock = Lock::None;
        let learned_in_string = learned_in.map(|s| s.0.to_string());
        let frontmatter_json = serde_json::to_string(&frontmatter)
            .map_err(|e| ArgoError::Schema(format!("frontmatter serialize: {e}")))?;
        sqlx::query(
            "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, \
             learned_in, created_at, vertical_id, lifecycle_state) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.0.as_bytes().as_slice())
        .bind(&frontmatter.name)
        .bind(&frontmatter.version)
        .bind(&body)
        .bind(&frontmatter_json)
        .bind(&frontmatter.provenance.source)
        .bind(hash.as_slice())
        .bind(learned_in.map(|s| s.0.as_bytes().to_vec()))
        .bind(created_at.timestamp_micros())
        .bind(vertical_id)
        .bind(state.label())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill install: {e}")))?;
        record_provenance(
            self.store.pool(),
            NewSkillProvenance {
                skill_name: &frontmatter.name,
                skill_version: &frontmatter.version,
                kind: provenance_kind(&frontmatter.provenance.source),
                actor: actor_for_source(&frontmatter.provenance.source),
                parent_id: learned_in_string.as_deref(),
                change_reason: Some("skill row inserted"),
                breaking_change: false,
            },
        )
        .await?;
        Ok(Skill {
            id,
            frontmatter,
            body,
            hash,
            learned_in,
            superseded_by: None,
            created_at,
            vertical_id: vertical_id.map(str::to_string),
            lifecycle_state: state,
        })
    }
}

fn provenance_kind(source: &str) -> SkillProvenanceKind {
    if source == "builtin" || source.starts_with("bundle:") {
        SkillProvenanceKind::Bootstrap
    } else if source == "forge" {
        SkillProvenanceKind::ForgePromoted
    } else if source == "user" || source == "cli-create" || source.starts_with("refined-from-") {
        SkillProvenanceKind::OperatorAuthored
    } else if source == "learned" || source.starts_with("learned:") {
        SkillProvenanceKind::DreamsDerived
    } else {
        SkillProvenanceKind::Imported
    }
}

fn actor_for_source(source: &str) -> Option<&'static str> {
    if source == "user" || source == "cli-create" || source.starts_with("refined-from-") {
        Some("operator")
    } else if source == "forge" {
        Some("forge")
    } else {
        None
    }
}

fn sha256(body: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}
