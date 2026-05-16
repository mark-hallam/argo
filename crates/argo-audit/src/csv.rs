//! Streaming CSV export of audit rows (`mission-audit-csv-export`).
//!
//! Pure formatting helpers that the CLI's `argo audit export
//! --format csv` handler uses to stream rows to a writer without
//! buffering the whole result set in memory. Each row's columns
//! match the SIEM-ingestion contract: a stable prefix used by the
//! pre-existing `argo audit export` (#855) plus three trailing
//! columns asked for by `mission-audit-csv-export` (tenant id,
//! worker id, ISO-8601 timestamp).
//!
//! Quoting follows RFC 4180:
//!   * CRLF line terminator.
//!   * Fields containing `,`, `"`, `CR`, or `LF` are wrapped in `"`.
//!   * Embedded `"` is doubled.
//!
//! The `payload_json` column carries the *canonical* form (sorted
//! keys, no whitespace) — the same canonical form fed into the
//! chain-hash so operators can recompute hashes from the export.
use crate::ExportedRow;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::io::{self, Write};

/// Column header for the CSV export. The prefix is stable for
/// backwards compat with #855; trailing columns were added by
/// `mission-audit-csv-export` so SIEMs can filter by tenant / worker
/// / human-readable timestamp without parsing the payload.
pub const CSV_HEADER: &[&str] = &[
    "seq",
    "id",
    "session_id",
    "vertical_id",
    "kind",
    "created_at_micros",
    "payload_json",
    "parent_hash",
    "self_hash",
    "tenant_id",
    "worker_id",
    "timestamp",
];

/// Pointers tried in order to find `tenant_id` in a payload.
const TENANT_POINTERS: &[&str] = &["/tenant_id", "/tenant", "/tenant/id"];

/// Pointers tried in order to find `worker_id` in a payload.
const WORKER_POINTERS: &[&str] =
    &["/worker_id", "/from_worker_id", "/to/worker_id", "/worker/id"];

/// Write the CSV header row.
///
/// # Errors
/// Propagates [`io::Error`] from `writer`.
pub fn write_audit_csv_header<W: Write>(writer: &mut W) -> io::Result<()> {
    write_record(writer, CSV_HEADER.iter().copied())
}

/// Write one CSV row for `row`.
///
/// # Errors
/// Propagates [`io::Error`] from `writer`.
pub fn write_audit_csv_row<W: Write>(row: &ExportedRow, writer: &mut W) -> io::Result<()> {
    let seq = row.seq.to_string();
    let session_id = row.session_id.as_deref().unwrap_or_default();
    let vertical_id = row.vertical_id.as_deref().unwrap_or_default();
    let created_at_micros = row.created_at_micros.to_string();
    let payload_json = canonical_payload_json(&row.payload);
    let tenant_id = payload_pointer_string(&row.payload, TENANT_POINTERS).unwrap_or_default();
    let worker_id = payload_pointer_string(&row.payload, WORKER_POINTERS).unwrap_or_default();
    let timestamp = micros_to_rfc3339(row.created_at_micros);

    let fields: [&str; 12] = [
        &seq,
        &row.id,
        session_id,
        vertical_id,
        &row.kind,
        &created_at_micros,
        &payload_json,
        &row.parent_hash,
        &row.self_hash,
        &tenant_id,
        &worker_id,
        &timestamp,
    ];
    write_record(writer, fields.iter().copied())
}

/// Canonical-form JSON used by both the chain hash and the CSV
/// `payload_json` column. Object keys are recursively sorted; arrays
/// preserve order; serialization carries no whitespace. Matches the
/// shape passed to `compute_chain_hash` in `sink.rs`.
#[must_use]
pub fn canonical_payload_json(value: &Value) -> String {
    let canonical = canonicalize(value);
    serde_json::to_string(&canonical).unwrap_or_else(|_| "null".to_string())
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> =
                map.iter().map(|(k, v)| (k.clone(), canonicalize(v))).collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let sorted: serde_json::Map<String, Value> = entries.into_iter().collect();
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

fn payload_pointer_string(payload: &Value, pointers: &[&str]) -> Option<String> {
    for ptr in pointers {
        if let Some(v) = payload.pointer(ptr).and_then(Value::as_str) {
            return Some(v.to_string());
        }
    }
    None
}

fn micros_to_rfc3339(micros: i64) -> String {
    let secs = micros.div_euclid(1_000_000);
    let nsec = u32::try_from(micros.rem_euclid(1_000_000)).unwrap_or(0) * 1_000;
    DateTime::<Utc>::from_timestamp(secs, nsec)
        .map_or_else(String::new, |dt| dt.to_rfc3339())
}

fn write_record<'a, I>(writer: &mut impl Write, fields: I) -> io::Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut first = true;
    for field in fields {
        if !first {
            writer.write_all(b",")?;
        }
        write_field(writer, field)?;
        first = false;
    }
    writer.write_all(b"\r\n")
}

fn write_field<W: Write>(writer: &mut W, field: &str) -> io::Result<()> {
    let needs_quote = field.chars().any(|c| matches!(c, ',' | '"' | '\r' | '\n'));
    if !needs_quote {
        return writer.write_all(field.as_bytes());
    }
    writer.write_all(b"\"")?;
    for ch in field.chars() {
        if ch == '"' {
            writer.write_all(b"\"\"")?;
        } else {
            let mut buf = [0u8; 4];
            writer.write_all(ch.encode_utf8(&mut buf).as_bytes())?;
        }
    }
    writer.write_all(b"\"")
}

#[cfg(test)]
#[path = "csv_tests.rs"]
mod csv_tests;
