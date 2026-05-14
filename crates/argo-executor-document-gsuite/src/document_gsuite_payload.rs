use crate::types::{GsuiteRejection, GsuiteSheetRow, GsuiteWriteRequest};
use argo_executor_document_csv::{CsvColumnKind, CsvColumnSchema};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Build the `appendCells.rows` payload + collect per-row rejections.
/// One bad cell rejects the whole row, mirroring the CSV writer.
pub(crate) fn build_sheet_rows(
    schema: &[CsvColumnSchema],
    rows: &[GsuiteSheetRow],
) -> (Vec<Value>, Vec<GsuiteRejection>) {
    let mut out_rows: Vec<Value> = Vec::with_capacity(rows.len() + 1);
    let header_cells: Vec<Value> = schema
        .iter()
        .map(|c| json!({ "userEnteredValue": { "stringValue": c.header } }))
        .collect();
    out_rows.push(json!({ "values": header_cells }));

    let mut rejections: Vec<GsuiteRejection> = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let mut cells: Vec<Value> = Vec::with_capacity(schema.len());
        let mut row_ok = true;
        for col in schema {
            let val = row.cells.get(&col.header);
            match val {
                None | Some(Value::Null) => {
                    if col.required {
                        rejections.push(GsuiteRejection {
                            index: row_index as u32,
                            field: Some(col.header.clone()),
                            message: format!("column '{}' is required", col.header),
                        });
                        row_ok = false;
                        break;
                    }
                    cells.push(json!({ "userEnteredValue": { "stringValue": "" } }));
                }
                Some(v) => match coerce_cell(col, v) {
                    Ok(c) => cells.push(c),
                    Err(msg) => {
                        rejections.push(GsuiteRejection {
                            index: row_index as u32,
                            field: Some(col.header.clone()),
                            message: msg,
                        });
                        row_ok = false;
                        break;
                    }
                },
            }
        }
        if row_ok {
            out_rows.push(json!({ "values": cells }));
        }
    }
    (out_rows, rejections)
}

/// Per-kind cell coercion. Mirrors the CSV crate's coercion table,
/// emitting Sheets-flavoured `userEnteredValue` JSON instead of CSV.
fn coerce_cell(col: &CsvColumnSchema, v: &Value) -> Result<Value, String> {
    match col.kind {
        CsvColumnKind::Text => {
            let s = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            Ok(json!({ "userEnteredValue": { "stringValue": s } }))
        }
        CsvColumnKind::EmailAddress => match v {
            Value::String(s) if is_email_shape(s) => {
                Ok(json!({ "userEnteredValue": { "stringValue": s } }))
            }
            Value::String(s) => Err(format!("'{s}' is not in addr@domain.tld shape")),
            other => Err(format!("expected email string, got {other}")),
        },
        CsvColumnKind::Date => match v {
            Value::String(s) => {
                let dt = chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|e| format!("cannot parse '{s}' as RFC 3339 date: {e}"))?;
                Ok(json!({ "userEnteredValue": {
                    "stringValue": dt.format("%Y-%m-%d").to_string()
                }}))
            }
            other => Err(format!("expected RFC 3339 date string, got {other}")),
        },
        CsvColumnKind::Integer => match v {
            Value::Number(n) if n.is_i64() || n.is_u64() => Ok(json!({
                "userEnteredValue": { "numberValue": n.as_f64().unwrap_or(0.0) }
            })),
            Value::String(s) => {
                let parsed = s
                    .parse::<i64>()
                    .map_err(|_| format!("cannot parse '{s}' as integer"))?;
                Ok(json!({ "userEnteredValue": { "numberValue": parsed as f64 } }))
            }
            other => Err(format!("expected integer, got {other}")),
        },
        CsvColumnKind::Decimal => match v {
            Value::Number(n) => Ok(json!({
                "userEnteredValue": { "numberValue": n.as_f64().unwrap_or(0.0) }
            })),
            Value::String(s) => {
                let f = s
                    .parse::<f64>()
                    .map_err(|_| format!("cannot parse '{s}' as decimal"))?;
                Ok(json!({ "userEnteredValue": { "numberValue": f } }))
            }
            other => Err(format!("expected decimal, got {other}")),
        },
        CsvColumnKind::Boolean => match v {
            Value::Bool(b) => Ok(json!({ "userEnteredValue": { "boolValue": b } })),
            Value::String(s) => match s.as_str() {
                "true" | "1" => Ok(json!({ "userEnteredValue": { "boolValue": true } })),
                "false" | "0" => Ok(json!({ "userEnteredValue": { "boolValue": false } })),
                _ => Err(format!("cannot parse '{s}' as boolean")),
            },
            other => Err(format!("expected boolean, got {other}")),
        },
    }
}

fn is_email_shape(s: &str) -> bool {
    let mut parts = s.split('@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    parts.next().is_none() && !local.is_empty() && domain.contains('.') && !domain.starts_with('.')
}

pub(crate) fn collect_batch_rejections(resp: &Value) -> Vec<GsuiteRejection> {
    let Some(replies) = resp.get("replies").and_then(Value::as_array) else {
        return Vec::new();
    };
    replies
        .iter()
        .enumerate()
        .filter_map(|(i, reply)| {
            reply.get("error").map(|err| GsuiteRejection {
                index: i as u32,
                field: None,
                message: err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            })
        })
        .collect()
}

/// SHA-256 of the canonical-JSON serialisation of `req`. Deterministic
/// because the struct fields are declared in stable order.
pub(crate) fn canonical_request_sha256(req: &GsuiteWriteRequest) -> String {
    let serialised = serde_json::to_string(req).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(serialised.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
