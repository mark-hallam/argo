//! HIPAA deployment posture breadcrumb emitted at gateway startup.

use argo_audit::compute_chain_hash;
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::Row;
use std::path::Path;
use uuid::Uuid;

const KIND: &str = "hipaa_compliance_check";

pub(super) async fn emit_startup_check(cwd: &Path, store: &SessionStore) -> ArgoResult<()> {
    let enabled = hipaa_enabled(cwd);
    let payload = posture_payload(enabled);
    append_audit_row(store, payload).await
}

pub(super) async fn emit_startup_check_best_effort(cwd: &Path, store: &SessionStore) {
    if let Err(e) = emit_startup_check(cwd, store).await {
        eprintln!("argo gateway: hipaa compliance check audit failed ({e})");
    }
}

fn hipaa_enabled(cwd: &Path) -> bool {
    ArgoConfig::load(&cwd.join("argo.toml"))
        .map(|cfg| cfg.security.hipaa.enabled)
        .unwrap_or(false)
}

#[cfg(test)]
pub(super) fn hipaa_enabled_from_toml(raw: &str) -> bool {
    toml::from_str::<ArgoConfig>(raw)
        .map(|cfg| cfg.security.hipaa.enabled)
        .unwrap_or(false)
}

pub(super) fn posture_payload(enabled: bool) -> Value {
    let mode = if enabled { "claimed" } else { "not_claimed" };
    json!({
        "mode": mode,
        "hipaa_enabled": enabled,
        "checked_at": Utc::now().to_rfc3339(),
        "features": [
            feature("access_control", "available", "gateway bearer auth, RBAC audit rows, elevated-view grants"),
            feature("audit_controls", "available", "hash-chained audit_record table and audit watchdog"),
            feature("integrity_controls", "available", "audit chain hashes plus append-only migration discipline"),
            feature("transmission_security", "operator_required", "terminate TLS at a reverse proxy or future native TLS path"),
            feature("at_rest_encryption", "operator_required", "SQLCipher/encrypted-volume deployment required for PHI"),
            feature("phi_fence", "available", "healthcare intake PHI fence and PII redaction primitives"),
            feature("disclosure_log", "follow_up_required", "J17 disclosure-log brief remains separate"),
            feature("breach_notification", "documented", "60-day workflow in docs/skycot/security/hipaa-deployment.md"),
            feature("baa_provider_review", "operator_required", "operator must execute BAAs with cloud and LLM providers")
        ]
    })
}

fn feature(name: &str, status: &str, evidence: &str) -> Value {
    json!({ "name": name, "status": status, "evidence": evidence })
}

async fn append_audit_row(store: &SessionStore, payload: Value) -> ArgoResult<()> {
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Schema(format!("hipaa posture serialize: {e}")))?;
    let parent_hash = last_hash_for_global_chain(store).await?;
    let self_hash = compute_chain_hash(&parent_hash, KIND, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at, principal_id) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(KIND)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("hipaa compliance audit insert: {e}")))?;
    Ok(())
}

async fn last_hash_for_global_chain(store: &SessionStore) -> ArgoResult<[u8; 32]> {
    let row = sqlx::query(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("hipaa compliance audit last_hash: {e}")))?;
    let Some(row) = row else {
        return Ok([0u8; 32]);
    };
    let bytes: Vec<u8> = row
        .try_get("self_hash")
        .map_err(|e| ArgoError::Provider(format!("hipaa compliance audit self_hash: {e}")))?;
    let mut out = [0u8; 32];
    if bytes.len() == 32 {
        out.copy_from_slice(&bytes);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "hipaa_startup_check_tests.rs"]
mod tests;
