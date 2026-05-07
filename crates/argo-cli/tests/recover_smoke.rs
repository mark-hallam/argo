//! Integration smoke for `argo recover` (Stream
//! `claude-error-recovery-suggestions`).
//!
//! - First test exercises the cache-hit path end-to-end with no
//!   LLM call: pre-populate `recovery_cache`, run `argo recover`,
//!   assert the JSON envelope round-trips.
//! - Second test asserts the missing-audit-chain error path.
//! - Third test is gated on `ANTHROPIC_API_KEY` and is the only
//!   real-network test.

use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{ArgoResult, SessionId};
use chrono::Utc;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use uuid::Uuid;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

async fn seed_session_with_failing_chain(
    home: &Path,
) -> ArgoResult<(SessionId, String)> {
    let store = SessionStore::open(home.join("sessions.db")).await?;
    store.migrate().await?;
    let root = home.to_string_lossy().to_string();
    let (tenant, workspace) = store.ensure_workspace("recover-smoke", &root).await?;
    let session = store
        .create_session(tenant, workspace, "mock", "mock", Some("recover"), None)
        .await?;
    let sink = AuditSink::new(Arc::new(store));
    sink.write(
        Some(session),
        AuditKind::ToolCall,
        serde_json::json!({"tool": "shell", "args": ["echo", "hi"]}),
    )
    .await?;
    sink.write(
        Some(session),
        AuditKind::ToolResult,
        serde_json::json!({"is_error": true, "reason": "exit code 1"}),
    )
    .await?;
    let chain_hash = compute_chain_hash_from_db(home, session).await?;
    Ok((session, chain_hash))
}

async fn compute_chain_hash_from_db(home: &Path, session: SessionId) -> ArgoResult<String> {
    let store = SessionStore::open(home.join("sessions.db")).await?;
    let rows = store.list_audit_for_session(session).await?;
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for row in &rows {
        hasher.update(row.seq.to_le_bytes());
        hasher.update(row.kind.as_bytes());
        hasher.update(b"\0");
        hasher.update(row.payload_json.as_bytes());
        hasher.update(b"\0");
    }
    Ok(hex::encode(hasher.finalize()))
}

async fn write_recovery_cache(
    home: &Path,
    session: SessionId,
    chain_hash: &str,
    suggestions_json: &str,
) -> ArgoResult<()> {
    let store = SessionStore::open(home.join("sessions.db")).await?;
    let schema = include_str!("../../argo-store/migrations/28__recovery_cache.sql");
    for stmt in schema.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| argo_types::ArgoError::Provider(format!("schema: {e}")))?;
    }
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT OR REPLACE INTO recovery_cache \
         (session_id, chain_hash, suggestions_json, failure_kind, cached_at_micros) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(session.0.to_string())
    .bind(chain_hash)
    .bind(suggestions_json)
    .bind("tool_result")
    .bind(now)
    .execute(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("seed cache: {e}")))?;
    Ok(())
}

#[tokio::test]
async fn cache_hit_returns_seeded_suggestions_without_llm_call() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    let (session, chain_hash) = seed_session_with_failing_chain(home.path()).await.unwrap();
    let suggestions = serde_json::json!({
        "likely_cause": "Tool exited with non-zero status",
        "fixes": [
            {"rank": 1, "title": "retry", "command": "argo action replay X", "rationale": "transient"},
            {"rank": 2, "title": "raise timeout", "command": "argo worker update", "rationale": "slow"}
        ]
    });
    write_recovery_cache(
        home.path(),
        session,
        &chain_hash,
        &suggestions.to_string(),
    )
    .await
    .unwrap();

    // Run with NO ANTHROPIC_API_KEY — must hit cache.
    let out = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .env_remove("ANTHROPIC_API_KEY")
        .args(["recover", &session.0.to_string(), "--json"])
        .output()
        .unwrap();
    assert_ok(&out, "argo recover (cache hit)");
    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(envelope["from_cache"], true);
    assert_eq!(envelope["chain_hash"], chain_hash);
    assert_eq!(envelope["suggestions"]["likely_cause"], "Tool exited with non-zero status");
    let fixes = envelope["suggestions"]["fixes"].as_array().unwrap();
    assert_eq!(fixes.len(), 2);
}

#[test]
fn missing_session_audit_chain_errors_cleanly() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let unknown_session = Uuid::now_v7().to_string();
    let bad = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .env_remove("ANTHROPIC_API_KEY")
        .args(["recover", &unknown_session])
        .output()
        .unwrap();
    assert!(!bad.status.success(), "expected non-zero on unknown session");
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("no audit rows") || stderr.contains("invalid"),
        "expected helpful error, got: {stderr}",
    );
}

#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY for the real LLM call"]
async fn live_llm_call_returns_structured_suggestions() {
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("ANTHROPIC_API_KEY unset — skipping live test");
        return;
    }
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let (session, _) = seed_session_with_failing_chain(home.path()).await.unwrap();
    let out = run(
        workspace.path(),
        home.path(),
        &["recover", &session.0.to_string(), "--json"],
    );
    assert_ok(&out, "argo recover live");
    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(envelope["from_cache"], false);
    assert!(envelope["suggestions"]["likely_cause"]
        .as_str()
        .map(|s| !s.is_empty())
        .unwrap_or(false));
}
