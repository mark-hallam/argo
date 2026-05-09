//! Pure helpers for the ClickUp HTTP client: response decoding, query
//! shaping, body shaping, and argument extraction.
use serde_json::{Map, Value};

pub(crate) const CREATE_TASK_FIELDS: &[&str] = &[
    "name",
    "description",
    "markdown_content",
    "assignees",
    "archived",
    "group_assignees",
    "tags",
    "status",
    "priority",
    "due_date",
    "due_date_time",
    "time_estimate",
    "start_date",
    "start_date_time",
    "points",
    "notify_all",
    "parent",
    "links_to",
    "check_required_custom_fields",
    "custom_fields",
    "custom_item_id",
];

pub(crate) const UPDATE_TASK_FIELDS: &[&str] = &[
    "name",
    "description",
    "markdown_content",
    "status",
    "priority",
    "due_date",
    "due_date_time",
    "time_estimate",
    "start_date",
    "start_date_time",
    "points",
    "assignees",
    "group_assignees",
    "watchers",
    "archived",
    "custom_item_id",
];

pub(crate) async fn decode_response(
    method: &str,
    path: &str,
    resp: reqwest::Response,
) -> Result<Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("clickup {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("clickup {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("clickup {method} {path}: HTTP {status}: {parsed}"))
    }
}

pub(crate) fn task_list_query(args: &Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    for field in [
        "archived",
        "include_markdown_description",
        "reverse",
        "subtasks",
        "include_closed",
        "include_timl",
    ] {
        push_bool(args, &mut query, field);
    }
    for field in [
        "page",
        "due_date_gt",
        "due_date_lt",
        "date_created_gt",
        "date_created_lt",
    ] {
        push_u64(args, &mut query, field);
    }
    for field in [
        "date_updated_gt",
        "date_updated_lt",
        "date_done_gt",
        "date_done_lt",
    ] {
        push_u64(args, &mut query, field);
    }
    push_str(args, &mut query, "order_by");
    push_array(args, &mut query, "statuses", "statuses[]");
    push_array(args, &mut query, "assignees", "assignees[]");
    push_array(args, &mut query, "watchers", "watchers[]");
    push_array(args, &mut query, "tags", "tags[]");
    push_jsonish(args, &mut query, "custom_fields");
    query
}

pub(crate) fn task_query(args: &Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    push_bool(args, &mut query, "custom_task_ids");
    push_str(args, &mut query, "team_id");
    push_bool(args, &mut query, "include_subtasks");
    push_bool(args, &mut query, "include_markdown_description");
    push_jsonish(args, &mut query, "custom_fields");
    query
}

pub(crate) fn copy_body(args: &Value, fields: &[&str]) -> Map<String, Value> {
    let mut body = Map::new();
    for field in fields {
        copy_value(args, &mut body, field);
    }
    body
}

pub(crate) fn copy_value(args: &Value, out: &mut Map<String, Value>, field: &str) {
    if let Some(value) = args.get(field) {
        out.insert(field.into(), value.clone());
    }
}

pub(crate) fn required_id(args: &Value, field: &str) -> Result<String, String> {
    clean_id(&required_str(args, field)?)
}

pub(crate) fn required_any_id(args: &Value, fields: &[&str]) -> Result<String, String> {
    clean_id(&required_any_str(args, fields)?)
}

pub(crate) fn required_str(args: &Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

pub(crate) fn required_any_str(args: &Value, fields: &[&str]) -> Result<String, String> {
    fields
        .iter()
        .find_map(|field| required_str(args, field).ok())
        .ok_or_else(|| format!("missing string field '{}'", fields[0]))
}

fn clean_id(raw: &str) -> Result<String, String> {
    if raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe ClickUp id: {raw}"))
    }
}

pub(crate) fn push_bool(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(Value::as_bool) {
        query.push((field.into(), value.to_string()));
    }
}

pub(crate) fn push_u64(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(Value::as_u64) {
        query.push((field.into(), value.to_string()));
    }
}

pub(crate) fn push_str(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(Value::as_str) {
        query.push((field.into(), value.to_string()));
    }
}

pub(crate) fn push_array(args: &Value, query: &mut Vec<(String, String)>, source: &str, target: &str) {
    if let Some(values) = args.get(source).and_then(Value::as_array) {
        for value in values {
            if let Some(value) = value.as_str() {
                query.push((target.into(), value.to_string()));
            } else if let Some(value) = value.as_u64() {
                query.push((target.into(), value.to_string()));
            }
        }
    }
}

pub(crate) fn push_jsonish(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field) {
        if let Some(value) = value.as_str() {
            query.push((field.into(), value.to_string()));
        } else if value.is_array() || value.is_object() {
            query.push((field.into(), value.to_string()));
        }
    }
}
