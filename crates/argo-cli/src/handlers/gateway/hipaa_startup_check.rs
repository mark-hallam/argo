//! HIPAA deployment posture breadcrumb emitted at gateway startup.

use argo_audit::compute_chain_hash;
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{Sqlite, Transaction};
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
    // `BEGIN IMMEDIATE` serializes the read-then-insert so a concurrent
    // append can't fork the chain (fix-audit-chain-append-serialization).
    let mut tx = store
        .pool()
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| ArgoError::Provider(format!("hipaa compliance audit tx begin: {e}")))?;
    let parent_hash = read_global_parent(&mut tx).await?;
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
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("hipaa compliance audit insert: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("hipaa compliance audit tx commit: {e}")))?;
    Ok(())
}

/// Read the global chain tail inside the caller's transaction.
async fn read_global_parent(tx: &mut Transaction<'_, Sqlite>) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("hipaa compliance audit last_hash: {e}")))?;
    let mut out = [0u8; 32];
    if let Some(bytes) = bytes {
        if bytes.len() == 32 {
            out.copy_from_slice(&bytes);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "hipaa_startup_check_tests.rs"]
mod tests;
