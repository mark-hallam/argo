//! Small helpers lifted out of `audit.rs` so it stays under the
//! 500-LOC budget once the rollup family kept growing. Pure utility
//! code — no I/O, no async — re-exported via `audit.rs` for callers
//! in sibling modules.

/// Parse the `--kinds <a,b,c>` flag into a `HashSet` for fast row
/// filtering. Returns `None` when the flag is absent (i.e. no filter).
/// Empty / whitespace-only entries are skipped.
pub(super) fn parse_kinds(spec: Option<&str>) -> Option<std::collections::HashSet<String>> {
    let raw = spec?;
    let set: std::collections::HashSet<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    Some(set)
}

/// Summary counts derived from a session's audit chain.
pub(super) struct SessionSummary {
    pub(super) turns: u32,
    pub(super) tool_calls: u32,
    pub(super) tool_errors: u32,
    pub(super) tokens_in: u64,
    pub(super) tokens_out: u64,
}

pub(super) fn summarize_session_rows(rows: &[argo_audit::ExportedRow]) -> SessionSummary {
    let mut s = SessionSummary {
        turns: 0,
        tool_calls: 0,
        tool_errors: 0,
        tokens_in: 0,
        tokens_out: 0,
    };
    for r in rows {
        match r.kind.as_str() {
            "TurnStart" => s.turns += 1,
            "ToolCall" => s.tool_calls += 1,
            "ToolResult"
                if r.payload.get("is_error").and_then(|v| v.as_bool()) == Some(true) =>
            {
                s.tool_errors += 1;
            }
            "AssistantMessage" => {
                if let Some(n) = r.payload.get("tokens_in").and_then(|v| v.as_u64()) {
                    s.tokens_in += n;
                }
                if let Some(n) = r.payload.get("tokens_out").and_then(|v| v.as_u64()) {
                    s.tokens_out += n;
                }
            }
            _ => {}
        }
    }
    s
}

/// One-line "interesting field" extract from a row's payload, keyed
/// by audit kind. Keeps `audit show --tail` readable without the
/// reader having to dig the payload column out of sqlite.
pub(super) fn payload_detail(kind: &str, payload: &serde_json::Value) -> String {
    let s = |k: &str| payload.get(k).and_then(|v| v.as_str()).unwrap_or("");
    let n = |k: &str| payload.get(k).and_then(serde_json::Value::as_u64);
    let b = |k: &str| payload.get(k).and_then(serde_json::Value::as_bool);
    match kind {
        "TurnStart" => format!("provider={} model={}", s("provider"), s("model")),
        "AssistantMessage" => format!(
            "round={} tool_calls={} in={} out={}",
            n("round").unwrap_or(0),
            n("tool_calls").unwrap_or(0),
            n("tokens_in").unwrap_or(0),
            n("tokens_out").unwrap_or(0),
        ),
        "ToolCall" => format!("tool={} call={}", s("tool"), s("call_id")),
        "ToolResult" => {
            let tag = if b("is_error") == Some(true) {
                " [error]"
            } else {
                ""
            };
            format!(
                "tool={} duration_ms={}{tag}",
                s("tool"),
                n("duration_ms").unwrap_or(0)
            )
        }
        "TurnEnd" | "Halt" => String::new(),
        _ => String::new(),
    }
}
