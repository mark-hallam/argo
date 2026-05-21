//! Evidence-bundle assembly for `POST /api/v1/audit/export`
//! (`web-gateway-endpoints-audit-mutations-batch`).
//!
//! Wraps the existing per-row exporters (NDJSON, CSV, tar.gz) behind
//! one entry point that takes a `[from, to]` timestamp window plus a
//! format selector and returns the bundle bytes. Pure transform — no
//! file system or HTTP coupling; the gateway handler owns disk
//! persistence and the signed-URL flow.
//!
//! The tar.gz layout is identical to [`crate::archive`]:
//! `audit.ndjson` + `chain_metadata.json`. Operators can run
//! `tar -tzf` and `jq` against an exported bundle without `argo`.

use crate::csv::{write_audit_csv_header, write_audit_csv_row};
use crate::export::ExportedRow;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use flate2::{write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;

/// Wire selector for the bundle format. Lowercase variants are
/// accepted on the wire; [`BundleFormat::parse`] also accepts a few
/// common aliases (`ndjson`, `tar.gz`, `tgz`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BundleFormat {
    /// NDJSON — one [`ExportedRow`] per line.
    Json,
    /// RFC 4180 CSV with the [`crate::csv::CSV_HEADER`] columns.
    Csv,
    /// `tar.gz` bundle (`audit.ndjson` + `chain_metadata.json`).
    Tar,
}

impl BundleFormat {
    /// Parse a wire string. Returns `None` for unknown labels so the
    /// caller can produce a 400 with the allowed set.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "json" | "ndjson" => Some(Self::Json),
            "csv" => Some(Self::Csv),
            "tar" | "tar.gz" | "tgz" => Some(Self::Tar),
            _ => None,
        }
    }

    /// Filename extension (no leading dot).
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "ndjson",
            Self::Csv => "csv",
            Self::Tar => "tar.gz",
        }
    }

    /// Wire `Content-Type` header for a direct download.
    #[must_use]
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Json => "application/x-ndjson",
            Self::Csv => "text/csv",
            Self::Tar => "application/gzip",
        }
    }
}

/// `[from, to]` inclusive timestamp window. Either bound may be
/// `None` for unbounded.
#[derive(Debug, Clone, Default)]
pub struct ExportWindow {
    /// Inclusive lower bound on `audit_record.created_at`.
    pub from: Option<DateTime<Utc>>,
    /// Inclusive upper bound on `audit_record.created_at`.
    pub to: Option<DateTime<Utc>>,
}

impl ExportWindow {
    /// Returns `true` when `micros` falls inside the window.
    #[must_use]
    pub fn matches(&self, micros: i64) -> bool {
        let lo = self.from.map_or(i64::MIN, |t| t.timestamp_micros());
        let hi = self.to.map_or(i64::MAX, |t| t.timestamp_micros());
        micros >= lo && micros <= hi
    }
}

/// Preview shape for `POST /api/v1/audit/export/preview` — what
/// would land in the bundle without spending the I/O to build it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreview {
    /// Total rows that would be included.
    pub row_count: usize,
    /// Conservative byte estimate (NDJSON-sized; CSV/tar typically
    /// differ by ≤20%).
    pub byte_estimate: usize,
    /// First few rows from the window — useful for spot-checking
    /// before authorising a large export.
    pub sample: Vec<ExportedRow>,
}

/// Sample-size cap for [`fetch_preview`].
pub const PREVIEW_SAMPLE_CAP: usize = 5;

/// Fetch every [`ExportedRow`] in `window`, in `seq` order.
///
/// # Errors
/// Propagates [`ArgoError::Provider`] from the underlying NDJSON
/// scan.
pub async fn fetch_rows_for_export(
    store: &SessionStore,
    window: &ExportWindow,
) -> ArgoResult<Vec<ExportedRow>> {
    let mut out = Vec::new();
    crate::export::export_ndjson(store, |row| {
        if window.matches(row.created_at_micros) {
            out.push(row.clone());
        }
        Ok(())
    })
    .await?;
    Ok(out)
}

/// Cheap preview: count + sample without realising the full bundle
/// on disk. The byte estimate is the NDJSON serialization size,
/// which is the largest of the three formats today.
///
/// # Errors
/// Propagates [`ArgoError::Provider`] / [`ArgoError::Schema`] from
/// row fetch and per-row serialization.
pub async fn fetch_preview(
    store: &SessionStore,
    window: &ExportWindow,
) -> ArgoResult<ExportPreview> {
    let rows = fetch_rows_for_export(store, window).await?;
    let mut byte_estimate = 0_usize;
    for row in &rows {
        let line = serde_json::to_string(row)
            .map_err(|e| ArgoError::Schema(format!("preview serialize: {e}")))?;
        byte_estimate += line.len() + 1; // trailing newline
    }
    let sample = rows.iter().take(PREVIEW_SAMPLE_CAP).cloned().collect();
    Ok(ExportPreview {
        row_count: rows.len(),
        byte_estimate,
        sample,
    })
}

/// Build the bundle bytes for `rows` in the requested `format`.
///
/// # Errors
/// Propagates [`ArgoError::Provider`] / [`ArgoError::Schema`] on
/// serialization or gzip failure.
pub fn build_bundle(rows: &[ExportedRow], format: BundleFormat) -> ArgoResult<Vec<u8>> {
    match format {
        BundleFormat::Json => build_ndjson(rows),
        BundleFormat::Csv => build_csv(rows),
        BundleFormat::Tar => build_tarball(rows),
    }
}

fn build_ndjson(rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let mut buf = Vec::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|e| ArgoError::Schema(format!("ndjson serialize: {e}")))?;
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
    }
    Ok(buf)
}

fn build_csv(rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let mut buf = Vec::new();
    write_audit_csv_header(&mut buf)
        .map_err(|e| ArgoError::Provider(format!("csv header: {e}")))?;
    for row in rows {
        write_audit_csv_row(row, &mut buf)
            .map_err(|e| ArgoError::Provider(format!("csv row: {e}")))?;
    }
    Ok(buf)
}

fn build_tarball(rows: &[ExportedRow]) -> ArgoResult<Vec<u8>> {
    let ndjson = build_ndjson(rows)?;
    let metadata = serde_json::to_vec_pretty(&serde_json::json!({
        "generator": "argo audit export",
        "generator_version": env!("CARGO_PKG_VERSION"),
        "row_count": rows.len(),
        "audit_ndjson_sha256": sha256_hex(&ndjson),
    }))
    .map_err(|e| ArgoError::Schema(format!("metadata serialize: {e}")))?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar_b = tar::Builder::new(&mut gz);
        append_tar(&mut tar_b, "audit.ndjson", &ndjson)?;
        append_tar(&mut tar_b, "chain_metadata.json", &metadata)?;
        tar_b
            .finish()
            .map_err(|e| ArgoError::Provider(format!("tar finish: {e}")))?;
    }
    gz.finish()
        .map_err(|e| ArgoError::Provider(format!("gzip finish: {e}")))
}

fn append_tar<W: Write>(b: &mut tar::Builder<W>, name: &str, body: &[u8]) -> ArgoResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_cksum();
    b.append_data(&mut header, name, body)
        .map_err(|e| ArgoError::Provider(format!("tar append {name}: {e}")))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuditKind;
    use argo_store::SessionStore;
    use std::sync::Arc;

    async fn fresh_store() -> Arc<SessionStore> {
        let s = SessionStore::open(":memory:").await.unwrap();
        s.migrate().await.unwrap();
        Arc::new(s)
    }

    async fn write_n(store: &Arc<SessionStore>, n: usize) {
        #[allow(deprecated)]
        let sink = crate::AuditSink::new(Arc::clone(store));
        for i in 0..n {
            sink.write(
                None,
                AuditKind::TurnStart,
                serde_json::json!({"i": i}),
                None,
            )
            .await
            .unwrap();
        }
    }

    #[test]
    fn parse_format_handles_common_aliases() {
        assert_eq!(BundleFormat::parse("json"), Some(BundleFormat::Json));
        assert_eq!(BundleFormat::parse("NDJSON"), Some(BundleFormat::Json));
        assert_eq!(BundleFormat::parse("csv"), Some(BundleFormat::Csv));
        assert_eq!(BundleFormat::parse("TAR"), Some(BundleFormat::Tar));
        assert_eq!(BundleFormat::parse("tar.gz"), Some(BundleFormat::Tar));
        assert_eq!(BundleFormat::parse("tgz"), Some(BundleFormat::Tar));
        assert!(BundleFormat::parse("xml").is_none());
    }

    #[test]
    fn export_window_includes_endpoints() {
        let w = ExportWindow {
            from: DateTime::<Utc>::from_timestamp_micros(100),
            to: DateTime::<Utc>::from_timestamp_micros(200),
        };
        assert!(w.matches(100));
        assert!(w.matches(150));
        assert!(w.matches(200));
        assert!(!w.matches(99));
        assert!(!w.matches(201));
    }

    #[tokio::test]
    async fn fetch_and_build_round_trip_json() {
        let store = fresh_store().await;
        write_n(&store, 3).await;
        let window = ExportWindow::default();
        let rows = fetch_rows_for_export(&store, &window).await.unwrap();
        assert_eq!(rows.len(), 3);
        let bytes = build_bundle(&rows, BundleFormat::Json).unwrap();
        assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 3);
    }

    #[tokio::test]
    async fn fetch_and_build_round_trip_csv_has_header() {
        let store = fresh_store().await;
        write_n(&store, 2).await;
        let rows = fetch_rows_for_export(&store, &ExportWindow::default())
            .await
            .unwrap();
        let bytes = build_bundle(&rows, BundleFormat::Csv).unwrap();
        let s = String::from_utf8(bytes).unwrap();
        // RFC 4180 CRLF + first column label.
        assert!(s.starts_with("seq,"), "got: {s:?}");
        assert!(s.contains("\r\n"));
    }

    #[tokio::test]
    async fn fetch_and_build_round_trip_tar() {
        let store = fresh_store().await;
        write_n(&store, 4).await;
        let rows = fetch_rows_for_export(&store, &ExportWindow::default())
            .await
            .unwrap();
        let bytes = build_bundle(&rows, BundleFormat::Tar).unwrap();
        // gzip magic prefix
        assert!(bytes.len() > 10);
        assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
    }

    #[tokio::test]
    async fn preview_counts_and_samples_match_full_export() {
        let store = fresh_store().await;
        write_n(&store, 10).await;
        let p = fetch_preview(&store, &ExportWindow::default())
            .await
            .unwrap();
        assert_eq!(p.row_count, 10);
        assert_eq!(p.sample.len(), PREVIEW_SAMPLE_CAP);
        assert!(p.byte_estimate > 0);
    }

    #[tokio::test]
    async fn preview_window_filter_excludes_outside_rows() {
        let store = fresh_store().await;
        write_n(&store, 5).await;
        let far_future = DateTime::<Utc>::from_timestamp_micros(
            chrono::Utc::now().timestamp_micros() + 60_000_000_000,
        );
        let window = ExportWindow {
            from: far_future,
            to: None,
        };
        let p = fetch_preview(&store, &window).await.unwrap();
        assert_eq!(p.row_count, 0);
        assert_eq!(p.byte_estimate, 0);
        assert!(p.sample.is_empty());
    }
}
