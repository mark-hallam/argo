use crate::archetype::{Archetype, ClassificationResult};
use argo_types::{ArgoError, ArgoResult};

pub(crate) fn parse_confidence_response(raw: &str) -> ClassificationResult {
    let mut parts = raw.split_whitespace();
    let slug = parts.next().unwrap_or("");
    let confidence: f32 = parts
        .next()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let archetype = if slug.eq_ignore_ascii_case("none") || slug.is_empty() {
        None
    } else {
        Archetype::from_slug(slug)
    };
    ClassificationResult {
        archetype,
        confidence,
    }
}

#[derive(Debug)]
pub(crate) struct ParsedBroadRole {
    pub(crate) persona_description: String,
    pub(crate) action_vocabulary: Vec<String>,
    pub(crate) success_metrics: Vec<String>,
    pub(crate) onboarding_script: String,
}

pub(crate) fn parse_broad_role_response(raw: &str) -> ArgoResult<ParsedBroadRole> {
    let json_text = first_balanced_object(raw)
        .ok_or_else(|| ArgoError::Provider("broad-role response: no JSON object".into()))?;
    let value: serde_json::Value = serde_json::from_str(json_text)
        .map_err(|e| ArgoError::Provider(format!("broad-role JSON parse: {e}")))?;
    let persona_description = json_string(&value, "persona_description");
    let action_vocabulary = json_string_array(&value, "action_vocabulary");
    let success_metrics = json_string_array(&value, "success_metrics");
    let onboarding_script = json_string(&value, "onboarding_script");
    if persona_description.is_empty() || action_vocabulary.is_empty() {
        return Err(ArgoError::Provider(
            "broad-role response missing persona_description or action_vocabulary".into(),
        ));
    }
    Ok(ParsedBroadRole {
        persona_description,
        action_vocabulary,
        success_metrics,
        onboarding_script,
    })
}

fn json_string(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn json_string_array(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(str::trim))
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Extract the first balanced top-level JSON object from `s`.
pub(crate) fn first_balanced_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let bytes = s.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}
