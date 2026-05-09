//! Tool catalog endpoints lifted out of `routes_meta.rs`:
//! `GET /tools`, `GET /tools/<name>`, and `GET /tools/count`.
//! Cohesive sub-concern — every endpoint here builds the in-core
//! `ToolRegistry` from `builtin_tools(cwd)` and reads its
//! `JSON-schema` shape. Lifted out to keep `routes_meta.rs`
//! under the 500-LOC budget; `pub(crate) use` re-exports in the
//! parent leave call sites unchanged.
use argo_tool::{builtin_tools, ToolRegistry};

/// `GET /tools/<name>[?format=markdown]` — single tool by name.
/// Returns 200 + `{name, description, parameters}` or 404 when
/// no tool by that name is registered. With
/// `?format=markdown`, returns a `# <name>` doc with the
/// description + pretty-printed JSON-schema parameters block.
pub(super) async fn tool_show_response(
    raw_name: &str,
    format: Option<&str>,
) -> (&'static str, String) {
    let Ok(cwd) = std::env::current_dir() else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"cwd unavailable\"}".to_string(),
        );
    };
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd) {
        reg.register(t);
    }
    match reg.schemas().into_iter().find(|s| s.name == raw_name) {
        Some(s) => {
            if matches!(format, Some("markdown")) {
                let mut out = format!("# {}\n\n{}\n\n## Parameters\n\n", s.name, s.description);
                let pretty = serde_json::to_string_pretty(&s.parameters)
                    .unwrap_or_else(|_| "{}".to_string());
                out.push_str("```json\n");
                out.push_str(&pretty);
                out.push_str("\n```\n");
                return ("200 OK", out);
            }
            let payload = serde_json::json!({
                "name": s.name,
                "description": s.description,
                "parameters": s.parameters,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        None => (
            "404 Not Found",
            format!("{{\"error\":\"no tool named '{raw_name}'\"}}"),
        ),
    }
}

/// `GET /tools[?name-contains=<sub>&format=markdown]` — list
/// every registered in-core tool with its JSON-schema parameter
/// shape. Optional `name-contains` filter (case-insensitive)
/// lets clients find tools by pattern. With `format=markdown`,
/// the response is a markdown table — paste-able into
/// READMEs / runbooks.
pub(super) async fn tools_response(
    name_contains: Option<&str>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Ok(cwd) = std::env::current_dir() else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"cwd unavailable\"}".to_string(),
        );
    };
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd) {
        reg.register(t);
    }
    let needle = name_contains.map(str::to_lowercase);
    let mut schemas: Vec<argo_tool::ToolSchema> = reg
        .schemas()
        .into_iter()
        .filter(|s| match needle.as_deref() {
            Some(n) => s.name.to_lowercase().contains(n),
            None => true,
        })
        .collect();
    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    if matches!(format, Some("markdown")) {
        let mut out = format!("# Tools ({})\n\n", schemas.len());
        if !schemas.is_empty() {
            out.push_str("| name | description |\n");
            out.push_str("| --- | --- |\n");
            for s in &schemas {
                let desc = s
                    .description
                    .lines()
                    .next()
                    .unwrap_or("")
                    .replace('|', "\\|");
                out.push_str(&format!("| `{}` | {desc} |\n", s.name));
            }
        }
        return ("200 OK", out);
    }
    let rows: Vec<serde_json::Value> = schemas
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "description": s.description,
                "parameters": s.parameters,
            })
        })
        .collect();
    (
        "200 OK",
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
    )
}

/// `GET /tools/count[?name-contains=…&format=markdown]` —
/// count of registered in-core tools, optionally narrowed by
/// the same `name-contains` substring filter `tools_response`
/// uses. Returns `{count}` JSON; markdown returns a single
/// header line with optional filter bullet.
pub(super) async fn tools_count_response(
    name_contains: Option<&str>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Ok(cwd) = std::env::current_dir() else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"cwd unavailable\"}".to_string(),
        );
    };
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd) {
        reg.register(t);
    }
    let needle = name_contains.map(str::to_lowercase);
    let count = reg
        .schemas()
        .into_iter()
        .filter(|s| match needle.as_deref() {
            Some(n) => s.name.to_lowercase().contains(n),
            None => true,
        })
        .count();
    if matches!(format, Some("markdown")) {
        let mut out = format!("# Tools ({count})\n\n");
        if let Some(n) = name_contains {
            out.push_str(&format!("- **name-contains**: `{n}`\n"));
        }
        return ("200 OK", out);
    }
    (
        "200 OK",
        serde_json::to_string(&serde_json::json!({ "count": count }))
            .unwrap_or_else(|_| "{}".to_string()),
    )
}
