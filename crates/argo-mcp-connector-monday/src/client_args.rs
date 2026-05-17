//! JSON argument parsing and GraphQL variable builders for the Monday.com client.

pub(crate) fn board_list_variables(args: &serde_json::Value) -> serde_json::Value {
    let mut vars = serde_json::Map::new();
    push_u64(args, &mut vars, "limit", 100);
    push_u64(args, &mut vars, "page", 500);
    serde_json::Value::Object(vars)
}

pub(crate) fn board_items_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut vars = board_id_variables_map(args)?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(25);
    vars.insert("limit".into(), limit.clamp(1, 100).into());
    Ok(serde_json::Value::Object(vars))
}

pub(crate) fn board_id_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Object(board_id_variables_map(args)?))
}

fn board_id_variables_map(
    args: &serde_json::Value,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let board_id = required_str(args, "board_id")?;
    let mut vars = serde_json::Map::new();
    vars.insert("board_ids".into(), serde_json::json!([board_id]));
    Ok(vars)
}

pub(crate) fn item_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let item_id = required_str(args, "item_id")?;
    Ok(serde_json::json!({"item_ids": [item_id]}))
}

pub(crate) fn create_item_variables(
    args: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut vars = serde_json::Map::new();
    vars.insert("board_id".into(), required_str(args, "board_id")?.into());
    vars.insert(
        "item_name".into(),
        required_any_str(args, &["item_name", "name"])?.into(),
    );
    push_optional_str(args, &mut vars, "group_id");
    if let Some(column_values) = column_values_arg(args)? {
        vars.insert("column_values".into(), column_values.into());
    }
    Ok(serde_json::Value::Object(vars))
}

pub(crate) fn update_status_variables(
    args: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut vars = serde_json::Map::new();
    vars.insert("board_id".into(), required_str(args, "board_id")?.into());
    vars.insert("item_id".into(), required_str(args, "item_id")?.into());
    vars.insert(
        "column_id".into(),
        optional_str(args, "column_id")
            .unwrap_or_else(|| "status".into())
            .into(),
    );
    vars.insert(
        "value".into(),
        required_any_str(args, &["status", "value"])?.into(),
    );
    Ok(serde_json::Value::Object(vars))
}

pub(crate) fn add_update_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "item_id": required_str(args, "item_id")?,
        "body": required_str(args, "body")?
    }))
}

fn column_values_arg(args: &serde_json::Value) -> Result<Option<String>, String> {
    match args.get("column_values") {
        None => Ok(None),
        Some(value) if value.is_object() || value.is_array() => Ok(Some(value.to_string())),
        Some(value) => value
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| "column_values must be an object, array, or JSON string".into()),
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_any_str(args: &serde_json::Value, fields: &[&str]) -> Result<String, String> {
    fields
        .iter()
        .find_map(|field| required_str(args, field).ok())
        .ok_or_else(|| format!("missing string field '{}', or '{}'", fields[0], fields[1]))
}

fn optional_str(args: &serde_json::Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
}

fn push_optional_str(
    args: &serde_json::Value,
    vars: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = optional_str(args, field) {
        vars.insert(field.into(), value.into());
    }
}

fn push_u64(
    args: &serde_json::Value,
    vars: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
    max: u64,
) {
    if let Some(value) = args.get(field).and_then(|v| v.as_u64()) {
        vars.insert(field.into(), value.clamp(1, max).into());
    }
}
