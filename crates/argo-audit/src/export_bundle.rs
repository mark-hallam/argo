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
//! `audit.ndjson` + `audit.csv` + `chain_metadata.json` + `verify.sh`.
//! Operators can unpack an exported bundle and run `./verify.sh`
//! without `argo`. The sibling `audit.csv` mirrors the same row set
//! (one row per audit event, RFC 4180) so procurement reviewers can
//! load the evidence directly in a spreadsheet without re-running
//! `argo audit export --format csv` against the NDJSON stream
//! (`mission-audit-export-csv-format`).

use crate::csv::{write_audit_csv_header, write_audit_csv_row};
use crate::export::ExportedRow;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use flate2::{write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;

const ZERO_CHAIN_HEAD: &str = "0000000000000000000000000000000000000000000000000000000000000000";

const VERIFY_SH: &str = r#"#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

python3 - <<'PY'
import hashlib
import json
import sys
from pathlib import Path

ZERO = "0" * 64

def fail(message, code=1):
    print(f"verify.sh: {message}", file=sys.stderr)
    sys.exit(code)

def load_json(path):
    try:
        return json.loads(Path(path).read_text())
    except FileNotFoundError:
        fail(f"{path} missing", 2)
    except json.JSONDecodeError as exc:
        fail(f"{path} invalid JSON: {exc}", 2)

def hash_bytes(value, name, line_no):
    if not isinstance(value, str):
        fail(f"line {line_no}: missing {name}")
    try:
        decoded = bytes.fromhex(value)
    except ValueError:
        fail(f"line {line_no}: invalid {name} hex")
    if len(decoded) != 32:
        fail(f"line {line_no}: {name} must be 32 bytes")
    return decoded

metadata = load_json("chain_metadata.json")
expected_head = metadata.get("head_hash")
if not isinstance(expected_head, str) or len(expected_head) != 64:
    fail("chain_metadata.json missing head_hash", 2)

audit_path = Path("audit.ndjson")
try:
    audit_bytes = audit_path.read_bytes()
except FileNotFoundError:
    fail("audit.ndjson missing", 2)

expected_sha = metadata.get("audit_ndjson_sha256")
if isinstance(expected_sha, str) and expected_sha:
    actual_sha = hashlib.sha256(audit_bytes).hexdigest()
    if actual_sha != expected_sha:
        fail(f"audit.ndjson sha256 mismatch (actual={actual_sha} expected={expected_sha})")

parent = bytes.fromhex(ZERO)
row_count = 0
for raw in audit_bytes.splitlines():
    if not raw.strip():
        continue
    row_count += 1
    try:
        row = json.loads(raw)
    except json.JSONDecodeError as exc:
        fail(f"line {row_count}: invalid JSON: {exc}")
    row_parent = hash_bytes(row.get("parent_hash"), "parent_hash", row_count)
    if row_parent != parent:
        fail(f"line {row_count}: parent_hash does not match previous self_hash")
    kind = row.get("kind")
    if not isinstance(kind, str):
        fail(f"line {row_count}: missing kind")
    payload_json = json.dumps(
        row.get("payload"),
        separators=(",", ":"),
        sort_keys=True,
        ensure_ascii=False,
    )
    recomputed = hashlib.sha256(parent + kind.encode() + b"\0" + payload_json.encode()).digest()
    self_hash = hash_bytes(row.get("self_hash"), "self_hash", row_count)
    if recomputed != self_hash:
        fail(f"line {row_count}: self_hash does not match recomputed chain hash")
    parent = self_hash

expected_count = metadata.get("row_count")
if isinstance(expected_count, int) and expected_count != row_count:
    fail(f"row_count mismatch (actual={row_count} expected={expected_count})")

recomputed_head = parent.hex()
if recomputed_head != expected_head:
    fail(f"chain head mismatch (recomputed={recomputed_head} expected={expected_head})")

print(f"chain OK (head={expected_head})")
PY
"#;

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
    let csv = build_csv(rows)?;
    let head_hash = rows
        .last()
        .map_or(ZERO_CHAIN_HEAD, |row| row.self_hash.as_str());
    // `audit_csv_sha256` is procurement-grade chain-of-custody for the
    // sibling CSV: external reviewers can re-hash `audit.csv` and
    // confirm it matches the manifest without trusting the tar
    // archive's own integrity. verify.sh stays JSONL-only.
    let metadata = serde_json::to_vec_pretty(&serde_json::json!({
        "generator": "argo audit export",
        "generator_version": env!("CARGO_PKG_VERSION"),
        "row_count": rows.len(),
        "head_hash": head_hash,
        "audit_ndjson_sha256": sha256_hex(&ndjson),
        "audit_csv_sha256": sha256_hex(&csv),
    }))
    .map_err(|e| ArgoError::Schema(format!("metadata serialize: {e}")))?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar_b = tar::Builder::new(&mut gz);
        append_tar(&mut tar_b, "audit.ndjson", &ndjson)?;
        append_tar(&mut tar_b, "audit.csv", &csv)?;
        append_tar(&mut tar_b, "chain_metadata.json", &metadata)?;
        append_tar_mode(&mut tar_b, "verify.sh", VERIFY_SH.as_bytes(), 0o755)?;
        tar_b
            .finish()
            .map_err(|e| ArgoError::Provider(format!("tar finish: {e}")))?;
    }
    gz.finish()
        .map_err(|e| ArgoError::Provider(format!("gzip finish: {e}")))
}

fn append_tar<W: Write>(b: &mut tar::Builder<W>, name: &str, body: &[u8]) -> ArgoResult<()> {
    append_tar_mode(b, name, body, 0o644)
}

fn append_tar_mode<W: Write>(
    b: &mut tar::Builder<W>,
    name: &str,
    body: &[u8],
    mode: u32,
) -> ArgoResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(mode);
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
#[path = "export_bundle_tests.rs"]
mod tests;
