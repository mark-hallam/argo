use super::{canonical_payload_json, write_audit_csv_header, write_audit_csv_row, CSV_HEADER};
use crate::ExportedRow;
use serde_json::json;

fn sample_row() -> ExportedRow {
    ExportedRow {
        seq: 7,
        id: "row-7".into(),
        session_id: Some("session-1".into()),
        vertical_id: Some("receptionist".into()),
        kind: "tool_call".into(),
        payload: json!({
            "tool": "shell",
            "tenant_id": "ws-default",
            "worker_id": "worker-a",
            "email": "ops@example.com",
        }),
        parent_hash: "00".repeat(32),
        self_hash: "11".repeat(32),
        created_at_micros: 1_778_025_600_000_000,
    }
}

#[test]
fn header_matches_documented_columns() {
    let mut buf = Vec::new();
    write_audit_csv_header(&mut buf).unwrap();
    let header_line = String::from_utf8(buf).unwrap();
    let expected = format!("{}\r\n", CSV_HEADER.join(","));
    assert_eq!(header_line, expected);
    assert!(header_line.starts_with("seq,id,session_id,"));
    assert!(header_line.contains(",tenant_id,worker_id,timestamp\r\n"));
}

#[test]
fn row_canonicalizes_payload_and_extracts_pointers() {
    let mut buf = Vec::new();
    write_audit_csv_row(&sample_row(), &mut buf).unwrap();
    let line = String::from_utf8(buf).unwrap();
    let fields = line.trim_end_matches("\r\n").split(',').collect::<Vec<_>>();
    assert_eq!(fields[0], "7");
    assert_eq!(fields[1], "row-7");
    assert_eq!(fields[2], "session-1");
    assert_eq!(fields[3], "receptionist");
    assert_eq!(fields[4], "tool_call");
    assert_eq!(fields[5], "1778025600000000");
    // The payload column contains commas, so it's quoted — split-by-
    // comma corrupts it. Search for the canonical substring instead.
    assert!(
        line.contains(r#""{""email"":""ops@example.com"",""tenant_id"":""ws-default"",""tool"":""shell"",""worker_id"":""worker-a""}""#),
        "missing canonical payload column in: {line}"
    );
    // Trailing columns: tenant_id, worker_id, timestamp.
    assert!(line.ends_with("ws-default,worker-a,2026-05-06T00:00:00+00:00\r\n"));
}

#[test]
fn rfc4180_quotes_payload_with_commas_quotes_and_newlines() {
    let mut row = sample_row();
    row.payload = json!({
        "note": "a, \"b\", c\nnewline",
    });
    let mut buf = Vec::new();
    write_audit_csv_row(&row, &mut buf).unwrap();
    let line = String::from_utf8(buf).unwrap();
    // Doubled quotes inside the wrapped payload field.
    assert!(line.contains(r#"""note"":""a, \""b\"", c\nnewline"""#) || line.contains(r#""""note"":""a, """"b"""", c\nnewline"""#));
    // Round-trip via the csv crate is exercised in the handler-
    // level tests; here we just sanity-check the quoting rules.
    assert!(line.starts_with("7,row-7,session-1,"));
}

#[test]
fn canonical_payload_sorts_keys_recursively_and_omits_whitespace() {
    let value = json!({
        "z": 1,
        "a": {"y": 2, "x": [3, 1, 2]},
    });
    let out = canonical_payload_json(&value);
    assert_eq!(out, r#"{"a":{"x":[3,1,2],"y":2},"z":1}"#);
}

#[test]
fn streaming_writer_emits_header_then_rows_in_order() {
    let mut buf = Vec::new();
    write_audit_csv_header(&mut buf).unwrap();
    for seq in 1..=3 {
        let mut row = sample_row();
        row.seq = seq;
        row.id = format!("row-{seq}");
        write_audit_csv_row(&row, &mut buf).unwrap();
    }
    let body = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = body.split("\r\n").filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 4);
    assert!(lines[0].starts_with("seq,id,session_id,"));
    assert!(lines[1].starts_with("1,row-1,"));
    assert!(lines[2].starts_with("2,row-2,"));
    assert!(lines[3].starts_with("3,row-3,"));
}

#[test]
fn payload_with_no_tenant_or_worker_emits_empty_trailing_columns() {
    let mut row = sample_row();
    row.payload = json!({"tool": "shell"});
    let mut buf = Vec::new();
    write_audit_csv_row(&row, &mut buf).unwrap();
    let line = String::from_utf8(buf).unwrap();
    // ends with: ",,,<rfc3339>\r\n" — empty tenant + empty worker + timestamp.
    assert!(line.ends_with(",,2026-05-06T00:00:00+00:00\r\n"));
}
