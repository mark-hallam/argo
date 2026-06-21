//! Richening helpers for the from-scratch broad-role synthesis path.
//!
//! The base broad-role prompt asks the model for only four fields
//! (persona description, action vocabulary, success metrics, onboarding
//! script). A teammate synthesised from just those four reads thin: its
//! system prompt falls back to the literal
//! `(derive your responsibilities from the charter above)` line
//! (`materialize_writer::*`), it carries no typed KPIs, no firing cadence,
//! and no access fence. This module holds the deterministic post-processing
//! that turns the model's richer JSON envelope into the typed
//! [`crate::spec`] shapes the hire→duty bridge (`store_hire_duty_shape.rs`)
//! and the role contract consume:
//!
//! - **Cadence** → a validated 5-field cron expression. The bridge's
//!   `normalize_stored_cron` accepts ONLY a 5- or 6-field cron; a free-text
//!   cadence would be skipped+warned and the duty would never fire. So a
//!   model-emitted `cadence_cron` is shape-checked here and dropped (→ no
//!   cadence) when it is not a real cron, rather than poisoning the bridge.
//! - **Access-integrations** → least-privilege whitelist. The model may only
//!   reference one of the 17 wired connectors; anything else is dropped (we
//!   never invent or surface a connector the platform cannot wire).
//! - **Success-criteria** → typed [`SuccessCriterion`] (kind + target +
//!   comparator), reusing the manifest KPI vocabulary so the captured KPIs
//!   are measurable, not free text.
//!
//! Everything here is deterministic and transport-free; the only model
//! dependence is the raw JSON the broad-role client returned.

use crate::spec::SuccessCriterion;
use argo_vertical::{KpiComparator, KpiKind};

/// The 17 wired MCP connectors a from-scratch role may reference (DEC v1
/// enterprise set). The broad-role synthesiser is held to this list: a
/// model-named integration outside it is dropped (least-privilege — never
/// surface an access need the platform cannot honour). Lower-case canonical
/// slugs; matching is case-insensitive after trimming.
pub(crate) const WIRED_CONNECTORS: &[&str] = &[
    "gmail",
    "gcal",
    "hubspot",
    "notion",
    "salesforce",
    "slack",
    "jira",
    "asana",
    "linear",
    "monday",
    "pagerduty",
    "fireflies",
    "airtable",
    "stripe",
    "zendesk",
    "clickup",
    "discord",
];

/// Canonicalise + whitelist a model-emitted access-integration name. Returns
/// the canonical connector slug when the (trimmed, lower-cased) name is one of
/// the [`WIRED_CONNECTORS`], else `None` — so an invented or unsupported
/// integration is silently dropped rather than surfaced as a draft binding.
#[must_use]
pub(crate) fn canonical_connector(raw: &str) -> Option<&'static str> {
    let needle = raw.trim().to_ascii_lowercase();
    WIRED_CONNECTORS
        .iter()
        .copied()
        .find(|c| *c == needle)
}

/// Filter a list of model-emitted access names down to the canonical wired
/// connectors, de-duplicated and order-preserving. The result is the
/// least-privilege access set a from-scratch teammate may reference.
#[must_use]
pub(crate) fn whitelist_access(raw: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in raw {
        if let Some(canon) = canonical_connector(name) {
            let canon = canon.to_string();
            if !out.contains(&canon) {
                out.push(canon);
            }
        }
    }
    out
}

/// `true` when `s` is a syntactically-shaped cron expression the hire→duty
/// bridge will accept: exactly 5 or 6 whitespace-separated fields, each field
/// non-empty and built only from the legal cron field alphabet
/// (`0-9 * / , -` and the day/month name letters). This is a deterministic
/// SHAPE gate, NOT a full parse — the bridge's `normalize_stored_cron` runs the
/// authoritative `Schedule::from_str` validation. We shape-check here so a
/// free-text cadence (`"every weekday at 9am"`) is dropped to `None` instead of
/// being threaded into the spec where it would later be skipped+warned by the
/// bridge and produce a teammate with NO firing duty.
#[must_use]
pub(crate) fn is_cron_shaped(s: &str) -> bool {
    let fields: Vec<&str> = s.split_whitespace().collect();
    if fields.len() != 5 && fields.len() != 6 {
        return false;
    }
    fields.iter().all(|f| {
        !f.is_empty()
            && f.chars().all(|c| {
                c.is_ascii_digit()
                    || matches!(c, '*' | '/' | ',' | '-')
                    || c.is_ascii_alphabetic()
            })
    })
}

/// Pick the cadence the spec should carry, given a model-emitted cron string.
/// Returns `Some(trimmed_cron)` only when it is [`is_cron_shaped`]; otherwise
/// `None` (the teammate carries no cadence rather than a non-firing one).
#[must_use]
pub(crate) fn cadence_from_cron(raw_cron: &str) -> Option<String> {
    let trimmed = raw_cron.trim();
    is_cron_shaped(trimmed).then(|| trimmed.to_string())
}

/// Parse one model success-criterion JSON object into a typed
/// [`SuccessCriterion`]. Tolerant: a missing/blank `name` drops the criterion
/// (`None`); a missing `kind`/`comparator` defaults to the honest placeholder
/// (`Count` / `Gte`) and a missing/`NaN` `target` defaults to `1.0` — the same
/// "captured but operator-refinable" floor the wizard path uses. Never panics.
#[must_use]
pub(crate) fn parse_success_criterion(value: &serde_json::Value) -> Option<SuccessCriterion> {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }
    let kind = value
        .get("kind")
        .and_then(|v| v.as_str())
        .map_or(KpiKind::Count, kpi_kind_from_str);
    let comparator = value
        .get("comparator")
        .and_then(|v| v.as_str())
        .map_or(KpiComparator::Gte, kpi_comparator_from_str);
    let target = value
        .get("target")
        .and_then(serde_json::Value::as_f64)
        .filter(|t| t.is_finite())
        .unwrap_or(1.0);
    Some(SuccessCriterion {
        name,
        kind,
        target,
        comparator,
    })
}

/// Map a model `kind` token to a [`KpiKind`]. Unknown tokens default to
/// [`KpiKind::Count`] (the safest, unit-free measure).
fn kpi_kind_from_str(raw: &str) -> KpiKind {
    match raw.trim().to_ascii_lowercase().as_str() {
        "percentage" | "percent" | "%" => KpiKind::Percentage,
        "duration_ms" | "durationms" | "duration" | "ms" => KpiKind::DurationMs,
        "ratio" => KpiKind::Ratio,
        "score" => KpiKind::Score,
        _ => KpiKind::Count,
    }
}

/// Map a model `comparator` token to a [`KpiComparator`]. Unknown tokens
/// default to [`KpiComparator::Gte`] ("at least", the common "more is better"
/// direction).
fn kpi_comparator_from_str(raw: &str) -> KpiComparator {
    match raw.trim().to_ascii_lowercase().as_str() {
        "lte" | "<=" | "at_most" | "below" => KpiComparator::Lte,
        "eq" | "==" | "equal" | "exactly" => KpiComparator::Eq,
        _ => KpiComparator::Gte,
    }
}

#[cfg(test)]
#[path = "broad_role_richen_tests.rs"]
mod tests;
