//! Compliance-oriented `argo audit export`.
//!
//! Orchestration only — input parsing/normalization lives in
//! `audit_export_args`, GDPR redaction in `audit_export_redact`,
//! CSV rendering in `argo_audit::csv`. This file owns the runtime
//! flow (chain verification, row collection, format dispatch,
//! output sink) and the JSON-family format encoders.
//!
//! Streaming: the `csv`, `jsonl`, `json`, and `ndjson` formats write
//! rows to a `BufWriter` through a sha256+byte-counting wrapper as
//! `export_ndjson` produces them — no in-memory `Vec<u8>` of
//! formatted output (`mission-audit-csv-export`). The `tarball`
//! format remains buffered because the manifest commits to the
//! NDJSON sha256 of its sibling member.
use super::audit_export_args::{ExportFormat, NormalizedExport};
use super::audit_export_redact::redact_value;
use argo_audit::{
    export_ndjson, verify_all, write_audit_csv_header, write_audit_csv_row, ExportedRow,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::io::{self, BufWriter, Write};

pub use super::audit_export_args::ExportArgs;

pub async fn run(store: &SessionStore, args: ExportArgs) -> ArgoResult<()> {
    let query = NormalizedExport::new(args)?;
    ensure_chain_ok(store).await?;
    match query.format {
        ExportFormat::Tarball => run_buffered_tarball(store, &query).await,
        _ => run_streaming(store, &query).await,
    }
}

async fn run_streaming(store: &SessionStore, query: &NormalizedExport) -> ArgoResult<()> {
    let sink = open_sink(query.bundle_out.as_deref())?;
    let counter = RefCell::new(WriteAccount::default());
    let mut writer = HashingWriter::new(sink, &counter);
    write_prologue(&mut writer, query.format)?;

    let mut first = true;
    let mut row_count: usize = 0;
    export_ndjson(store, |row| {
        if !query.matches(row) {
            return Ok(());
        }
        let mut row = row.clone();
        if query.gdpr_redact_emails {
            redact_value(&mut row.payload);
        }
        emit_row(&mut writer, query.format, &row, &mut first)
            .map_err(|e| ArgoError::Io(format!("write audit row: {e}")))?;
        row_count += 1;
        Ok(())
    })
    .await?;

    write_epilogue(&mut writer, query.format)?;
    writer
        .flush()
        .map_err(|e| ArgoError::Io(format!("flush export: {e}")))?;
    drop(writer);
    let summary = counter.into_inner().finalize();
    emit_envelope_with_rows(query, &summary, Some(row_count))?;
    if query.bundle_out.is_some() && !query.json {
        eprintln!("audit export: wrote {row_count} rows");
    }
    Ok(())
}

/// One I/O sink (file or stdout) wrapped in a `BufWriter`. The two
/// arms keep the `dyn` boundary so callers don't fork the streaming
/// loop on output destination.
enum Sink {
    File(BufWriter<std::fs::File>),
    Stdout(BufWriter<io::Stdout>),
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::File(w) => w.write(buf),
            Self::Stdout(w) => w.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::File(w) => w.flush(),
            Self::Stdout(w) => w.flush(),
        }
    }
}

fn open_sink(path: Option<&str>) -> ArgoResult<Sink> {
    match path {
        Some(p) => {
            let f = std::fs::File::create(p)
                .map_err(|e| ArgoError::Io(format!("open {p}: {e}")))?;
            Ok(Sink::File(BufWriter::new(f)))
        }
        None => Ok(Sink::Stdout(BufWriter::new(io::stdout()))),
    }
}

/// Running totals captured by `HashingWriter` so the JSON envelope
/// can report byte count + sha256 without re-reading the artifact.
#[derive(Default)]
struct WriteAccount {
    bytes: usize,
    hasher: Sha256,
}

impl WriteAccount {
    fn finalize(self) -> ArtifactSummary {
        ArtifactSummary {
            bytes: self.bytes,
            sha256: hex::encode(self.hasher.finalize()),
        }
    }
}

struct ArtifactSummary {
    bytes: usize,
    sha256: String,
}

/// Writer adapter that tees bytes to an inner writer while
/// accumulating sha256 + byte count in a borrowed `WriteAccount`.
struct HashingWriter<'a, W: Write> {
    inner: W,
    acct: &'a RefCell<WriteAccount>,
}

impl<'a, W: Write> HashingWriter<'a, W> {
    fn new(inner: W, acct: &'a RefCell<WriteAccount>) -> Self {
        Self { inner, acct }
    }
}

impl<W: Write> Write for HashingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        let mut acct = self.acct.borrow_mut();
        acct.bytes += n;
        acct.hasher.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn write_prologue<W: Write>(writer: &mut W, format: ExportFormat) -> ArgoResult<()> {
    match format {
        ExportFormat::Csv => write_audit_csv_header(writer)
            .map_err(|e| ArgoError::Io(format!("write csv header: {e}"))),
        ExportFormat::Json => writer
            .write_all(b"[")
            .map_err(|e| ArgoError::Io(format!("write json prologue: {e}"))),
        ExportFormat::Ndjson | ExportFormat::Jsonl | ExportFormat::Tarball => Ok(()),
    }
}

fn write_epilogue<W: Write>(writer: &mut W, format: ExportFormat) -> ArgoResult<()> {
    match format {
        ExportFormat::Json => writer
            .write_all(b"]")
            .map_err(|e| ArgoError::Io(format!("write json epilogue: {e}"))),
        _ => Ok(()),
    }
}

fn emit_row<W: Write>(
    writer: &mut W,
    format: ExportFormat,
    row: &ExportedRow,
    first: &mut bool,
) -> io::Result<()> {
    match format {
        ExportFormat::Csv => write_audit_csv_row(row, writer),
        ExportFormat::Ndjson | ExportFormat::Jsonl => {
            let line = serde_json::to_vec(row)
                .map_err(|e| io::Error::other(format!("serialize row: {e}")))?;
            writer.write_all(&line)?;
            writer.write_all(b"\n")
        }
        ExportFormat::Json => {
            if !*first {
                writer.write_all(b",")?;
            }
            *first = false;
            let chunk = serde_json::to_vec(row)
                .map_err(|e| io::Error::other(format!("serialize row: {e}")))?;
            writer.write_all(&chunk)
        }
        ExportFormat::Tarball => unreachable!("tarball goes through buffered path"),
    }
}

async fn run_buffered_tarball(store: &SessionStore, query: &NormalizedExport) -> ArgoResult<()> {
    let mut rows = collect_rows(store, query).await?;
    if query.gdpr_redact_emails {
        for row in &mut rows {
            redact_value(&mut row.payload);
        }
    }
    let artifact = render_tarball(query, &rows)?;
    let sha = sha256_hex(&artifact);
    let bytes = artifact.len();
    if let Some(path) = query.bundle_out.as_deref() {
        std::fs::write(path, &artifact)
            .map_err(|e| ArgoError::Io(format!("write {path}: {e}")))?;
    } else if !query.json {
        let mut out = io::stdout().lock();
        out.write_all(&artifact)
            .and_then(|()| out.flush())
            .map_err(|e| ArgoError::Io(format!("write stdout: {e}")))?;
    }
    let summary = ArtifactSummary { bytes, sha256: sha };
    if query.json {
        emit_envelope_with_rows(query, &summary, Some(rows.len()))
    } else {
        if query.bundle_out.is_some() {
            eprintln!("audit export: wrote {} rows", rows.len());
        }
        Ok(())
    }
}

fn emit_envelope_with_rows(
    query: &NormalizedExport,
    summary: &ArtifactSummary,
    rows: Option<usize>,
) -> ArgoResult<()> {
    if !query.json {
        return Ok(());
    }
    let line = serde_json::to_string(&serde_json::json!({
        "export": {
            "ok": true,
            "format": query.format,
            "rows": rows,
            "bytes": summary.bytes,
            "sha256": summary.sha256,
            "destination": query.bundle_out.as_deref(),
            "since": query.since.as_deref(),
            "until": query.until.as_deref(),
            "worker_id": query.worker_id.as_deref(),
            "tenant": query.tenant.as_deref(),
            "session": query.session.as_deref(),
            "kinds": if query.kinds.is_empty() { None } else { Some(&query.kinds) },
            "gdpr_redact_emails": query.gdpr_redact_emails,
        }
    }))
    .map_err(|e| ArgoError::Provider(format!("serialize export summary: {e}")))?;
    println!("{line}");
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

fn render_tarball(query: &NormalizedExport, rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let ndjson = render_ndjson(rows)?;
    let ndjson_sha = sha256_hex(&ndjson);
    let csv = render_csv(rows)?;
    let csv_sha = sha256_hex(&csv);
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
        "audit_csv_sha256": csv_sha,
    }))
    .map_err(|e| ArgoError::Provider(format!("serialize manifest: {e}")))?;
    let readme = README_TXT.as_bytes();
    let mut out = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut out);
        append_tar_bytes(&mut builder, "audit.ndjson", &ndjson)?;
        append_tar_bytes(&mut builder, "audit.csv", &csv)?;
        append_tar_bytes(&mut builder, "manifest.json", &manifest)?;
        append_tar_bytes(&mut builder, "README.txt", readme)?;
        builder
            .finish()
            .map_err(|e| ArgoError::Provider(format!("finish tarball: {e}")))?;
    }
    Ok(out)
}

fn render_csv(rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let mut out = Vec::new();
    write_audit_csv_header(&mut out)
        .map_err(|e| ArgoError::Io(format!("csv header: {e}")))?;
    for row in rows {
        write_audit_csv_row(row, &mut out)
            .map_err(|e| ArgoError::Io(format!("csv row: {e}")))?;
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
audit.ndjson contains records in seq order. audit.csv mirrors the same rows in RFC 4180 CSV (procurement / spreadsheet-friendly). manifest.json records query parameters, generator version, and the sha256 of both audit.ndjson and audit.csv. Email redaction is deterministic when enabled.\n";

#[cfg(test)]
#[path = "audit_export_tests.rs"]
mod tests;
