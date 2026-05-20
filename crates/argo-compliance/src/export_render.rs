use argo_audit::compute_chain_hash;
use argo_types::{ArgoError, ArgoResult};
use serde_json::{json, Value};
use std::io::Write;

pub(super) fn stamp_integrity(envelope: &mut Value) -> ArgoResult<()> {
    let canonical = serde_json::to_string(envelope)
        .map_err(|e| ArgoError::Provider(format!("compliance export integrity json: {e}")))?;
    let checksum = compute_chain_hash(&[0_u8; 32], "compliance.export.checksum", &canonical);
    let signature = compute_chain_hash(&checksum, "compliance.export.signature", &canonical);
    envelope["integrity"] = json!({
        "checksum": format!("sha256:{}", hex::encode(checksum)),
        "signature": format!("argo-chain:{}", hex::encode(signature)),
    });
    Ok(())
}

pub(super) fn write_envelope(envelope: &Value, out: Option<&str>, format: &str) -> ArgoResult<()> {
    let body = match format {
        "csv" => render_csv(envelope),
        "pdf" => render_pdf(envelope),
        _ => serde_json::to_string_pretty(envelope)
            .map_err(|e| ArgoError::Provider(format!("compliance export serialize: {e}")))?,
    };
    match out {
        Some(path) => {
            std::fs::write(path, &body).map_err(|e| ArgoError::Io(format!("write {path}: {e}")))
        }
        None => {
            let mut sink = std::io::stdout().lock();
            sink.write_all(body.as_bytes())
                .and_then(|()| sink.write_all(b"\n"))
                .and_then(|()| sink.flush())
                .map_err(|e| ArgoError::Provider(format!("compliance export stdout: {e}")))
        }
    }
}

pub(super) fn render_csv(envelope: &Value) -> String {
    let mut out = "table,row_index,row_json\n".to_string();
    if let Some(tables) = envelope.get("tables").and_then(Value::as_object) {
        for (table, entry) in tables {
            if let Some(rows) = entry.get("rows").and_then(Value::as_array) {
                for (idx, row) in rows.iter().enumerate() {
                    let row_json = serde_json::to_string(row).unwrap_or_default();
                    out.push_str(&format!(
                        "{},{},{}\n",
                        csv_cell(table),
                        idx,
                        csv_cell(&row_json)
                    ));
                }
            }
        }
    }
    let integrity = serde_json::to_string(&envelope["integrity"]).unwrap_or_default();
    out.push_str(&format!("integrity,0,{}\n", csv_cell(&integrity)));
    out
}

fn csv_cell(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

pub(super) fn render_pdf(envelope: &Value) -> String {
    let subject = envelope["subject"].as_str().unwrap_or("unknown");
    let checksum = envelope["integrity"]["checksum"].as_str().unwrap_or("");
    format!(
        "%PDF-1.4\n% argo compliance export\n1 0 obj <<>> endobj\n\
         2 0 obj << /Length 128 >> stream\nBT /F1 12 Tf 72 720 Td \
         (Argo compliance export) Tj 0 -18 Td (subject: {subject}) Tj \
         0 -18 Td (checksum: {checksum}) Tj ET\nendstream endobj\n%%EOF\n"
    )
}
