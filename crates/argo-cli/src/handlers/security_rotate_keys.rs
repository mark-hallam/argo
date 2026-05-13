//! `argo security rotate-keys` coordinator (Mission
//! `mission-automated-key-rotation`).
//!
//! Operator-facing surface that enumerates the workspace's
//! rotation-managed keys, summarises a rotation plan, and records
//! `security.key_rotated` audit rows when a rotation is committed.
//! The CLI does not generate key material or invoke SQLCipher /
//! credential rekey directly — those primitives live in the
//! subsystem-specific commands referenced by `rotation_command`
//! in [`security_rotate_keys_registry`].
//!
//! No key material is logged, printed, or stored by this surface.
//! `--new-key-source` is recorded as a redacted descriptor only
//! (`random` / `prompt` / `env`).
use crate::cli::cli_security_cmds::{
    RotateKeysAuditAgeArgs, RotateKeysCmd, RotateKeysCommitArgs, RotateKeysListArgs,
    RotateKeysPlanArgs,
};
use crate::handlers::security_rotate_keys_registry::{lookup, KeyEntry, DEFAULT_KEYS};
use crate::handlers::security_rotate_keys_subprocess::{
    commit_envelope, print_commit_human, print_dry_run, run_execution, PriorRotation,
    SubprocessResult, ROTATION_FAILED, ROTATION_SUCCEEDED,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{compute_chain_hash, AuditKind};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// Canonical label persisted in `audit_record.kind` for rotation
/// rows. Sourced from `AuditKind::SecurityKeyRotated` (promoted from
/// a local-only string constant in `mission-audit-kind-runtime-events`)
/// so the chain verifier and CLI filters categorize it consistently.
pub const AUDIT_KIND: &str = AuditKind::SecurityKeyRotated.label();
const FORMAT_MARKDOWN: &str = "markdown";
const FORMAT_JSON: &str = "json";

/// Dispatch entrypoint wired from `main.rs`. Runs the matched
/// subcommand and exits non-zero for cron-friendly statuses
/// surfaced by `audit-key-age` (0 = all fresh, 1 = ≥1 stale,
/// 2 = config error).
pub async fn dispatch(cmd: RotateKeysCmd) -> ArgoResult<()> {
    let rc = run(cmd).await?;
    if rc != 0 {
        std::process::exit(rc);
    }
    Ok(())
}

/// Run one `rotate-keys` subcommand and return its desired exit
/// code. Split out from [`dispatch`] so tests can assert exit
/// codes without invoking `std::process::exit`.
pub async fn run(cmd: RotateKeysCmd) -> ArgoResult<i32> {
    match cmd {
        RotateKeysCmd::List(args) => list(args).await.map(|()| 0),
        RotateKeysCmd::Plan(args) => plan(args).await.map(|()| 0),
        RotateKeysCmd::Commit(args) => commit(args).await.map(|()| 0),
        RotateKeysCmd::AuditKeyAge(args) => audit_key_age(args).await,
    }
}

/// Redacted descriptor of a `--new-key-source` value. Never echoes
/// the underlying material — `env:FOO` collapses to `env` (the
/// env-var name itself can be sensitive in some deployments).
fn redact_source(raw: &str) -> &'static str {
    let trimmed = raw.trim();
    if trimmed.starts_with("env:") {
        "env"
    } else if trimmed.eq_ignore_ascii_case("prompt") {
        "prompt"
    } else if trimmed.eq_ignore_ascii_case("random") || trimmed.is_empty() {
        "random"
    } else {
        "other"
    }
}

#[derive(Serialize)]
struct ListRow {
    name: &'static str,
    category: &'static str,
    env_var: &'static str,
    last_rotated: Option<String>,
    age_days: Option<i64>,
}

async fn list(args: RotateKeysListArgs) -> ArgoResult<()> {
    let store = open_store().await?;
    let rows = build_list_rows(&store).await?;
    let json_mode = args.json || args.format.eq_ignore_ascii_case(FORMAT_JSON);
    if json_mode {
        let body = serde_json::to_string_pretty(&rows)
            .map_err(|e| ArgoError::Provider(format!("rotate-keys list json: {e}")))?;
        println!("{body}");
    } else if args.format.eq_ignore_ascii_case(FORMAT_MARKDOWN) {
        print_markdown_table(&rows);
    } else {
        return Err(ArgoError::Provider(format!(
            "rotate-keys list: unsupported --format '{}', expected 'markdown' or 'json'",
            args.format
        )));
    }
    Ok(())
}

async fn build_list_rows(store: &SessionStore) -> ArgoResult<Vec<ListRow>> {
    let now = Utc::now();
    let mut out = Vec::with_capacity(DEFAULT_KEYS.len());
    for entry in DEFAULT_KEYS {
        let last = last_rotated_at(store, entry.name).await?;
        let age = last.map(|ts| (now - ts).num_days());
        out.push(ListRow {
            name: entry.name,
            category: entry.category,
            env_var: entry.env_var,
            last_rotated: last.map(|t| t.to_rfc3339()),
            age_days: age,
        });
    }
    Ok(out)
}

fn print_markdown_table(rows: &[ListRow]) {
    println!("| Name | Category | Env var | Last rotated | Age (days) |");
    println!("| --- | --- | --- | --- | --- |");
    for r in rows {
        let last = r.last_rotated.as_deref().unwrap_or("never");
        let age = r
            .age_days
            .map(|d| d.to_string())
            .unwrap_or_else(|| "—".to_string());
        println!(
            "| {} | {} | {} | {} | {} |",
            r.name, r.category, r.env_var, last, age,
        );
    }
}

async fn plan(args: RotateKeysPlanArgs) -> ArgoResult<()> {
    let entry = resolve_entry(&args.key_name)?;
    let store = open_store().await?;
    let last = last_rotated_at(&store, entry.name).await?;
    if args.json {
        let body = serde_json::to_string_pretty(&plan_envelope(entry, last))
            .map_err(|e| ArgoError::Provider(format!("rotate-keys plan json: {e}")))?;
        println!("{body}");
    } else {
        print_plan_text(entry, last);
    }
    Ok(())
}

fn plan_envelope(entry: &KeyEntry, last: Option<DateTime<Utc>>) -> Value {
    json!({
        "key": entry.name,
        "category": entry.category,
        "env_var": entry.env_var,
        "rotation_command": entry.rotation_command,
        "description": entry.description,
        "last_rotated": last.map(|t| t.to_rfc3339()),
        "operator_followups": entry.operator_followups,
    })
}

fn print_plan_text(entry: &KeyEntry, last: Option<DateTime<Utc>>) {
    println!("# Rotation plan: {} ({})", entry.name, entry.category);
    println!();
    println!("{}", entry.description);
    println!("- env var: {}", entry.env_var);
    if entry.rotation_command.is_empty() {
        println!("- rotation: operator follow-ups only (no subsystem command)");
    } else {
        println!("- subsystem command: `{}`", entry.rotation_command);
    }
    match last {
        Some(t) => println!("- last rotated: {}", t.to_rfc3339()),
        None => println!("- last rotated: never (no prior audit row)"),
    }
    if !entry.operator_followups.is_empty() {
        println!("\nOperator follow-ups:");
        for step in entry.operator_followups {
            println!("  - {step}");
        }
    }
}

async fn commit(args: RotateKeysCommitArgs) -> ArgoResult<()> {
    let entry = resolve_entry(&args.key_name)?;
    let source = redact_source(&args.new_key_source);
    if args.dry_run {
        print_dry_run(entry, &args.new_key_source, source, args.json);
        return Ok(());
    }
    let store = open_store().await?;
    let rotation_id = rotation_id_for(entry.name, source);
    // Retry semantics: dedup against prior audit rows for this
    // rotation_id, but allow re-run when the prior attempt failed.
    let prior = prior_rotation_outcome(&store, &rotation_id).await?;
    let (status, audit_uuid, subprocess_result) = match prior {
        Some(p) if p.outcome == ROTATION_SUCCEEDED => ("noop", p.audit_id, None),
        Some(_) | None => {
            let exec_result = run_execution(entry, &args.new_key_source)?;
            let uuid =
                append_rotation_audit(&store, entry, source, &rotation_id, exec_result.as_ref())
                    .await?;
            let status = match exec_result.as_ref() {
                Some(r) if !r.succeeded => "failed",
                _ => "rotated",
            };
            (status, uuid, exec_result)
        }
    };
    let envelope = commit_envelope(
        entry,
        source,
        status,
        audit_uuid,
        &rotation_id,
        subprocess_result.as_ref(),
    );
    if args.json {
        let body = serde_json::to_string_pretty(&envelope)
            .map_err(|e| ArgoError::Provider(format!("rotate-keys commit json: {e}")))?;
        println!("{body}");
    } else {
        print_commit_human(entry, source, status, audit_uuid, &rotation_id, subprocess_result.as_ref());
    }
    if status == "failed" {
        return Err(ArgoError::Provider(format!(
            "rotate-keys commit: subsystem command exited non-zero for `{}`; re-run to retry.",
            entry.name
        )));
    }
    Ok(())
}

async fn audit_key_age(args: RotateKeysAuditAgeArgs) -> ArgoResult<i32> {
    if args.threshold == 0 {
        return Err(ArgoError::Provider(
            "rotate-keys audit-key-age: --threshold must be > 0".into(),
        ));
    }
    let store = open_store().await?;
    let rows = build_list_rows(&store).await?;
    let threshold = i64::from(args.threshold);
    let mut stale = Vec::new();
    let mut fresh = Vec::new();
    for r in &rows {
        match r.age_days {
            Some(age) if age > threshold => stale.push(r),
            Some(_) => fresh.push(r),
            None => stale.push(r),
        }
    }
    if args.json {
        let envelope = json!({
            "threshold_days": args.threshold,
            "stale": stale.iter().map(|r| r.name).collect::<Vec<_>>(),
            "fresh": fresh.iter().map(|r| r.name).collect::<Vec<_>>(),
        });
        let body = serde_json::to_string_pretty(&envelope)
            .map_err(|e| ArgoError::Provider(format!("rotate-keys audit-key-age json: {e}")))?;
        println!("{body}");
    } else {
        println!("Threshold: {} days", args.threshold);
        if stale.is_empty() {
            println!("All {} registered keys are within the rotation window.", rows.len());
        } else {
            println!("Stale keys ({}):", stale.len());
            for r in &stale {
                let label = r
                    .age_days
                    .map(|d| format!("{d}d"))
                    .unwrap_or_else(|| "never rotated".to_string());
                println!("  - {} ({})", r.name, label);
            }
        }
    }
    Ok(if stale.is_empty() { 0 } else { 1 })
}

fn resolve_entry(name: &str) -> ArgoResult<&'static KeyEntry> {
    lookup(name).ok_or_else(|| {
        let known = DEFAULT_KEYS
            .iter()
            .map(|k| k.name)
            .collect::<Vec<_>>()
            .join(", ");
        ArgoError::Provider(format!(
            "rotate-keys: unknown key `{name}`. Registered keys: {known}"
        ))
    })
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("rotate-keys cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

async fn last_rotated_at(
    store: &SessionStore,
    key_name: &str,
) -> ArgoResult<Option<DateTime<Utc>>> {
    let micros: Option<i64> = sqlx::query_scalar(
        "SELECT created_at FROM audit_record \
         WHERE kind = ? AND json_extract(payload_json, '$.key') = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(AUDIT_KIND)
    .bind(key_name)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("rotate-keys last-rotated query: {e}")))?;
    Ok(micros.and_then(DateTime::<Utc>::from_timestamp_micros))
}

/// Look up the latest audit row for this `rotation_id` and surface
/// its outcome. Drives the retry-on-failure semantics: prior
/// `succeeded` rows short-circuit to noop; prior `failed` rows fall
/// through to a fresh subsystem attempt. Older rows (written before
/// `outcome` was added) default to `succeeded` for backward compat.
async fn prior_rotation_outcome(
    store: &SessionStore,
    rotation_id: &str,
) -> ArgoResult<Option<PriorRotation>> {
    let row: Option<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT id, payload_json FROM audit_record \
         WHERE kind = ? AND json_extract(payload_json, '$.rotation_id') = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(AUDIT_KIND)
    .bind(rotation_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("rotate-keys retry-lookup query: {e}")))?;
    let Some((bytes, payload_json)) = row else {
        return Ok(None);
    };
    let audit_id = Uuid::from_slice(&bytes)
        .map_err(|e| ArgoError::Provider(format!("rotate-keys audit id parse: {e}")))?;
    let parsed: Value = serde_json::from_str(&payload_json)
        .map_err(|e| ArgoError::Provider(format!("rotate-keys audit payload parse: {e}")))?;
    let outcome = match parsed.get("outcome").and_then(|v| v.as_str()) {
        Some(ROTATION_FAILED) => ROTATION_FAILED,
        _ => ROTATION_SUCCEEDED,
    };
    Ok(Some(PriorRotation { audit_id, outcome }))
}

fn rotation_id_for(key_name: &str, redacted_source: &str) -> String {
    let day = Utc::now().format("%Y-%m-%d");
    format!("{key_name}:{redacted_source}:{day}")
}

async fn append_rotation_audit(
    store: &SessionStore,
    entry: &KeyEntry,
    redacted_source: &str,
    rotation_id: &str,
    subprocess_result: Option<&SubprocessResult>,
) -> ArgoResult<Uuid> {
    let parent = last_global_hash(store).await?;
    let at = Utc::now();
    let outcome = match subprocess_result {
        Some(r) if !r.succeeded => ROTATION_FAILED,
        _ => ROTATION_SUCCEEDED,
    };
    let subprocess_payload = subprocess_result.map(|r| {
        json!({
            "command": r.command_display,
            "exit_code": r.exit_code,
            "stdout": r.stdout,
            "stderr": r.stderr,
            "succeeded": r.succeeded,
        })
    });
    let payload = json!({
        "actor": "cli",
        "action": AUDIT_KIND,
        "key": entry.name,
        "category": entry.category,
        "env_var": entry.env_var,
        "rotation_command": entry.rotation_command,
        "new_key_source": redacted_source,
        "rotation_id": rotation_id,
        "execution": entry.execution,
        "subprocess": subprocess_payload,
        "outcome": outcome,
        "at": at.to_rfc3339(),
    });
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("rotate-keys audit json: {e}")))?;
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
    .bind(at.timestamp_micros())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("rotate-keys audit insert: {e}")))?;
    Ok(id)
}

async fn last_global_hash(store: &SessionStore) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("rotate-keys parent-hash query: {e}")))?;
    let mut out = [0_u8; 32];
    if let Some(b) = bytes {
        if b.len() == 32 {
            out.copy_from_slice(&b);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "security_rotate_keys_tests.rs"]
mod tests;
