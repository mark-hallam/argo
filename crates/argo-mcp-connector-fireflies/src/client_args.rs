//! JSON argument parsing helpers for the Fireflies.ai client.
use serde_json::{Map, Value};

pub(crate) fn transcript_variables(
    args: &Value,
    require_keyword: bool,
) -> Result<Map<String, Value>, String> {
    let mut variables = Map::new();
    if require_keyword {
        variables.insert("keyword".into(), required_str(args, "keyword")?.into());
    } else {
        copy_string(args, &mut variables, "keyword", "keyword");
    }
    copy_string(args, &mut variables, "scope", "scope");
    copy_string_alias(args, &mut variables, &["from_date", "fromDate"], "fromDate");
    copy_string_alias(args, &mut variables, &["to_date", "toDate"], "toDate");
    copy_number(args, &mut variables, "date", "date");
    copy_u64(args, &mut variables, "limit", "limit", 1, 50);
    copy_u64(args, &mut variables, "skip", "skip", 0, u64::MAX);
    copy_string(args, &mut variables, "host_email", "hostEmail");
    copy_string(args, &mut variables, "user_id", "userId");
    copy_bool(args, &mut variables, "mine", "mine");
    copy_array(args, &mut variables, "organizers", "organizers");
    copy_array(args, &mut variables, "participants", "participants");
    copy_string(args, &mut variables, "channel_id", "channelId");
    Ok(variables)
}

pub(crate) fn required_id(args: &Value) -> Result<String, String> {
    optional_str(args, "meeting_id")
        .or_else(|| optional_str(args, "transcript_id"))
        .ok_or_else(|| "missing string field 'meeting_id'".into())
}

fn required_str(args: &Value, field: &str) -> Result<String, String> {
    optional_str(args, field).ok_or_else(|| format!("missing string field '{field}'"))
}

fn optional_str(args: &Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn copy_string(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = optional_str(args, source) {
        out.insert(target.into(), value.into());
    }
}

fn copy_string_alias(args: &Value, out: &mut Map<String, Value>, sources: &[&str], target: &str) {
    if let Some(value) = sources.iter().find_map(|source| optional_str(args, source)) {
        out.insert(target.into(), value.into());
    }
}

fn copy_number(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = args.get(source).filter(|value| value.is_number()) {
        out.insert(target.into(), value.clone());
    }
}

fn copy_u64(
    args: &Value,
    out: &mut Map<String, Value>,
    source: &str,
    target: &str,
    min: u64,
    max: u64,
) {
    if let Some(value) = args.get(source).and_then(Value::as_u64) {
        out.insert(target.into(), value.clamp(min, max).into());
    }
}

fn copy_bool(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = args.get(source).and_then(Value::as_bool) {
        out.insert(target.into(), value.into());
    }
}

fn copy_array(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = args.get(source).filter(|value| value.is_array()) {
        out.insert(target.into(), value.clone());
    }
}
