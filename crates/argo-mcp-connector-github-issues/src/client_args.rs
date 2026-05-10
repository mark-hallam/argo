//! JSON argument parsing and URL path helpers for the GitHub Issues client.

pub(crate) fn repo_path(args: &serde_json::Value) -> Result<(String, String), String> {
    Ok((
        clean_segment(&required_str(args, "owner")?)?,
        clean_segment(&required_str(args, "repo")?)?,
    ))
}

pub(crate) fn required_value(
    args: &serde_json::Value,
    field: &str,
) -> Result<serde_json::Value, String> {
    args.get(field)
        .filter(|v| !v.is_null())
        .cloned()
        .ok_or_else(|| format!("missing field '{field}'"))
}

pub(crate) fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

pub(crate) fn required_u64(args: &serde_json::Value, field: &str) -> Result<u64, String> {
    args.get(field)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("missing integer field '{field}'"))
}

pub(crate) fn push_value(
    args: &serde_json::Value,
    out: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = args.get(field).filter(|v| !v.is_null()) {
        out.insert(field.into(), value.clone());
    }
}

pub(crate) fn push_str(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
        out.push((field.into(), value.to_string()));
    }
}

pub(crate) fn push_str_or_default(
    args: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    field: &str,
    default: &str,
) {
    let value = args.get(field).and_then(|v| v.as_str()).unwrap_or(default);
    out.push((field.into(), value.to_string()));
}

pub(crate) fn limit_query(
    args: &serde_json::Value,
    default: u64,
    max: u64,
) -> Vec<(String, String)> {
    vec![(
        "per_page".into(),
        args.get("per_page")
            .or_else(|| args.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(default)
            .clamp(1, max)
            .to_string(),
    )]
}

pub(crate) fn optional_string_array(
    args: &serde_json::Value,
    field: &str,
) -> Result<Vec<String>, String> {
    let Some(value) = args.get(field) else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        return Err(format!("field '{field}' must be an array of strings"));
    };
    values
        .iter()
        .map(|v| {
            v.as_str()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("field '{field}' must contain only strings"))
        })
        .collect()
}

fn clean_segment(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe GitHub path segment: {raw}"))
    }
}

pub(crate) fn encode_path_segment(raw: &str) -> String {
    raw.bytes()
        .flat_map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![b as char]
            }
            _ => format!("%{b:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_paths_are_percent_encoded() {
        assert_eq!(encode_path_segment("needs review"), "needs%20review");
        assert_eq!(encode_path_segment("a/b"), "a%2Fb");
    }
}
