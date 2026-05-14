//! Compliance-oriented `argo audit export`.
//!
//! Orchestration only — input parsing/normalization lives in
//! `audit_export_args`, GDPR redaction in `audit_export_redact`. This
//! file owns the runtime flow (chain verification, row collection,
//! format dispatch, output sink) and the format encoders.
use super::audit_export_args::{ExportFormat, NormalizedExport};
use super::audit_export_redact::redact_value;
use argo_audit::{export_ndjson, verify_all, ExportedRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

pub use super::audit_export_args::ExportArgs;

pub async fn run(store: &SessionStore, args: ExportArgs) -> ArgoResult<()> {
    let query = NormalizedExport::new(args)?;
    ensure_chain_ok(store).await?;
    let mut rows = collect_rows(store, &query).await?;
    if query.gdpr_redact_emails {
        for row in &mut rows {
            redact_value(&mut row.payload);
        }
    }
    let artifact = match query.format {
        ExportFormat::Ndjson => render_ndjson(&rows)?,
        ExportFormat::Csv => render_csv(&rows)?,
        ExportFormat::Tarball => render_tarball(&query, &rows)?,
    };
    let sha = sha256_hex(&artifact);
    if let Some(path) = query.bundle_out.as_deref() {
        std::fs::write(path, &artifact)
            .map_err(|e| ArgoError::Io(format!("write {path}: {e}")))?;
    } else if !query.json {
        let mut out = io::stdout().lock();
        out.write_all(&artifact)
            .and_then(|()| out.flush())
            .map_err(|e| ArgoError::Io(format!("write stdout: {e}")))?;
    }
    if query.json {
        let line = serde_json::to_string(&serde_json::json!({
            "export": {
                "ok": true,
                "format": query.format,
                "rows": rows.len(),
                "bytes": artifact.len(),
                "sha256": sha,
                "destination": query.bundle_out.as_deref(),
                "since": query.since.as_deref(),
                "until": query.until.as_deref(),
                "worker_id": query.worker_id.as_deref(),
                "tenant": query.tenant.as_deref(),
                "session": query.session.as_deref(),
                "gdpr_redact_emails": query.gdpr_redact_emails,
            }
        }))
        .map_err(|e| ArgoError::Provider(format!("serialize export summary: {e}")))?;
        println!("{line}");
    } else if query.bundle_out.is_some() {
        eprintln!("audit export: wrote {} rows", rows.len());
    }
    Ok(())
}

async fn ensure_chain_ok(store: &SessionStore) -> ArgoResult<()> {
    let report = verify_all(store).await?;
    if report.ok {
        return Ok(());
    }
    let detail = report
        .breaks
        .first()
        .map(|b| format!("first break seq={} kind={} ({})", b.seq, b.kind, b.reason))
        .unwrap_or_else(|| "break detail unavailable".to_string());
    Err(ArgoError::Provider(format!(
        "audit chain verification failed before export: {} break(s); {detail}",
        report.breaks.len()
    )))
}

async fn collect_rows(
    store: &SessionStore,
    query: &NormalizedExport,
) -> ArgoResult<Vec<ExportedRow>> {
    let mut rows = Vec::new();
    export_ndjson(store, |row| {
        if query.matches(row) {
            rows.push(row.clone());
        }
        Ok(())
    })
    .await?;
    Ok(rows)
}

fn render_ndjson(rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let mut out = Vec::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|e| ArgoError::Provider(format!("serialize audit row: {e}")))?;
        out.extend_from_slice(line.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

fn render_csv(rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let header = [
        "seq",
        "id",
        "session_id",
        "vertical_id",
        "kind",
        "created_at_micros",
        "payload_json",
        "parent_hash",
        "self_hash",
    ];
    let mut body_rows: Vec<Vec<String>> = Vec::with_capacity(rows.len());
    for row in rows {
        let payload = serde_json::to_string(&row.payload)
            .map_err(|e| ArgoError::Provider(format!("serialize audit payload: {e}")))?;
        body_rows.push(vec![
            row.seq.to_string(),
            row.id.clone(),
            row.session_id.clone().unwrap_or_default(),
            row.vertical_id.clone().unwrap_or_default(),
            row.kind.clone(),
            row.created_at_micros.to_string(),
            payload,
            row.parent_hash.clone(),
            row.self_hash.clone(),
        ]);
    }
    Ok(argo_executor_document_csv::render_rfc4180(&header, &body_rows))
}

fn render_tarball(query: &NormalizedExport, rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let ndjson = render_ndjson(rows)?;
    let ndjson_sha = sha256_hex(&ndjson);
    let manifest = serde_json::to_vec_pretty(&serde_json::json!({
        "generator": "argo audit export",
        "generator_version": env!("CARGO_PKG_VERSION"),
        "format": "tarball",
        "since": query.since,
        "until": query.until,
        "worker_id": query.worker_id,
        "tenant": query.tenant,
        "session": query.session,
        "gdpr_redact_emails": query.gdpr_redact_emails,
        "row_count": rows.len(),
        "audit_ndjson_sha256": ndjson_sha,
    }))
    .map_err(|e| ArgoError::Provider(format!("serialize manifest: {e}")))?;
    let readme = README_TXT.as_bytes();
    let mut out = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut out);
        append_tar_bytes(&mut builder, "audit.ndjson", &ndjson)?;
        append_tar_bytes(&mut builder, "manifest.json", &manifest)?;
        append_tar_bytes(&mut builder, "README.txt", readme)?;
        builder
            .finish()
            .map_err(|e| ArgoError::Provider(format!("finish tarball: {e}")))?;
    }
    Ok(out)
}

fn append_tar_bytes<W: Write>(
    builder: &mut tar::Builder<W>,
    path: &str,
    bytes: &[u8],
) -> ArgoResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_cksum();
    builder
        .append_data(&mut header, path, bytes)
        .map_err(|e| ArgoError::Provider(format!("append {path}: {e}")))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

const README_TXT: &str = "Argo audit export bundle\n\n\
audit.ndjson contains records in seq order. manifest.json records query parameters, generator version, and the sha256 of audit.ndjson. Email redaction is deterministic when enabled.\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ndjson_formatter_redacts_payloads() {
        let mut row = sample_row();
        redact_value(&mut row.payload);
        let out = String::from_utf8(render_ndjson(&[row]).unwrap()).unwrap();
        assert!(out.contains("redacted-"));
        assert!(!out.contains("ops@example.com"));
    }

    #[test]
    fn tarball_contains_manifest_readme_and_ndjson() {
        let query = NormalizedExport::new(args("tarball")).unwrap();
        let bytes = render_tarball(&query, &[sample_row()]).unwrap();
        let mut names = Vec::new();
        let mut archive = tar::Archive::new(bytes.as_slice());
        for entry in archive.entries().unwrap() {
            let entry = entry.unwrap();
            names.push(entry.path().unwrap().to_string_lossy().to_string());
        }
        assert_eq!(names, vec!["audit.ndjson", "manifest.json", "README.txt"]);
    }

    fn args(format: &str) -> ExportArgs {
        ExportArgs {
            format: format.into(),
            bundle_out: None,
            worker_id: Some("worker-a".into()),
            tenant: None,
            session: None,
            since: Some("2026-05-06".into()),
            until: Some("2026-05-07".into()),
            since_minutes: None,
            gdpr_redact_emails: false,
            json: false,
        }
    }

    fn sample_row() -> ExportedRow {
        ExportedRow {
            seq: 1,
            id: "row-1".into(),
            session_id: Some("session-1".into()),
            vertical_id: None,
            kind: "tool_call".into(),
            payload: serde_json::json!({"email":"ops@example.com","tool":"shell"}),
            parent_hash: "00".repeat(32),
            self_hash: "11".repeat(32),
            created_at_micros: 1_778_025_600_000_000,
        }
    }
}
