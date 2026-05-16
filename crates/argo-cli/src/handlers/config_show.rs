//! `argo config show` — read-only effective-config inspection.

use argo_config::source_tracking::{secrets_display_allowed, ConfigSourceMap};
use argo_types::{ArgoError, ArgoResult};
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

const REDACTED: &str = "[redacted]";

/// Run `argo config show`.
pub fn run(key: Option<&str>, json: bool, source: bool, show_secrets: bool) -> ArgoResult<()> {
    let cfg = argo_config::load()?;
    let reveal = secrets_display_allowed(show_secrets);
    let mut json_value = serde_json::to_value(&cfg)
        .map_err(|e| ArgoError::Provider(format!("serialize config to json: {e}")))?;
    redact_json(&mut json_value, reveal);

    if let Some(key) = key {
        return print_key(&json_value, key, json, source);
    }

    if json {
        let body = serde_json::to_string_pretty(&json_value)
            .map_err(|e| ArgoError::Provider(format!("render config json: {e}")))?;
        println!("{body}");
        return Ok(());
    }

    let mut toml_value = TomlValue::try_from(&cfg)
        .map_err(|e| ArgoError::Provider(format!("serialize config to toml: {e}")))?;
    redact_toml(&mut toml_value, reveal);
    let body = toml::to_string_pretty(&toml_value)
        .map_err(|e| ArgoError::Provider(format!("render config toml: {e}")))?;
    if source {
        let sources = ConfigSourceMap::detect();
        print!("{}", annotate_toml_sources(&body, &sources));
    } else {
        print!("{body}");
    }
    Ok(())
}

fn print_key(value: &JsonValue, key: &str, json: bool, source: bool) -> ArgoResult<()> {
    let found = lookup_effective_key(value, key)
        .ok_or_else(|| ArgoError::Provider(format!("config key '{key}' not found")))?;
    if json {
        let body = serde_json::to_string_pretty(&found)
            .map_err(|e| ArgoError::Provider(format!("render config key json: {e}")))?;
        println!("{body}");
        return Ok(());
    }
    let rendered = render_key_value(&found)?;
    if source {
        let sources = ConfigSourceMap::detect();
        println!(
            "{rendered} # source: {}",
            source_label_for_key(&sources, key)
        );
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn lookup_effective_key(value: &JsonValue, dotted: &str) -> Option<JsonValue> {
    if dotted == "gateway.bind" {
        return gateway_bind_value(value);
    }
    lookup_json(value, dotted).cloned()
}

fn gateway_bind_value(value: &JsonValue) -> Option<JsonValue> {
    let host = lookup_json(value, "gateway.host")?.as_str()?;
    let port = lookup_json(value, "gateway.port")?.as_u64()?;
    Some(JsonValue::String(format!("{host}:{port}")))
}

fn source_label_for_key(sources: &ConfigSourceMap, key: &str) -> String {
    if key == "gateway.bind" {
        return format!(
            "gateway.host: {}, gateway.port: {}",
            sources.source_for("gateway.host"),
            sources.source_for("gateway.port")
        );
    }
    sources.source_for(key).into()
}

fn render_key_value(value: &JsonValue) -> ArgoResult<String> {
    match value {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Bool(b) => Ok(b.to_string()),
        JsonValue::Number(n) => Ok(n.to_string()),
        JsonValue::Null => Ok("null".into()),
        JsonValue::Array(_) | JsonValue::Object(_) => serde_json::to_string_pretty(value)
            .map_err(|e| ArgoError::Provider(format!("render config key: {e}"))),
    }
}

fn lookup_json<'a>(value: &'a JsonValue, dotted: &str) -> Option<&'a JsonValue> {
    let parts: Vec<&str> = dotted.split('.').filter(|part| !part.is_empty()).collect();
    lookup_parts(value, &parts)
}

fn lookup_parts<'a>(value: &'a JsonValue, parts: &[&str]) -> Option<&'a JsonValue> {
    if parts.is_empty() {
        return Some(value);
    }
    let JsonValue::Object(map) = value else {
        return None;
    };
    for end in 1..=parts.len() {
        let key = parts[..end].join(".");
        if let Some(next) = map.get(&key) {
            if let Some(found) = lookup_parts(next, &parts[end..]) {
                return Some(found);
            }
        }
    }
    None
}

fn redact_json(value: &mut JsonValue, reveal: bool) {
    match value {
        JsonValue::Object(map) => {
            for (key, child) in map {
                if should_redact(key, child, reveal) {
                    *child = JsonValue::String(REDACTED.into());
                } else {
                    redact_json(child, reveal);
                }
            }
        }
        JsonValue::Array(values) => {
            for child in values {
                redact_json(child, reveal);
            }
        }
        _ => {}
    }
}

fn redact_toml(value: &mut TomlValue, reveal: bool) {
    match value {
        TomlValue::Table(table) => {
            for (key, child) in table {
                if should_redact_toml(key, child, reveal) {
                    *child = TomlValue::String(REDACTED.into());
                } else {
                    redact_toml(child, reveal);
                }
            }
        }
        TomlValue::Array(values) => {
            for child in values {
                redact_toml(child, reveal);
            }
        }
        _ => {}
    }
}

fn should_redact(key: &str, value: &JsonValue, reveal: bool) -> bool {
    !reveal && matches!(value, JsonValue::String(s) if !s.is_empty()) && secret_key_name(key)
}

fn should_redact_toml(key: &str, value: &TomlValue, reveal: bool) -> bool {
    !reveal && matches!(value, TomlValue::String(s) if !s.is_empty()) && secret_key_name(key)
}

fn secret_key_name(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("api_key")
        || lower.contains("secret")
        || lower.contains("password")
        || lower == "token"
        || lower.ends_with("_token")
}

fn annotate_toml_sources(body: &str, sources: &ConfigSourceMap) -> String {
    let mut out = String::with_capacity(body.len() + 256);
    let mut section = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed
                .trim_matches('[')
                .trim_matches(']')
                .trim_matches('"')
                .to_string();
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let Some((key, _)) = trimmed.split_once('=') else {
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let key = key.trim().trim_matches('"');
        let dotted = if section.is_empty() {
            key.to_string()
        } else {
            format!("{section}.{key}")
        };
        out.push_str(line);
        out.push_str(" # source: ");
        out.push_str(sources.source_for(&dotted));
        out.push('\n');
    }
    out
}

#[cfg(test)]
#[path = "config_show_tests.rs"]
mod tests;
