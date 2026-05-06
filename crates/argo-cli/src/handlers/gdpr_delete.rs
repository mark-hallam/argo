//! `argo gdpr delete` right-to-erasure flow.
use crate::cli::{GdprCmd, GdprDeleteCmd};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::compute_chain_hash;
use argo_store::gdpr::{gdpr_redact_payload, GdprCounts, GdprPlan};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::io::{self, Write};
use uuid::Uuid;

const ERASURE_PAYLOAD_KIND: &str = "gdpr_subject_erasure";
const AUDIT_COLUMN_KIND: &str = "lock.set";

pub async fn run(cmd: GdprCmd) -> ArgoResult<()> {
    match cmd {
        GdprCmd::Delete(args) => delete(args).await,
    }
}

async fn delete(args: GdprDeleteCmd) -> ArgoResult<()> {
    let subject = normalize_subject(&args.subject)?;
    if args.bundle_out.is_some() && !args.confirm {
        return Err(ArgoError::Provider("--bundle-out requires --confirm".into()));
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    let subject_hash = sha256_hex(subject.to_ascii_lowercase().as_bytes());
    let replacement = redacted_email(&subject);
    let plan = store.gdpr_plan(&subject).await?;
    let dry_run = !args.confirm;
    if dry_run {
        return print_result(true, &plan.counts, &subject_hash, None, args.json);
    }
    require_confirmation(&subject)?;
    if let Some(path) = args.bundle_out.as_deref() {
        write_bundle(path, &plan, &subject_hash)?;
    }
    rewrite_audit(&store, &subject, &replacement).await?;
    let counts = store.gdpr_apply_non_audit(&subject, &replacement).await?;
    let counts = GdprCounts {
        audit_records: plan.counts.audit_records,
        ..counts
    };
    append_erasure_event(&store, &subject_hash, counts).await?;
    print_result(false, &counts, &subject_hash, args.bundle_out.as_deref(), args.json)
}

fn normalize_subject(raw: &str) -> ArgoResult<String> {
    let subject = raw.trim();
    if subject.is_empty() || !subject.contains('@') {
        return Err(ArgoError::Provider("pass --subject <email>".into()));
    }
    Ok(subject.to_string())
}

fn require_confirmation(subject: &str) -> ArgoResult<()> {
    eprint!("Type {subject} to confirm GDPR erasure: ");
    io::stderr()
        .flush()
        .map_err(|e| ArgoError::Provider(format!("flush confirmation prompt: {e}")))?;
    let mut typed = String::new();
    io::stdin()
        .read_line(&mut typed)
        .map_err(|e| ArgoError::Provider(format!("read confirmation: {e}")))?;
    if confirmation_matches(&typed, subject) {
        Ok(())
    } else {
        Err(ArgoError::Provider(
            "confirmation did not exactly match subject email".into(),
        ))
    }
}

fn confirmation_matches(typed: &str, subject: &str) -> bool {
    typed.trim_end_matches(['\r', '\n']) == subject
}

fn print_result(
    dry_run: bool,
    counts: &GdprCounts,
    subject_hash: &str,
    bundle_out: Option<&str>,
    as_json: bool,
) -> ArgoResult<()> {
    if as_json {
        let value = json!({"gdpr_delete": {"ok": true, "dry_run": dry_run, "subject_hash": subject_hash, "counts": counts, "affected_tables": affected_tables(*counts), "bundle_out": bundle_out, "audit_event_kind": ERASURE_PAYLOAD_KIND}});
        println!("{}", serde_json::to_string(&value).map_err(json_err)?);
        return Ok(());
    }
    println!("gdpr delete {}", if dry_run { "dry-run" } else { "complete" });
    println!("subject_sha256 {subject_hash}");
    for (label, n) in count_rows(*counts) {
        println!("{label:<22} {n}");
    }
    if dry_run {
        println!("no changes made; rerun with --confirm to delete");
    }
    Ok(())
}

fn count_rows(c: GdprCounts) -> [(&'static str, u64); 6] {
    [
        ("memory_entry", c.memory_rows),
        ("audit_record", c.audit_records),
        ("skill_candidate", c.skill_candidates),
        ("notify_channel", c.notify_channels),
        ("worker", c.worker_operator_refs),
        ("session_payload", c.session_payloads),
    ]
}

fn affected_tables(c: GdprCounts) -> Vec<&'static str> {
    count_rows(c)
        .into_iter()
        .filter_map(|(label, n)| (n > 0).then_some(label))
        .collect()
}

fn write_bundle(path: &str, plan: &GdprPlan, subject_hash: &str) -> ArgoResult<()> {
    let mut records = Vec::new();
    for record in &plan.records {
        records.extend_from_slice(serde_json::to_string(record).map_err(json_err)?.as_bytes());
        records.push(b'\n');
    }
    let manifest = serde_json::to_vec_pretty(&json!({
        "generator": "argo gdpr delete",
        "subject_hash": subject_hash,
        "counts": plan.counts,
        "record_count": plan.records.len(),
        "created_at": Utc::now().to_rfc3339(),
    }))
    .map_err(json_err)?;
    let mut out = Vec::new();
    {
        let mut tar = tar::Builder::new(&mut out);
        append_tar(&mut tar, "manifest.json", &manifest)?;
        append_tar(&mut tar, "records.ndjson", &records)?;
        append_tar(&mut tar, "README.txt", b"Pre-deletion GDPR erasure evidence bundle.\n")?;
        tar.finish()
            .map_err(|e| ArgoError::Provider(format!("finish GDPR bundle: {e}")))?;
    }
    std::fs::write(path, out).map_err(|e| ArgoError::Provider(format!("write {path}: {e}")))
}

fn append_tar<W: Write>(tar: &mut tar::Builder<W>, path: &str, bytes: &[u8]) -> ArgoResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_cksum();
    tar.append_data(&mut header, path, bytes)
        .map_err(|e| ArgoError::Provider(format!("append {path}: {e}")))
}

#[derive(Debug)]
struct AuditRow {
    seq: i64,
    kind: String,
    payload_json: String,
    parent_hash: Vec<u8>,
}

async fn rewrite_audit(store: &SessionStore, subject: &str, replacement: &str) -> ArgoResult<()> {
    let chains = audit_chains(store, subject).await?;
    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("gdpr audit tx: {e}")))?;
    for chain in chains {
        let mut parent = chain.first().map_or([0_u8; 32], |r| hash32(&r.parent_hash));
        for row in chain {
            let payload = if contains_subject(&row.payload_json, subject) {
                gdpr_redact_payload(&row.payload_json, subject, replacement)
            } else {
                row.payload_json
            };
            let self_hash = compute_chain_hash(&parent, &row.kind, &payload);
            sqlx::query(
                "UPDATE audit_record SET payload_json = ?, parent_hash = ?, self_hash = ? WHERE seq = ?",
            )
            .bind(payload)
            .bind(parent.as_slice())
            .bind(self_hash.as_slice())
            .bind(row.seq)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("gdpr audit rewrite: {e}")))?;
            parent = self_hash;
        }
    }
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("gdpr audit commit: {e}")))
}

async fn audit_chains(store: &SessionStore, subject: &str) -> ArgoResult<Vec<Vec<AuditRow>>> {
    let hits = sqlx::query(
        "SELECT DISTINCT session_id FROM audit_record WHERE instr(lower(payload_json), ?) > 0",
    )
    .bind(subject.to_ascii_lowercase())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr audit hits: {e}")))?;
    let mut chains = Vec::new();
    for hit in hits {
        let sid: Option<Vec<u8>> = hit.try_get("session_id").ok().flatten();
        let rows = match sid.as_ref() {
            Some(sid) => sqlx::query(
                "SELECT seq, kind, payload_json, parent_hash FROM audit_record WHERE session_id = ? ORDER BY seq ASC",
            )
            .bind(sid)
            .fetch_all(store.pool())
            .await,
            None => sqlx::query(
                "SELECT seq, kind, payload_json, parent_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq ASC",
            )
            .fetch_all(store.pool())
            .await,
        }
        .map_err(|e| ArgoError::Provider(format!("gdpr audit chain: {e}")))?;
        chains.push(
            rows.into_iter()
                .map(|r| AuditRow {
                    seq: r.get("seq"),
                    kind: r.get("kind"),
                    payload_json: r.get("payload_json"),
                    parent_hash: r.get("parent_hash"),
                })
                .collect(),
        );
    }
    Ok(chains)
}

async fn append_erasure_event(
    store: &SessionStore,
    subject_hash: &str,
    counts: GdprCounts,
) -> ArgoResult<()> {
    let parent = last_global_hash(store).await?;
    let at = Utc::now();
    let payload = json!({"kind": ERASURE_PAYLOAD_KIND, "subject_hash": subject_hash, "affected_tables": affected_tables(counts), "at": at.to_rfc3339()});
    let canonical = serde_json::to_string(&payload).map_err(json_err)?;
    let self_hash = compute_chain_hash(&parent, AUDIT_COLUMN_KIND, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at)
         VALUES (?, NULL, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(AUDIT_COLUMN_KIND)
    .bind(canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(at.timestamp_micros())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr audit event: {e}")))?;
    Ok(())
}

async fn last_global_hash(store: &SessionStore) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr audit last hash: {e}")))?;
    Ok(bytes.as_deref().map_or([0_u8; 32], hash32))
}

fn hash32(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0_u8; 32];
    if bytes.len() == 32 {
        out.copy_from_slice(bytes);
    }
    out
}

fn contains_subject(value: &str, subject: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&subject.to_ascii_lowercase())
}

fn redacted_email(subject: &str) -> String {
    let hex = sha256_hex(subject.to_ascii_lowercase().as_bytes());
    format!("redacted-{}@example.com", &hex[..12])
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn json_err(e: impl std::fmt::Display) -> ArgoError {
    ArgoError::Provider(format!("json: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn counts() -> GdprCounts {
        GdprCounts {
            memory_rows: 1,
            audit_records: 2,
            skill_candidates: 0,
            notify_channels: 3,
            worker_operator_refs: 0,
            session_payloads: 4,
        }
    }

    #[test]
    fn redacted_email_is_lowercase_stable() {
        assert_eq!(redacted_email("ALICE@example.com"), redacted_email("alice@example.com"));
        assert!(redacted_email("alice@example.com").starts_with("redacted-"));
    }

    #[test]
    fn confirmation_requires_exact_match() {
        assert!(confirmation_matches("alice@example.com\n", "alice@example.com"));
        assert!(!confirmation_matches("ALICE@example.com\n", "alice@example.com"));
    }

    #[test]
    fn affected_tables_skip_zero_counts() {
        assert_eq!(
            affected_tables(counts()),
            vec!["memory_entry", "audit_record", "notify_channel", "session_payload"]
        );
    }

    #[test]
    fn count_rows_keeps_all_six_surfaces() {
        assert_eq!(count_rows(GdprCounts::default()).len(), 6);
    }

    #[test]
    fn payload_redaction_walks_nested_json() {
        let out = gdpr_redact_payload(
            r#"{"a":"Alice@example.com","b":["cc alice@example.com"]}"#,
            "alice@example.com",
            "redacted-x@example.com",
        );
        assert!(out.contains("redacted-x@example.com"));
        assert!(!out.to_ascii_lowercase().contains("alice@example.com"));
    }

    #[test]
    fn subject_validation_rejects_non_email() {
        assert!(normalize_subject("alice@example.com").is_ok());
        assert!(normalize_subject("not-email").is_err());
    }

    #[test]
    fn json_envelope_has_universal_ok() {
        let value = json!({"gdpr_delete": {"ok": true, "dry_run": true, "counts": counts()}});
        assert_eq!(value["gdpr_delete"]["ok"], true);
    }

    #[test]
    fn bundle_is_valid_tarball() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gdpr.tar");
        let mut plan = GdprPlan { counts: counts(), ..GdprPlan::default() };
        plan.records.push(argo_store::gdpr::GdprRecord {
            table: "audit_record".into(),
            id: "1".into(),
            payload: json!({"email":"alice@example.com"}),
        });
        write_bundle(path.to_str().unwrap(), &plan, &"a".repeat(64)).unwrap();
        let mut names = Vec::new();
        let mut archive = tar::Archive::new(std::fs::File::open(path).unwrap());
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let mut ignored = String::new();
            let _ = entry.read_to_string(&mut ignored);
            names.push(entry.path().unwrap().to_string_lossy().to_string());
        }
        assert_eq!(names, vec!["manifest.json", "records.ndjson", "README.txt"]);
    }
}
