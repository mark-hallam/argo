//! Intake-capture wiring for `routes_genesis_stream` (UF Phase-A · A1-wire).
//!
//! A1 landed the [`argo_genesis::CapabilitySpec`] slots — `cadence` /
//! `exceptions` / `success_criteria` — plus the SPA capture panel
//! (`CadenceExceptionsCapture.tsx`) and the `vertical_ready` DISPLAY side
//! ([`super::routes_genesis_stream_payload::build_final_payload`] reads them via
//! `RoleContract::with_intake_slots`). What was missing was the INPUT seam: the
//! SSE handler never threaded the operator's submitted schedule / carve-outs /
//! measurable goals into the spec, so `plan.spec.success_criteria` was always
//! empty (`from_broad_draft` leaves the slots empty by contract — pinned by
//! `from_broad_draft_leaves_new_intake_slots_empty`).
//!
//! This module closes that seam WITHOUT touching the broad-draft contract: the
//! three values arrive on the same `POST /api/v1/genesis` body as separate
//! input fields, are parsed here, and are applied to the already-materialized
//! plan's spec. Only the from-scratch path carries a `spec` (`Some`); the
//! confident-archetype path has none, so there is nothing to enrich there
//! (the archetype review surfaces its KPIs from the manifest's `[[kpi]]` block).

use argo_genesis::{GenesisPlan, SuccessCriterion};
use serde::Deserialize;

/// The operator's intake-capture slots, parsed off the `POST /api/v1/genesis`
/// request body alongside `role_description`. Every field is
/// `#[serde(default)]` and the struct ignores the body's other keys, so the
/// text-only path (no capture-panel values) deserializes to the empty capture
/// and the spec slots stay empty — back-compat preserved.
#[derive(Debug, Default, Deserialize)]
pub(super) struct IntakeCapture {
    /// WHEN the teammate runs — free-text cadence/schedule.
    #[serde(default)]
    cadence: Option<String>,
    /// WHAT carve-outs apply — situations the teammate must hand off, not act on.
    #[serde(default)]
    exceptions: Vec<String>,
    /// HOW success is measured — typed, measurable criteria reusing the manifest
    /// `KpiKind` / `KpiComparator` vocabulary.
    #[serde(default)]
    success_criteria: Vec<SuccessCriterion>,
    /// WHAT systems/accounts the teammate will need (DEC-048 Access slot;
    /// draft-only names, never live bindings).
    #[serde(default)]
    access_integrations: Vec<String>,
    /// Escalate after N consecutive failures (DEC-048 Escalation slot;
    /// overrides the seeded default when supplied).
    #[serde(default)]
    escalation_failures_threshold: Option<u32>,
    /// Who to escalate to (DEC-048 Escalation slot; overrides the seeded
    /// default when supplied).
    #[serde(default)]
    escalation_to: Option<String>,
}

impl IntakeCapture {
    /// `true` when the operator supplied none of the three slots — the
    /// text-only path. Whitespace-only cadence / blank-named criteria do not
    /// count as supplied.
    fn has_values(&self) -> bool {
        self.cadence.as_deref().is_some_and(|c| !c.trim().is_empty())
            || self.exceptions.iter().any(|e| !e.trim().is_empty())
            || self
                .success_criteria
                .iter()
                .any(|c| !c.name.trim().is_empty())
            || self
                .access_integrations
                .iter()
                .any(|a| !a.trim().is_empty())
            || self.escalation_failures_threshold.is_some()
            || self
                .escalation_to
                .as_deref()
                .is_some_and(|e| !e.trim().is_empty())
    }
}

/// Parse the intake-capture slots off the request `body` and apply any supplied
/// values to the plan's [`argo_genesis::CapabilitySpec`].
///
/// A malformed / absent body yields the empty capture (the spec slots stay
/// empty), so this is always safe to call on the stream path. Only the
/// from-scratch path carries a `spec`; the archetype path (`spec == None`) is a
/// no-op.
pub(super) fn apply_intake(body: &str, plan: &mut GenesisPlan) {
    let capture: IntakeCapture = serde_json::from_str(body).unwrap_or_default();
    apply_capture(capture, plan);
}

/// Inner application step, split out so the unit tests can drive a typed
/// capture without round-tripping through JSON.
fn apply_capture(capture: IntakeCapture, plan: &mut GenesisPlan) {
    if !capture.has_values() {
        return;
    }
    let Some(spec) = plan.spec.as_mut() else {
        return;
    };
    if let Some(cadence) = capture
        .cadence
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
    {
        spec.cadence = Some(cadence);
    }
    for raw in capture.exceptions {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            spec.exceptions.push(trimmed.to_string());
        }
    }
    for mut criterion in capture.success_criteria {
        criterion.name = criterion.name.trim().to_string();
        if !criterion.name.is_empty() {
            spec.success_criteria.push(criterion);
        }
    }
    for raw in capture.access_integrations {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            spec.access_integrations.push(trimmed.to_string());
        }
    }
    // DEC-048: captured escalation prefs override the seeded conservative
    // defaults; absent values keep them (skip-safe).
    if let Some(threshold) = capture
        .escalation_failures_threshold
        .filter(|n| (1..=100).contains(n))
    {
        spec.escalation.escalate_after_failures = Some(threshold);
    }
    if let Some(to) = capture
        .escalation_to
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
    {
        spec.escalation.escalate_to = Some(to);
    }
}

#[cfg(test)]
#[path = "routes_genesis_stream_intake_tests.rs"]
mod tests;
