//! RFC 4180-conforming CSV writer + the `DocumentCsv` entry point.
//!
//! Per-row coercion table (driven by `CsvColumnSchema.kind`):
//!   * `Text` ← any JSON value → `to_string()` (`null` → `""`).
//!   * `Integer` ← `Number(i)` as-is; `String(s)` parsed as `i64` or
//!     row rejected.
//!   * `Decimal` ← `Number(f)` via `{}`; `String(s)` verbatim.
//!   * `Date` ← `String(s)` parsed as RFC 3339, rendered as
//!     `YYYY-MM-DD`; anything else rejects the row.
//!   * `Boolean` ← `Bool(b)` ⇒ "true"/"false";
//!     `String("true"|"false"|"1"|"0")` coerces.
//!   * `EmailAddress` ← `String(s)` matching `^[^@]+@[^@]+\.[^@]+$`.
//!
//! Rejection is record-level — a bad row never aborts the file. The
//! returned [`CsvWriteReport`] carries one [`RowRejection`] per
//! offending row.

use crate::sandbox::{resolve_sandboxed_path, CsvSandboxError};
use crate::types::{
    CsvColumnKind, CsvColumnSchema, LineTerminator, RejectionReason, RowRejection,
};
use argo_types::{ArgoError, ArgoResult};
use csv::{QuoteStyle, Terminator, WriterBuilder};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

/// Caller-supplied request for a single CSV write op.
#[derive(Debug, Clone)]
pub struct CsvWriteRequest {
    /// Operator-configured output root (typically pulled from
    /// `[document_csv].output_root`). `path` must resolve beneath
    /// this root after canonicalisation.
    pub output_root: PathBuf,
    /// Target file path. May be relative (joined under
    /// `output_root`) or absolute (canonical form must start with
    /// `output_root`).
    pub path: PathBuf,
    /// Column schema. Order is preserved in the output file.
    pub schema: Vec<CsvColumnSchema>,
    /// Row bodies keyed by `CsvColumnSchema.header`.
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    /// Emit a header row before the data rows.
    pub include_header: bool,
    /// Line terminator between records.
    pub line_terminator: LineTerminator,
    /// `true` ⇒ quote only when needed (RFC 4180 default);
    /// `false` ⇒ quote-all (helpful for downstream tools that
    /// mis-handle minimal quoting).
    pub quote_minimal: bool,
    /// Prepend a UTF-8 BOM (`EF BB BF`) to the output. Default off;
    /// when on, the BOM is hashed alongside the rest of the body.
    pub write_bom: bool,
}

/// Structured outcome from a write op. Surfaced to the worker action
/// bundle so the operator + audit ledger can verify the artifact
/// downstream.
#[derive(Debug, Clone)]
pub struct CsvWriteReport {
    /// Absolute, sandbox-resolved path that was written.
    pub path: PathBuf,
    /// Total bytes written (post-BOM, post-quoting). Matches the
    /// on-disk file size.
    pub bytes_written: u64,
    /// Count of rows that survived coercion + validation.
    pub rows_written: u32,
    /// Count of rows in the request that were rejected.
    pub rows_rejected: u32,
    /// Per-rejection detail.
    pub rejections: Vec<RowRejection>,
    /// SHA-256 of the bytes written, in hex. Deterministic for the
    /// same logical input.
    pub sha256: String,
}

/// Stateless dispatch handle. Holds nothing — construct once and
/// reuse per write.
#[derive(Debug, Default, Clone, Copy)]
pub struct DocumentCsv;

impl DocumentCsv {
    /// Constructor.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Render `req` to disk and return the report. See the module
    /// docs for the coercion table.
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] on sandbox violations and
    /// [`ArgoError::Io`] on filesystem failure. Per-row coercion
    /// failures are NOT errors — they appear in the report's
    /// `rejections` list.
    pub fn write(&self, req: CsvWriteRequest) -> ArgoResult<CsvWriteReport> {
        let target = resolve_sandboxed_path(&req.output_root, &req.path)
            .map_err(<CsvSandboxError as Into<ArgoError>>::into)?;

        let mut body: Vec<u8> = Vec::new();
        if req.write_bom {
            body.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }

        let terminator = match req.line_terminator {
            LineTerminator::Crlf => Terminator::CRLF,
            LineTerminator::Lf => Terminator::Any(b'\n'),
        };
        let quote_style = if req.quote_minimal {
            QuoteStyle::Necessary
        } else {
            QuoteStyle::Always
        };

        let (rows_written, rows_rejected, rejections) = {
            let mut writer = WriterBuilder::new()
                .terminator(terminator)
                .quote_style(quote_style)
                .has_headers(false)
                .from_writer(&mut body);

            if req.include_header {
                let header: Vec<&str> =
                    req.schema.iter().map(|c| c.header.as_str()).collect();
                writer
                    .write_record(&header)
                    .map_err(|e| ArgoError::Io(format!("write header: {e}")))?;
            }
            let mut written: u32 = 0;
            let mut rejected: u32 = 0;
            let mut rejections: Vec<RowRejection> = Vec::new();
            for (row_index, row) in req.rows.iter().enumerate() {
                match coerce_row(&req.schema, row, row_index as u32) {
                    Ok(record) => {
                        writer
                            .write_record(&record)
                            .map_err(|e| ArgoError::Io(format!("write row {row_index}: {e}")))?;
                        written += 1;
                    }
                    Err(r) => {
                        rejected += 1;
                        rejections.push(r);
                    }
                }
            }
            writer
                .flush()
                .map_err(|e| ArgoError::Io(format!("flush csv writer: {e}")))?;
            (written, rejected, rejections)
        };

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, &body)?;

        let mut hasher = Sha256::new();
        hasher.update(&body);
        let digest = hasher.finalize();
        let sha256_hex = digest.iter().map(|b| format!("{b:02x}")).collect::<String>();

        Ok(CsvWriteReport {
            path: target,
            bytes_written: body.len() as u64,
            rows_written,
            rows_rejected,
            rejections,
            sha256: sha256_hex,
        })
    }
}

/// Coerce one row into the column-ordered string vector that
/// `csv::Writer::write_record` expects, or build a typed rejection.
fn coerce_row(
    schema: &[CsvColumnSchema],
    row: &HashMap<String, serde_json::Value>,
    row_index: u32,
) -> Result<Vec<String>, RowRejection> {
    let mut out: Vec<String> = Vec::with_capacity(schema.len());
    for col in schema {
        let value = row.get(&col.header);
        let rendered = match value {
            None | Some(serde_json::Value::Null) => {
                if col.required {
                    return Err(RowRejection {
                        row_index,
                        column: Some(col.header.clone()),
                        reason: RejectionReason::MissingRequired,
                        message: format!("column '{}' is required", col.header),
                    });
                }
                String::new()
            }
            Some(v) => coerce_value(col, v).map_err(|reason_msg| RowRejection {
                row_index,
                column: Some(col.header.clone()),
                reason: RejectionReason::TypeMismatch,
                message: reason_msg,
            })?,
        };
        if let Some(cap) = col.max_len {
            if rendered.len() > cap as usize {
                return Err(RowRejection {
                    row_index,
                    column: Some(col.header.clone()),
                    reason: RejectionReason::MaxLenExceeded,
                    message: format!(
                        "column '{}' value of {} bytes exceeds max_len {cap}",
                        col.header,
                        rendered.len()
                    ),
                });
            }
        }
        out.push(rendered);
    }
    Ok(out)
}

/// Per-kind coercion from `serde_json::Value` to the on-wire string.
fn coerce_value(col: &CsvColumnSchema, v: &serde_json::Value) -> Result<String, String> {
    match col.kind {
        CsvColumnKind::Text => Ok(match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }),
        CsvColumnKind::Integer => match v {
            serde_json::Value::Number(n) if n.is_i64() || n.is_u64() => Ok(n.to_string()),
            serde_json::Value::String(s) => s
                .parse::<i64>()
                .map(|n| n.to_string())
                .map_err(|_| format!("cannot parse '{s}' as integer")),
            other => Err(format!("expected integer, got {other}")),
        },
        CsvColumnKind::Decimal => match v {
            serde_json::Value::Number(n) => Ok(n.to_string()),
            serde_json::Value::String(s) => {
                // Validate parseability but emit the verbatim string
                // so the caller controls trailing-zero presentation.
                s.parse::<f64>()
                    .map_err(|_| format!("cannot parse '{s}' as decimal"))?;
                Ok(s.clone())
            }
            other => Err(format!("expected decimal, got {other}")),
        },
        CsvColumnKind::Date => match v {
            serde_json::Value::String(s) => {
                let dt = chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|e| format!("cannot parse '{s}' as RFC 3339 date: {e}"))?;
                Ok(dt.format("%Y-%m-%d").to_string())
            }
            other => Err(format!("expected RFC 3339 date string, got {other}")),
        },
        CsvColumnKind::Boolean => match v {
            serde_json::Value::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
            serde_json::Value::String(s) => match s.as_str() {
                "true" | "1" => Ok("true".into()),
                "false" | "0" => Ok("false".into()),
                _ => Err(format!("cannot parse '{s}' as boolean")),
            },
            other => Err(format!("expected boolean, got {other}")),
        },
        CsvColumnKind::EmailAddress => match v {
            serde_json::Value::String(s) => {
                if is_email_shape(s) {
                    Ok(s.clone())
                } else {
                    Err(format!("'{s}' is not in addr@domain.tld shape"))
                }
            }
            other => Err(format!("expected email string, got {other}")),
        },
    }
}

/// Shape check for `^[^@]+@[^@]+\.[^@]+$` without a regex dep.
fn is_email_shape(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    let mut split = domain.splitn(2, '.');
    let head = split.next().unwrap_or("");
    let tail = split.next().unwrap_or("");
    !head.is_empty() && !tail.is_empty()
}
