use crate::archetype::{Archetype, ClassificationResult};
use crate::spec::SuccessCriterion;
use super::broad_role_richen::{cadence_from_cron, parse_success_criterion, whitelist_access};
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
    /// Concrete day-to-day duties — populates `CapabilitySpec.responsibilities`
    /// so the synthesized system prompt no longer falls back to the literal
    /// `(derive your responsibilities from the charter above)` line.
    pub(crate) responsibilities: Vec<String>,
    /// A firing cadence as a validated 5/6-field cron string (or `None` when the
    /// model emitted free text / no cron) — the shape the hire→duty bridge's
    /// `normalize_stored_cron` accepts.
    pub(crate) cadence: Option<String>,
    /// Typed, measurable success criteria (KPIs) — reuses the manifest KPI
    /// vocabulary so the captured goals are enforceable, not free text.
    pub(crate) success_criteria: Vec<SuccessCriterion>,
    /// Least-privilege access set — only the wired connectors the model named
    /// (anything else dropped). Surfaced draft-only ("to be connected").
    pub(crate) access_integrations: Vec<String>,
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
    // Richer fields are ADDITIVE + tolerant: a model that omits any of them
    // (or emits a malformed shape) degrades to the empty/None default, never a
    // parse error — so the base contract (persona + actions) stays the only
    // hard requirement and a pre-richening model still synthesizes a draft.
    let responsibilities = json_string_array(&value, "responsibilities");
    let cadence = value
        .get("cadence_cron")
        .and_then(|v| v.as_str())
        .and_then(cadence_from_cron);
    let success_criteria = parse_success_criteria(&value);
    let access_integrations = whitelist_access(&json_string_array(&value, "access_integrations"));
    Ok(ParsedBroadRole {
        persona_description,
        action_vocabulary,
        success_metrics,
        onboarding_script,
        responsibilities,
        cadence,
        success_criteria,
        access_integrations,
    })
}

/// Parse the `success_criteria` array of typed objects. Each well-formed entry
/// becomes a [`SuccessCriterion`]; malformed/blank entries are dropped. Returns
/// an empty vec when the key is absent or not an array (tolerant).
fn parse_success_criteria(value: &serde_json::Value) -> Vec<SuccessCriterion> {
    value
        .get("success_criteria")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_success_criterion).collect())
        .unwrap_or_default()
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

/// `true` when a broad-role provider response contains **no** balanced JSON
/// object at all — i.e. the model returned prose rather than the requested
/// JSON envelope. This is the refusal signal: the most common cause is the
/// model declining to synthesize a destructive / unsafe role and answering in
/// natural language instead.
///
/// It is deliberately distinct from a *transient* parse glitch (a `{...}` that
/// IS present but fails `serde_json::from_str`, or is missing required fields).
/// [`parse_broad_role_response`] still reports that glitch as a retryable
/// `ArgoError::Provider`; the caller routes a refusal (this predicate `true`)
/// to the graceful, recoverable [`crate::GenesisError::RoleRefused`] instead.
pub(crate) fn is_non_json_refusal(raw: &str) -> bool {
    first_balanced_object(raw).is_none()
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
