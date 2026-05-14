//! Specialized table extraction built on the base document extractor.
//!
//! Providers still see the M1 JSON-schema extraction contract. This
//! module narrows that contract to tables, then normalizes common
//! provider shapes into row-major JSON for downstream skills.

use crate::extract::provider::{DocumentInput, ExtractionError, ExtractionProvider};
use crate::extract::schema::{ExtractionSchema, FieldSpec, JsonType};
use crate::extract::{extract_document, DocumentExtractionRequest, ExtractionAudit};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Request for document table extraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableExtractionRequest {
    /// Scanned image or native PDF bytes.
    pub document: DocumentInput,
    /// Optional column hints used when providers return array rows.
    pub expected_headers: Vec<String>,
    /// Optional table description, e.g. "invoice line items".
    pub table_hint: Option<String>,
}

/// Row-major table extracted from a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractedTable {
    /// Optional provider-supplied table name.
    pub title: Option<String>,
    /// Stable column order.
    pub headers: Vec<String>,
    /// Row-major JSON keyed by header.
    pub rows: Vec<BTreeMap<String, Value>>,
    /// First page containing this table.
    pub page_start: u32,
    /// Last page containing this table.
    pub page_end: u32,
    /// Provider confidence clamped to `[0.0, 1.0]`.
    pub confidence: f32,
}

/// Table extraction result plus M1 audit metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableExtractionOutcome {
    /// Normalized tables, with continuation chunks merged.
    pub tables: Vec<ExtractedTable>,
    /// Total normalized rows across all tables.
    pub row_count: usize,
    /// Audit metadata from the underlying M1 extraction.
    pub audit: ExtractionAudit,
}

/// Extract tables from a scanned or native document.
pub fn extract_tables<P: ExtractionProvider>(
    provider: &P,
    request: TableExtractionRequest,
) -> Result<TableExtractionOutcome, ExtractionError> {
    let schema = table_schema();
    let outcome = extract_document(
        provider,
        DocumentExtractionRequest {
            schema,
            document: request.document,
        },
    )?;
    let tables_value = outcome
        .fields
        .get("tables")
        .map(|field| field.value.clone())
        .unwrap_or(Value::Array(Vec::new()));
    let tables = normalize_tables(
        &tables_value,
        &request.expected_headers,
        outcome
            .fields
            .get("tables")
            .map(|field| field.confidence)
            .unwrap_or(0.82),
    )?;
    let row_count = tables.iter().map(|table| table.rows.len()).sum();
    Ok(TableExtractionOutcome {
        tables,
        row_count,
        audit: outcome.audit,
    })
}

fn table_schema() -> ExtractionSchema {
    ExtractionSchema {
        fields: BTreeMap::from([
            (
                "tables".into(),
                FieldSpec {
                    json_type: JsonType::Array,
                    required: true,
                },
            ),
            (
                "hint".into(),
                FieldSpec {
                    json_type: JsonType::String,
                    required: false,
                },
            ),
        ]),
    }
}

fn normalize_tables(
    value: &Value,
    expected_headers: &[String],
    default_confidence: f32,
) -> Result<Vec<ExtractedTable>, ExtractionError> {
    let chunks = value
        .as_array()
        .ok_or_else(|| ExtractionError::Protocol("tables field must be an array".into()))?;
    let mut tables: Vec<ExtractedTable> = Vec::new();
    for chunk in chunks {
        let mut table = normalize_table(chunk, expected_headers, default_confidence)?;
        if should_merge_continuation(tables.last(), chunk, &table) {
            if let Some(previous) = tables.last_mut() {
                previous.page_end = previous.page_end.max(table.page_end);
                previous.confidence = previous.confidence.min(table.confidence);
                previous.rows.append(&mut table.rows);
            }
        } else {
            tables.push(table);
        }
    }
    Ok(tables)
}

fn normalize_table(
    value: &Value,
    expected_headers: &[String],
    default_confidence: f32,
) -> Result<ExtractedTable, ExtractionError> {
    let object = value
        .as_object()
        .ok_or_else(|| ExtractionError::Protocol("table entry must be an object".into()))?;
    let rows_value = object
        .get("rows")
        .ok_or_else(|| ExtractionError::Protocol("table entry missing rows".into()))?;
    let rows_array = rows_value
        .as_array()
        .ok_or_else(|| ExtractionError::Protocol("table rows must be an array".into()))?;
    let headers = explicit_headers(object)
        .or_else(|| (!expected_headers.is_empty()).then(|| expected_headers.to_vec()))
        .unwrap_or_else(|| infer_headers(rows_array));
    let rows = rows_array
        .iter()
        .map(|row| normalize_row(row, &headers))
        .collect::<Result<Vec<_>, _>>()?;
    let page_start = object
        .get("page_start")
        .or_else(|| object.get("page"))
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;
    let page_end = object
        .get("page_end")
        .and_then(Value::as_u64)
        .unwrap_or(page_start as u64) as u32;
    Ok(ExtractedTable {
        title: object
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_owned),
        headers,
        rows,
        page_start,
        page_end,
        confidence: object
            .get("confidence")
            .and_then(Value::as_f64)
            .map(|value| value as f32)
            .unwrap_or(default_confidence)
            .clamp(0.0, 1.0),
    })
}

fn explicit_headers(object: &serde_json::Map<String, Value>) -> Option<Vec<String>> {
    let headers = object.get("headers")?.as_array()?;
    let parsed = headers
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|header| !header.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    (!parsed.is_empty()).then_some(parsed)
}

fn infer_headers(rows: &[Value]) -> Vec<String> {
    rows.iter()
        .find_map(Value::as_object)
        .map(|row| row.keys().cloned().collect())
        .unwrap_or_else(|| {
            let width = rows
                .iter()
                .filter_map(Value::as_array)
                .map(Vec::len)
                .max()
                .unwrap_or(0);
            (1..=width).map(|index| format!("column_{index}")).collect()
        })
}

fn normalize_row(
    row: &Value,
    headers: &[String],
) -> Result<BTreeMap<String, Value>, ExtractionError> {
    if let Some(object) = row.as_object() {
        return Ok(headers
            .iter()
            .map(|header| {
                (
                    header.clone(),
                    object.get(header).cloned().unwrap_or(Value::Null),
                )
            })
            .collect());
    }
    let cells = row
        .as_array()
        .ok_or_else(|| ExtractionError::Protocol("table row must be an object or array".into()))?;
    Ok(headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            (
                header.clone(),
                cells.get(index).cloned().unwrap_or(Value::Null),
            )
        })
        .collect())
}

fn should_merge_continuation(
    previous: Option<&ExtractedTable>,
    raw_chunk: &Value,
    table: &ExtractedTable,
) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    let marked_continuation = raw_chunk
        .get("continued_from_previous")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    marked_continuation
        || (previous.headers == table.headers
            && previous.title == table.title
            && table.page_start <= previous.page_end.saturating_add(1))
}

#[cfg(test)]
pub(crate) fn row_accuracy(
    actual: &[BTreeMap<String, Value>],
    expected: &[BTreeMap<String, Value>],
) -> f32 {
    let expected_cells = expected.iter().map(BTreeMap::len).sum::<usize>();
    if expected_cells == 0 {
        return 1.0;
    }
    let mut matches = 0usize;
    for (actual_row, expected_row) in actual.iter().zip(expected.iter()) {
        for (key, expected_value) in expected_row {
            if actual_row.get(key) == Some(expected_value) {
                matches += 1;
            }
        }
    }
    matches as f32 / expected_cells as f32
}
