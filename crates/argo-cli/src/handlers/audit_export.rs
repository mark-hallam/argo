//! Compliance-oriented `argo audit export`.
use argo_audit::{export_ndjson, verify_all, ExportedRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, NaiveDate, Utc};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
pub struct ExportArgs {
    pub format: String,
    pub bundle_out: Option<String>,
    pub worker_id: Option<String>,
    pub tenant: Option<String>,
    pub session: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub since_minutes: Option<i64>,
    pub gdpr_redact_emails: bool,
    pub json: bool,
}
#[derive(Debug)]
struct NormalizedExport {
    format: ExportFormat,
    bundle_out: Option<String>,
    worker_id: Option<String>,
    tenant: Option<String>,
    session: Option<String>,
    session_hex: Option<String>,
    since: Option<String>,
    until: Option<String>,
    since_micros: Option<i64>,
    until_micros: Option<i64>,
    gdpr_redact_emails: bool,
    json: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum ExportFormat {
    Ndjson,
    Csv,
    Tarball,
}
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
            .map_err(|e| ArgoError::Provider(format!("write {path}: {e}")))?;
    } else if !query.json {
        let mut out = io::stdout().lock();
        out.write_all(&artifact)
            .and_then(|()| out.flush())
            .map_err(|e| ArgoError::Provider(format!("write stdout: {e}")))?;
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
impl NormalizedExport {
    fn new(args: ExportArgs) -> ArgoResult<Self> {
        let format = parse_format(&args.format)?;
        let session_hex = match args.session.as_deref() {
            Some(raw) => {
                let uuid = uuid::Uuid::parse_str(raw)
                    .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
                Some(hex::encode(uuid.as_bytes()))
            }
            None => None,
        };
        let since_micros = if let Some(raw) = args.since.as_deref() {
            Some(parse_bound(raw, false, "--since")?)
        } else {
            args.since_minutes
                .map(|minutes| (Utc::now() - chrono::Duration::minutes(minutes)).timestamp_micros())
        };
        let until_micros = args
            .until
            .as_deref()
            .map(|raw| parse_bound(raw, true, "--until"))
            .transpose()?;
        if let (Some(since), Some(until)) = (since_micros, until_micros) {
            if since > until {
                return Err(ArgoError::Provider(
                    "invalid audit export time range: --since is after --until".into(),
                ));
            }
        }
        Ok(Self {
            format,
            bundle_out: args.bundle_out,
            worker_id: args.worker_id,
            tenant: args.tenant,
            session: args.session,
            session_hex,
            since: args.since,
            until: args.until,
            since_micros,
            until_micros,
            gdpr_redact_emails: args.gdpr_redact_emails,
            json: args.json,
        })
    }
    fn matches(&self, row: &ExportedRow) -> bool {
        if self
            .since_micros
            .is_some_and(|since| row.created_at_micros < since)
        {
            return false;
        }
        if self
            .until_micros
            .is_some_and(|until| row.created_at_micros > until)
        {
            return false;
        }
        if self
            .session_hex
            .as_deref()
            .is_some_and(|sid| row.session_id.as_deref() != Some(sid))
        {
            return false;
        }
        if self
            .worker_id
            .as_deref()
            .is_some_and(|worker| !payload_matches(&row.payload, worker, WORKER_POINTERS))
        {
            return false;
        }
        if self
            .tenant
            .as_deref()
            .is_some_and(|tenant| !payload_matches(&row.payload, tenant, TENANT_POINTERS))
        {
            return false;
        }
        true
    }
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
const WORKER_POINTERS: &[&str] = &["/worker_id", "/from_worker_id", "/to/worker_id"];
const TENANT_POINTERS: &[&str] = &["/tenant_id"];
fn payload_matches(payload: &serde_json::Value, expected: &str, pointers: &[&str]) -> bool {
    pointers
        .iter()
        .any(|ptr| payload.pointer(ptr).and_then(|v| v.as_str()) == Some(expected))
}
fn render_csv(rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let mut out = String::from(
        "seq,id,session_id,vertical_id,kind,created_at_micros,payload_json,parent_hash,self_hash\n",
    );
    for row in rows {
        let payload = serde_json::to_string(&row.payload)
            .map_err(|e| ArgoError::Provider(format!("serialize audit payload: {e}")))?;
        let fields = [
            row.seq.to_string(),
            row.id.clone(),
            row.session_id.clone().unwrap_or_default(),
            row.vertical_id.clone().unwrap_or_default(),
            row.kind.clone(),
            row.created_at_micros.to_string(),
            payload,
            row.parent_hash.clone(),
            row.self_hash.clone(),
        ];
        out.push_str(
            &fields
                .iter()
                .map(|field| csv_escape(field))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    Ok(out.into_bytes())
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
fn parse_format(raw: &str) -> ArgoResult<ExportFormat> {
    match raw {
        "ndjson" => Ok(ExportFormat::Ndjson),
        "csv" => Ok(ExportFormat::Csv),
        "tarball" => Ok(ExportFormat::Tarball),
        other => Err(ArgoError::Provider(format!(
            "unsupported audit export format '{other}' (expected ndjson, csv, or tarball)"
        ))),
    }
}
fn parse_bound(raw: &str, until: bool, flag: &str) -> ArgoResult<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.timestamp_micros());
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|e| {
        ArgoError::Provider(format!(
            "invalid {flag} date '{raw}': {e} (expected YYYY-MM-DD or RFC3339)"
        ))
    })?;
    let time = if until {
        date.and_hms_micro_opt(23, 59, 59, 999_999)
    } else {
        date.and_hms_micro_opt(0, 0, 0, 0)
    }
    .ok_or_else(|| ArgoError::Provider(format!("invalid {flag} date '{raw}'")))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(time, Utc).timestamp_micros())
}
fn redact_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => *s = redact_emails_in_str(s),
        serde_json::Value::Array(items) => {
            for item in items {
                redact_value(item);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                redact_value(item);
            }
        }
        _ => {}
    }
}
fn redact_emails_in_str(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut cursor = 0_usize;
    for (idx, byte) in bytes.iter().enumerate() {
        if *byte != b'@' {
            continue;
        }
        let start = scan_left(bytes, idx);
        let end = scan_right(bytes, idx + 1);
        if start == idx || end == idx + 1 {
            continue;
        }
        let candidate = &input[start..end];
        if !candidate
            .rsplit_once('.')
            .is_some_and(|(_, tld)| tld.as_bytes().iter().all(u8::is_ascii_alphanumeric))
        {
            continue;
        }
        if start < cursor {
            continue;
        }
        out.push_str(&input[cursor..start]);
        out.push_str(&redacted_email(candidate));
        cursor = end;
    }
    out.push_str(&input[cursor..]);
    out
}
fn scan_left(bytes: &[u8], at: usize) -> usize {
    let mut pos = at;
    while pos > 0 && is_email_local(bytes[pos - 1]) {
        pos -= 1;
    }
    pos
}
fn scan_right(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && is_email_domain(bytes[pos]) {
        pos += 1;
    }
    pos
}
fn is_email_local(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'%' | b'+' | b'-')
}
fn is_email_domain(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-')
}
fn redacted_email(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(raw.to_ascii_lowercase().as_bytes());
    let hex = hex::encode(h.finalize());
    format!("redacted-{}@example.com", &hex[..12])
}
fn csv_escape(raw: &str) -> String {
    if raw.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw.to_string()
    }
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
    fn parse_bound_accepts_date_and_rfc3339() {
        assert_eq!(
            parse_bound("2026-05-06", false, "--since").unwrap(),
            1_778_025_600_000_000
        );
        assert_eq!(
            parse_bound("2026-05-06T01:02:03Z", true, "--until").unwrap(),
            1_778_029_323_000_000
        );
    }
    #[test]
    fn redacts_email_consistently_in_nested_json() {
        let mut value = serde_json::json!({"email":"ALICE@example.com","note":"cc alice@example.com and bob@example.org","items":["alice@example.com"]});
        redact_value(&mut value);
        let first = value["email"].as_str().unwrap().to_string();
        assert!(first.starts_with("redacted-"));
        assert_eq!(value["items"][0], first);
        assert!(value["note"].as_str().unwrap().contains(&first));
        assert!(!value.to_string().contains("bob@example.org"));
    }
    #[test]
    fn redactor_leaves_non_email_strings_alone() {
        assert_eq!(
            redact_emails_in_str("not-an-email @ local"),
            "not-an-email @ local"
        );
    }
    #[test]
    fn csv_escape_handles_quotes_commas_and_newlines() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b\nc"), "\"a\"\"b\nc\"");
    }
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
