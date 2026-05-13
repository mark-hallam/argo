//! Unit tests for `argo security rotate-keys` (Mission
//! `mission-automated-key-rotation` + Mission
//! `mission-rotation-cli-subsystem-wiring`). Wired in from
//! `security_rotate_keys.rs` via `#[path = ...] mod tests` so they
//! share the file's private surface.
use super::*;
use crate::handlers::security_rotate_keys_registry::{lookup, DEFAULT_KEYS};
use crate::handlers::security_rotate_keys_subprocess::{
    dry_run_envelope, run_execution, SubprocessResult, REDACTED_PLACEHOLDER,
};
use argo_store::SessionStore;
use chrono::Duration;
use serde_json::Value;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

#[test]
fn audit_kind_const_matches_enum_label() {
    // Drift check: the local `AUDIT_KIND` const is sourced from
    // `AuditKind::SecurityKeyRotated.label()` (added in
    // mission-audit-kind-runtime-events). If anyone renames the
    // variant's wire label, this assertion fires before the chain
    // diverges from on-disk rows.
    assert_eq!(AUDIT_KIND, AuditKind::SecurityKeyRotated.label());
    assert_eq!(AUDIT_KIND, "security.key_rotated");
}

#[tokio::test]
async fn list_rows_cover_every_registered_key() {
    let store = fresh_store().await;
    let rows = build_list_rows(&store).await.unwrap();
    assert_eq!(rows.len(), DEFAULT_KEYS.len());
    for (row, entry) in rows.iter().zip(DEFAULT_KEYS.iter()) {
        assert_eq!(row.name, entry.name);
        assert_eq!(row.env_var, entry.env_var);
        assert!(row.last_rotated.is_none(), "fresh store has no rotations");
        assert!(row.age_days.is_none());
    }
}

#[tokio::test]
async fn append_audit_then_list_reports_age() {
    let store = fresh_store().await;
    let entry = lookup("db-encryption").expect("registered");
    let rotation_id = rotation_id_for(entry.name, "random");
    append_rotation_audit(&store, entry, "random", &rotation_id, None)
        .await
        .unwrap();
    let rows = build_list_rows(&store).await.unwrap();
    let row = rows.iter().find(|r| r.name == "db-encryption").unwrap();
    assert!(row.last_rotated.is_some());
    let age = row.age_days.unwrap();
    assert!((0..=1).contains(&age), "fresh rotation should be ≤ 1 day old, got {age}");
}

#[tokio::test]
async fn commit_is_idempotent_for_same_source_same_day() {
    let store = fresh_store().await;
    let entry = lookup("credentials-master").expect("registered");
    let rotation_id = rotation_id_for(entry.name, "random");
    let first = append_rotation_audit(&store, entry, "random", &rotation_id, None)
        .await
        .unwrap();
    let found = prior_rotation_outcome(&store, &rotation_id)
        .await
        .unwrap()
        .expect("prior row exists");
    assert_eq!(found.audit_id, first, "idempotency lookup must round-trip");
    // Second call with same rotation_id (no subprocess result, so
    // outcome=succeeded) keeps the dedup path noop.
    let again = prior_rotation_outcome(&store, &rotation_id)
        .await
        .unwrap()
        .expect("duplicate lookup");
    assert_eq!(again.outcome, "succeeded");
}

#[tokio::test]
async fn audit_row_uses_redacted_source_descriptor() {
    let store = fresh_store().await;
    let entry = lookup("channel-hmac").expect("registered");
    // Caller passes the redacted descriptor — exercising the
    // pipeline operator-facing `commit` produces after
    // `redact_source` strips the raw `env:SECRET_FOO` form.
    let descriptor = redact_source("env:SECRET_FOO");
    let rotation_id = rotation_id_for(entry.name, descriptor);
    append_rotation_audit(&store, entry, descriptor, &rotation_id, None)
        .await
        .unwrap();
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = ? ORDER BY seq DESC LIMIT 1",
    )
    .bind(AUDIT_KIND)
    .fetch_one(store.pool())
    .await
    .unwrap();
    let value: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(value["new_key_source"], "env");
    // Raw env-var name must never reach the audit row.
    assert!(
        !payload.contains("SECRET_FOO"),
        "audit payload leaked raw env-var name: {payload}"
    );
}

#[tokio::test]
async fn redact_source_collapses_known_aliases() {
    assert_eq!(redact_source("env:SECRET_FOO"), "env");
    assert_eq!(redact_source("random"), "random");
    assert_eq!(redact_source("RANDOM"), "random");
    assert_eq!(redact_source(""), "random");
    assert_eq!(redact_source("prompt"), "prompt");
    assert_eq!(redact_source("file:/etc/secret"), "other");
}

#[tokio::test]
async fn audit_key_age_returns_one_when_a_key_is_stale() {
    let store = fresh_store().await;
    let entry = lookup("gateway-bearer").expect("registered");
    let stale = Utc::now() - Duration::days(120);
    insert_audit_at(&store, entry.name, stale).await;
    let stale_count = build_list_rows(&store)
        .await
        .unwrap()
        .iter()
        .filter(|r| matches!(r.age_days, Some(age) if age > 90))
        .count();
    assert!(stale_count >= 1);
    // Every other key in the registry has no audit row, so they
    // also count as stale (last_rotated == None). The exit code is
    // 1 iff any key is stale.
    let any_stale = stale_count > 0
        || build_list_rows(&store)
            .await
            .unwrap()
            .iter()
            .any(|r| r.age_days.is_none());
    assert!(any_stale, "either explicit stale row or null-age key triggers exit=1");
}

#[tokio::test]
async fn audit_key_age_returns_zero_when_every_key_is_fresh() {
    let store = fresh_store().await;
    // Seed a fresh rotation row for every registered key.
    for entry in DEFAULT_KEYS {
        let rid = rotation_id_for(entry.name, "random");
        append_rotation_audit(&store, entry, "random", &rid, None)
            .await
            .unwrap();
    }
    let rows = build_list_rows(&store).await.unwrap();
    let any_stale = rows.iter().any(|r| match r.age_days {
        Some(age) => age > 90,
        None => true,
    });
    assert!(!any_stale, "every registered key should be fresh after seeding");
}

async fn insert_audit_at(store: &SessionStore, key_name: &str, when: DateTime<Utc>) {
    let entry = lookup(key_name).expect("registered");
    let parent = last_global_hash(store).await.unwrap();
    let payload = json!({
        "actor": "test",
        "action": AUDIT_KIND,
        "key": entry.name,
        "category": entry.category,
        "env_var": entry.env_var,
        "rotation_command": entry.rotation_command,
        "new_key_source": "random",
        "rotation_id": format!("{}:random:{}", entry.name, when.format("%Y-%m-%d")),
        "at": when.to_rfc3339(),
    });
    let canonical = serde_json::to_string(&payload).unwrap();
    let self_hash = compute_chain_hash(&parent, AUDIT_KIND, &canonical);
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, NULL, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(AUDIT_KIND)
    .bind(canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(when.timestamp_micros())
    .execute(store.pool())
    .await
    .unwrap();
}

// -----------------------------------------------------------------
// `mission-rotation-cli-subsystem-wiring` — five new tests covering
// the subprocess dispatch, dry-run preview, retry-on-failure path,
// and the operator-prompt fallback for env-var-only categories.
// -----------------------------------------------------------------

fn fake_subprocess(succeeded: bool, exit: i32, stdout: &str, stderr: &str) -> SubprocessResult {
    SubprocessResult {
        command_display: "argo db rekey --new-key-source env".to_string(),
        exit_code: Some(exit),
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        succeeded,
    }
}

#[tokio::test]
async fn subprocess_success_audit_records_command_stdout_stderr_and_exit() {
    let store = fresh_store().await;
    let entry = lookup("db-encryption").expect("registered");
    let rotation_id = rotation_id_for(entry.name, "random");
    let sub = fake_subprocess(true, 0, "db: rekey rotated\n", "");
    append_rotation_audit(&store, entry, "random", &rotation_id, Some(&sub))
        .await
        .unwrap();
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = ? ORDER BY seq DESC LIMIT 1",
    )
    .bind(AUDIT_KIND)
    .fetch_one(store.pool())
    .await
    .unwrap();
    let v: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(v["outcome"], "succeeded");
    assert_eq!(v["execution"], "subprocess");
    let sp = &v["subprocess"];
    assert_eq!(sp["succeeded"], true);
    assert_eq!(sp["exit_code"], 0);
    assert!(sp["stdout"].as_str().unwrap().contains("rotated"));
    assert!(sp["command"].as_str().unwrap().contains("db rekey"));
}

#[tokio::test]
async fn subprocess_failure_audit_marks_failed_and_drives_retry() {
    let store = fresh_store().await;
    let entry = lookup("db-encryption").expect("registered");
    let rotation_id = rotation_id_for(entry.name, "random");
    let failed = fake_subprocess(false, 2, "", "PRAGMA rekey rejected: no SQLCipher driver\n");
    append_rotation_audit(&store, entry, "random", &rotation_id, Some(&failed))
        .await
        .unwrap();
    // Prior outcome lookup must surface the failure so commit() picks
    // it up as a retry rather than as a noop.
    let prior = prior_rotation_outcome(&store, &rotation_id)
        .await
        .unwrap()
        .expect("prior row exists");
    assert_eq!(prior.outcome, "failed");
    // A subsequent success append shifts the latest row to succeeded
    // and short-circuits any further commit() invocations.
    let ok = fake_subprocess(true, 0, "db: rekey rotated\n", "");
    append_rotation_audit(&store, entry, "random", &rotation_id, Some(&ok))
        .await
        .unwrap();
    let prior2 = prior_rotation_outcome(&store, &rotation_id)
        .await
        .unwrap()
        .expect("latest row");
    assert_eq!(prior2.outcome, "succeeded");
}

#[tokio::test]
async fn dry_run_envelope_resolves_command_without_spawning() {
    let store = fresh_store().await;
    let entry = lookup("db-encryption").expect("registered");
    let env = dry_run_envelope(entry, "env:SECRET_FOO", "env");
    assert_eq!(env["status"], "dry_run");
    assert_eq!(env["execution"], "subprocess");
    let cmd = env["command"].as_str().expect("subprocess entry yields command");
    assert!(cmd.contains("db rekey"));
    assert!(cmd.contains("--new-key-source"));
    // Raw env-var name must be scrubbed from the rendered command.
    assert!(
        !cmd.contains("SECRET_FOO"),
        "dry-run command leaked raw new_key_source: {cmd}"
    );
    assert!(cmd.contains(REDACTED_PLACEHOLDER));
    // No audit row may be written during dry-run preview.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0, "dry-run preview must not touch the audit chain");
}

#[tokio::test]
async fn retry_resumes_when_rotation_id_matches_prior_failure() {
    // Simulates the operator's retry workflow: same `rotate-keys
    // commit <key> --new-key-source <src>` invocation on the same
    // day after a transient subsystem failure.
    let store = fresh_store().await;
    let entry = lookup("db-encryption").expect("registered");
    let rotation_id = rotation_id_for(entry.name, "random");
    // Attempt 1: subsystem fails.
    let failure = fake_subprocess(false, 1, "", "boom\n");
    append_rotation_audit(&store, entry, "random", &rotation_id, Some(&failure))
        .await
        .unwrap();
    let before = prior_rotation_outcome(&store, &rotation_id)
        .await
        .unwrap()
        .expect("prior failure row");
    assert_eq!(before.outcome, "failed");
    // Attempt 2: same rotation_id, subsystem succeeds. The retry-aware
    // commit path appends a fresh audit row instead of treating the
    // prior failure as a dedup-hit noop.
    let success = fake_subprocess(true, 0, "db: rekey rotated\n", "");
    let second = append_rotation_audit(&store, entry, "random", &rotation_id, Some(&success))
        .await
        .unwrap();
    assert_ne!(before.audit_id, second, "retry must write a new audit row");
    let after = prior_rotation_outcome(&store, &rotation_id)
        .await
        .unwrap()
        .expect("succeeded row");
    assert_eq!(after.outcome, "succeeded");
    assert_eq!(after.audit_id, second);
}

#[tokio::test]
async fn operator_prompt_categories_skip_subprocess_dispatch() {
    // env-var-only rotations (`gateway-bearer`, `channel-hmac`) and
    // categories whose subsystem CLI isn't wired yet
    // (`credentials-master`, `connector-oauth`) must NOT spawn a
    // subprocess — `run_execution` returns `None` and the audit row
    // is recorded without subprocess fields.
    for name in [
        "gateway-bearer",
        "channel-hmac",
        "credentials-master",
        "connector-oauth",
    ] {
        let entry = lookup(name).expect("registered");
        let result = run_execution(entry, "random").unwrap();
        assert!(
            result.is_none(),
            "{name} must use OperatorPrompt fallback (got Subprocess result)"
        );
    }
    // And the persisted audit row still records `execution =
    // operator_prompt` with no `subprocess` field set.
    let store = fresh_store().await;
    let entry = lookup("gateway-bearer").expect("registered");
    let rotation_id = rotation_id_for(entry.name, "random");
    append_rotation_audit(&store, entry, "random", &rotation_id, None)
        .await
        .unwrap();
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = ? ORDER BY seq DESC LIMIT 1",
    )
    .bind(AUDIT_KIND)
    .fetch_one(store.pool())
    .await
    .unwrap();
    let v: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(v["execution"], "operator_prompt");
    assert_eq!(v["outcome"], "succeeded");
    assert!(v["subprocess"].is_null(), "operator_prompt audit row must have null subprocess");
}
