//! Capability-gap detection (#682, refined-mission work order #4).
//!
//! Observes the audit chain and emits structured signals when
//! the agent is repeatedly running into a wall — e.g., a tool
//! it can't dispatch (RbacDeny), a request it keeps failing,
//! a step that requires human approval. These signals are
//! written back into the chain as `capability.gap` audit rows
//! so the operator (or a later improvement-proposal flow) can
//! act on them.
//!
//! **Rule (work order #4): the detector NEVER auto-bypasses
//! policy.** It records signals only. Any improvement (e.g.
//! "promote this denied tool to allowed") goes through the
//! existing review gate (#681 learned-skill lifecycle, or a
//! human-driven CLI/gateway approval path).
//!
//! Today this module ships **one** detector — policy denials —
//! as the foundation. The other signal kinds in work order #4
//! (manual correction, weak evidence, missing knowledge,
//! repeated similar task) follow in subsequent PRs as the
//! agent runner accumulates the upstream telemetry.
use crate::AuditKind;
use argo_store::SessionStore;
use argo_types::ArgoResult;
use std::collections::HashMap;

/// One detected gap. Stable wire shape — also the JSON payload
/// of the corresponding `capability.gap` audit row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGap {
    /// What kind of gap this is.
    pub kind: GapKind,
    /// Tool / asset name the signal refers to (e.g. the denied
    /// tool name for policy_denial). May be empty for kinds
    /// that don't have a single anchor.
    pub anchor: String,
    /// How many underlying signals fed this gap (e.g. number of
    /// denials for a policy_denial gap).
    pub count: u64,
    /// Free-form one-line explanation, surfaced to the operator.
    pub message: String,
}

/// Stable label ladder for capability-gap kinds. Wire labels
/// are dot-separated lowercase to match the audit-kind
/// convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapKind {
    /// Agent kept trying to dispatch a tool the policy
    /// (deny-list / vertical manifest) refused. Repeated denial
    /// is a strong signal that either (a) the manifest is too
    /// strict for the task, OR (b) the agent is confused about
    /// the available toolset.
    PolicyDenial,
}

impl GapKind {
    /// Stable wire label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::PolicyDenial => "policy_denial",
        }
    }
}

impl CapabilityGap {
    /// JSON payload shape for the `capability.gap` audit row.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": self.kind.label(),
            "anchor": self.anchor,
            "count": self.count,
            "message": self.message,
        })
    }
}

/// Scan the session's recent audit chain for `rbac.deny` rows
/// and return a [`GapKind::PolicyDenial`] for every tool that
/// hit `>= threshold` denials within the window. `since_minutes
/// = 0` means "no time filter" — count every denial in the
/// session.
///
/// Caller decides whether to write the result back to the
/// audit chain as `capability.gap` rows (the runtime path) or
/// surface them to the operator (the CLI / gateway path).
pub async fn detect_policy_denials(
    store: &SessionStore,
    session_id: argo_types::SessionId,
    since_minutes: i64,
    threshold: u64,
) -> ArgoResult<Vec<CapabilityGap>> {
    let rows = store.list_audit_for_session(session_id).await?;
    let cutoff: Option<i64> = if since_minutes > 0 {
        // `created_at` is stored as Unix-epoch SECONDS in the
        // audit_record schema. Compare apples-to-apples.
        Some(chrono::Utc::now().timestamp() - since_minutes * 60)
    } else {
        None
    };
    let mut counts: HashMap<String, u64> = HashMap::new();
    let deny_label = AuditKind::RbacDeny.label();
    for row in rows {
        if row.kind != deny_label {
            continue;
        }
        if let Some(c) = cutoff {
            if row.created_at < c {
                continue;
            }
        }
        let parsed: serde_json::Value =
            serde_json::from_str(&row.payload_json).unwrap_or(serde_json::Value::Null);
        if let Some(tool) = parsed.get("tool").and_then(|v| v.as_str()) {
            *counts.entry(tool.to_string()).or_insert(0) += 1;
        }
    }
    let mut out = Vec::new();
    for (tool, count) in counts {
        if count >= threshold {
            out.push(CapabilityGap {
                kind: GapKind::PolicyDenial,
                anchor: tool.clone(),
                count,
                message: format!(
                    "tool '{tool}' was denied {count} time(s) — manifest may be too strict, or the agent is confused about the toolset"
                ),
            });
        }
    }
    // Stable order: highest count first, then lexicographic.
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.anchor.cmp(&b.anchor)));
    Ok(out)
}

#[cfg(test)]
#[path = "gap_tests.rs"]
mod tests;
