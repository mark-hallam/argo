//! Translate the operator's visual "Set boundaries" spec (Genesis Phase-2
//! Onboarding Studio) into an enforceable [`TruthBoundaryPolicy`].
//!
//! ## What this module is
//!
//! The `/genesis` "Set boundaries" panel lets the operator say — with toggles,
//! a slider, and free-text topics — *what a teammate may touch*, *what dollar
//! actions need sign-off*, and *which questions must be escalated to a human*.
//! This module is the function-side translator: a [`BoundariesSpec`] (the wire
//! shape the SPA posts) becomes a canonical [`TruthBoundaryPolicy`] that the
//! already-shipped pure evaluator ([`argo_capability::evaluate_claim`]) and the
//! runtime turn-exit gate enforce.
//!
//! It **consumes** the canonical `argo_capability::TruthBoundaryPolicy` — it
//! deliberately declares NO parallel policy type. The synthesized policy is
//! serialized to the SAME TOML shape `argo_capability::load_policy` reads, so a
//! round-trip (synthesize → write → `load_policy` → `evaluate_claim`) proves the
//! boundaries the operator set are the boundaries the runtime enforces.
//!
//! ## Honest ceiling (carried from `genesis_truth_boundary`)
//!
//! The pure evaluator matches only the claim classes a *producer declares* on a
//! candidate utterance — the promised semantic classifier was never built. So an
//! **undeclared** over-claim is still `Allow`-by-default. A `read_only` data
//! category or an escalation topic therefore gates a *declared* claim in that
//! class (a real improvement over a policy-less worker), not every conceivable
//! over-claim. The SPA copy reflects this.
//!
//! ## Scope boundary (this brief)
//!
//! Persisting the synthesized policy so the runtime auto-loads it *per worker*
//! at session boot needs a per-worker policy-routing store that does not exist
//! today (runtime loading is process-global via `ARGO_TRUTH_BOUNDARY_POLICY_PATH`
//! — see `argo_agent_runner::turn_helpers_truth_boundary`). That routing is a
//! separate follow-up brief. Here we (a) synthesize + serialize the policy in
//! the canonical loadable format, (b) write it next to the generated vertical as
//! the artifact that future loader will read, and (c) echo what was synthesized
//! on the `vertical_ready` SSE payload so the SPA renders server truth, not local
//! form state.

use argo_capability::{
    ChannelScope, ClaimClass, ClaimDisposition, EscalationKind, EscalationTarget,
    TruthBoundaryPolicy,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The stable `policy_id` stamped on a Genesis boundaries-synthesized policy.
pub const GENESIS_BOUNDARIES_POLICY_ID: &str = "genesis-boundaries";

/// Filename the synthesized policy is written under, next to the generated
/// vertical. The canonical [`argo_capability::load_policy`] reads this exact
/// TOML shape.
pub const BOUNDARIES_POLICY_FILENAME: &str = "truth-boundary-policy.toml";

/// A data category the operator can scope to read-only / read-write. Each maps
/// onto a canonical truth-boundary claim-class token a producer can declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataCategory {
    /// Customer personally-identifiable information.
    CustomerPii,
    /// Financial data: revenue, pricing, refunds, balances.
    Financial,
    /// Contract / legal-agreement content.
    Contracts,
    /// Source code / internal engineering material.
    SourceCode,
}

impl DataCategory {
    /// The canonical claim-class token a producer declares for this category.
    /// Kept stable: the runtime + canonical-Q&A skill agree by string.
    #[must_use]
    pub fn claim_class(self) -> &'static str {
        match self {
            DataCategory::CustomerPii => "customer-pii",
            DataCategory::Financial => "financial",
            DataCategory::Contracts => "contracts",
            DataCategory::SourceCode => "source-code",
        }
    }

    /// Stable wire token (matches the serde `snake_case` rename).
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            DataCategory::CustomerPii => "customer_pii",
            DataCategory::Financial => "financial",
            DataCategory::Contracts => "contracts",
            DataCategory::SourceCode => "source_code",
        }
    }
}

/// How much access the teammate has to a [`DataCategory`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    /// May read, but any declared claim/action in this category is held for
    /// human sign-off (Escalate).
    ReadOnly,
    /// No boundary added for this category (Allow-by-default).
    ReadWrite,
}

/// One access-scope toggle from the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessScope {
    /// Which data category.
    pub category: DataCategory,
    /// Read-only (gated) or read-write (ungated).
    pub access: AccessLevel,
}

/// The full "Set boundaries" spec the SPA posts on the genesis request.
///
/// All fields default-empty so the wire shape is byte-unchanged for a request
/// that sets no boundaries (the panel is optional).
///
/// ```
/// use argo_genesis::policy_synth::{AccessLevel, AccessScope, BoundariesSpec, DataCategory};
///
/// let spec = BoundariesSpec {
///     access_scopes: vec![AccessScope {
///         category: DataCategory::Financial,
///         access: AccessLevel::ReadOnly,
///     }],
///     approval_threshold_usd: Some(500.0),
///     escalation_topics: vec!["legal".into()],
/// };
/// assert_eq!(spec.access_scopes.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BoundariesSpec {
    /// Per-category access toggles.
    #[serde(default)]
    pub access_scopes: Vec<AccessScope>,
    /// Dollar ceiling above which money actions (refunds, sends) need human
    /// sign-off. `None` = no dollar ceiling configured.
    #[serde(default)]
    pub approval_threshold_usd: Option<f64>,
    /// Free-text "must escalate questions about X" topics (legal, compliance…).
    #[serde(default)]
    pub escalation_topics: Vec<String>,
}

impl BoundariesSpec {
    /// True when the operator configured nothing — the wire omits the field
    /// entirely in that case, so this also guards the SSE echo.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.access_scopes.is_empty()
            && self.approval_threshold_usd.is_none()
            && self.escalation_topics.is_empty()
    }
}

/// Normalize a free-text escalation topic into a stable claim-class token:
/// lowercase, trim, non-alphanumeric runs collapse to a single `-`. Empty after
/// normalization → `None` (skipped).
#[must_use]
pub fn normalize_topic(raw: &str) -> Option<String> {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.trim().to_lowercase().chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// The ordered, de-duplicated list of claim-class names a spec gates (Escalate).
///
/// Used by BOTH [`synthesize_policy`] and [`boundaries_echo`] so the policy the
/// runtime enforces and the summary the SPA renders can never diverge.
#[must_use]
pub fn gated_class_names(spec: &BoundariesSpec) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let push = |n: String, names: &mut Vec<String>| {
        if !names.contains(&n) {
            names.push(n);
        }
    };
    for scope in &spec.access_scopes {
        if scope.access == AccessLevel::ReadOnly {
            push(scope.category.claim_class().to_string(), &mut names);
        }
    }
    // A configured dollar ceiling implies money actions need sign-off, so the
    // financial class is gated even if its access toggle is read-write.
    if spec.approval_threshold_usd.is_some() {
        push(DataCategory::Financial.claim_class().to_string(), &mut names);
    }
    for topic in &spec.escalation_topics {
        if let Some(n) = normalize_topic(topic) {
            push(n, &mut names);
        }
    }
    names
}

/// Synthesize the canonical [`TruthBoundaryPolicy`] for `tenant_id` from a spec.
///
/// Every gated class (read-only category, configured dollar ceiling, escalation
/// topic) becomes an `Escalate` claim class on all channels, routed to the
/// tenant operator. The policy `tenant_id` is the worker's tenant, so the
/// runtime's per-turn tenant guard (`active_policy_for` — applies a policy only
/// when `policy.tenant_id == session.tenant_id`) keeps it from ever applying to
/// another tenant's session.
///
/// ```
/// use argo_genesis::policy_synth::{synthesize_policy, BoundariesSpec};
/// use argo_capability::{evaluate_claim, ChannelKind, ClaimCandidate, ClaimDisposition};
/// use argo_types::TenantId;
///
/// let spec = BoundariesSpec {
///     escalation_topics: vec!["legal".into()],
///     ..Default::default()
/// };
/// let policy = synthesize_policy(&spec, TenantId::new());
/// let verdict = evaluate_claim(
///     &policy,
///     ChannelKind::Chat,
///     &ClaimCandidate { text: "you should sue".into(), declared_classes: vec!["legal".into()] },
/// );
/// assert_eq!(verdict.disposition, ClaimDisposition::Escalate);
/// ```
#[must_use]
pub fn synthesize_policy(spec: &BoundariesSpec, tenant_id: TenantId) -> TruthBoundaryPolicy {
    let claim_classes = gated_class_names(spec)
        .into_iter()
        .map(|name| ClaimClass {
            name,
            disposition: ClaimDisposition::Escalate,
            channels: ChannelScope::AllChannels,
            escalate_to: Some(EscalationTarget {
                kind: EscalationKind::Approver,
                address: "operator".to_string(),
            }),
        })
        .collect();
    TruthBoundaryPolicy {
        id: GENESIS_BOUNDARIES_POLICY_ID.to_string(),
        tenant_id,
        claim_classes,
    }
}

/// The server-truth summary the SSE `vertical_ready` payload carries, so the SPA
/// renders what was actually synthesized (no theater — not the local form).
///
/// ```
/// use argo_genesis::policy_synth::{boundaries_echo, BoundariesSpec};
///
/// let spec = BoundariesSpec { approval_threshold_usd: Some(500.0), ..Default::default() };
/// let echo = boundaries_echo(&spec);
/// assert_eq!(echo["approval_threshold_usd"], 500.0);
/// assert_eq!(echo["gated_classes"][0], "financial");
/// ```
#[must_use]
pub fn boundaries_echo(spec: &BoundariesSpec) -> Value {
    let access_scopes: Vec<Value> = spec
        .access_scopes
        .iter()
        .map(|s| {
            json!({
                "category": s.category.wire(),
                "access": match s.access {
                    AccessLevel::ReadOnly => "read_only",
                    AccessLevel::ReadWrite => "read_write",
                },
            })
        })
        .collect();
    let topics: Vec<String> = spec
        .escalation_topics
        .iter()
        .filter_map(|t| normalize_topic(t))
        .collect();
    json!({
        "policy_id": GENESIS_BOUNDARIES_POLICY_ID,
        "gated_classes": gated_class_names(spec),
        "approval_threshold_usd": spec.approval_threshold_usd,
        "escalation_topics": topics,
        "access_scopes": access_scopes,
    })
}

/// Serialize a policy to the TOML shape [`argo_capability::load_policy`] reads.
#[must_use]
pub fn policy_to_toml(policy: &TruthBoundaryPolicy) -> String {
    let mut out = String::new();
    out.push_str(&format!("id = {}\n", toml_str(&policy.id)));
    out.push_str(&format!("tenant_id = {}\n", toml_str(&policy.tenant_id.0.to_string())));
    for class in &policy.claim_classes {
        out.push_str("\n[[claim_classes]]\n");
        out.push_str(&format!("name = {}\n", toml_str(&class.name)));
        match &class.disposition {
            ClaimDisposition::Allow => out.push_str("disposition = \"Allow\"\n"),
            ClaimDisposition::Forbid { refusal_template_id } => {
                out.push_str("disposition = \"Forbid\"\n");
                out.push_str(&format!(
                    "refusal_template_id = {}\n",
                    toml_str(refusal_template_id)
                ));
            }
            ClaimDisposition::Escalate => out.push_str("disposition = \"Escalate\"\n"),
        }
        out.push_str(&format!("channels = {}\n", toml_str(&channels_str(&class.channels))));
        if let Some(target) = &class.escalate_to {
            out.push_str(&format!(
                "escalate_to = {{ kind = {}, address = {} }}\n",
                toml_str(escalation_kind_str(target.kind)),
                toml_str(&target.address)
            ));
        }
    }
    out
}

/// Synthesize + write the policy TOML next to the generated vertical, returning
/// the written path. This is the artifact the (deferred) per-worker policy
/// loader will read; the canonical [`argo_capability::load_policy`] reads it
/// today in the round-trip test.
///
/// # Errors
/// Returns [`ArgoError::Io`] if the file cannot be written.
pub fn persist_policy(
    spec: &BoundariesSpec,
    tenant_id: TenantId,
    vertical_dir: &Path,
) -> ArgoResult<PathBuf> {
    let policy = synthesize_policy(spec, tenant_id);
    let toml = policy_to_toml(&policy);
    let path = vertical_dir.join(BOUNDARIES_POLICY_FILENAME);
    std::fs::write(&path, toml).map_err(|e| {
        ArgoError::Io(format!(
            "failed to write boundaries policy at {}: {e}",
            path.display()
        ))
    })?;
    Ok(path)
}

/// Deterministic smart defaults from the role description — no LLM call. Pre-fills
/// the panel for three archetypes (Customer Success, Sales SDR, Marketplace
/// Seller); a role that matches none gets a conservative all-read-only default.
#[must_use]
pub fn smart_defaults(role_text: &str) -> BoundariesSpec {
    let t = role_text.to_lowercase();
    let scope = |category, access| AccessScope { category, access };
    let ro = AccessLevel::ReadOnly;
    let rw = AccessLevel::ReadWrite;

    if t.contains("customer success") || t.contains("customer support") || t.contains("support") {
        BoundariesSpec {
            access_scopes: vec![
                scope(DataCategory::CustomerPii, rw),
                scope(DataCategory::Financial, ro),
                scope(DataCategory::Contracts, ro),
                scope(DataCategory::SourceCode, ro),
            ],
            approval_threshold_usd: Some(500.0),
            escalation_topics: vec!["legal".into(), "compliance".into()],
        }
    } else if t.contains("sales") || t.contains("sdr") || t.contains("account executive") {
        BoundariesSpec {
            access_scopes: vec![
                scope(DataCategory::CustomerPii, rw),
                scope(DataCategory::Financial, ro),
                scope(DataCategory::Contracts, ro),
                scope(DataCategory::SourceCode, ro),
            ],
            approval_threshold_usd: Some(1000.0),
            escalation_topics: vec!["legal".into(), "pricing".into()],
        }
    } else if t.contains("marketplace") || t.contains("seller") {
        BoundariesSpec {
            access_scopes: vec![
                scope(DataCategory::CustomerPii, ro),
                scope(DataCategory::Financial, ro),
                scope(DataCategory::Contracts, ro),
                scope(DataCategory::SourceCode, ro),
            ],
            approval_threshold_usd: Some(250.0),
            escalation_topics: vec!["legal".into(), "refunds".into()],
        }
    } else {
        BoundariesSpec {
            access_scopes: vec![
                scope(DataCategory::CustomerPii, ro),
                scope(DataCategory::Financial, ro),
                scope(DataCategory::Contracts, ro),
                scope(DataCategory::SourceCode, ro),
            ],
            approval_threshold_usd: None,
            escalation_topics: vec!["legal".into()],
        }
    }
}

fn channels_str(scope: &ChannelScope) -> String {
    match scope {
        ChannelScope::AllChannels => "AllChannels".to_string(),
        ChannelScope::Only(list) => format!("Only:{}", join_channels(list)),
        ChannelScope::Except(list) => format!("Except:{}", join_channels(list)),
    }
}

fn join_channels(list: &[argo_capability::ChannelKind]) -> String {
    list.iter()
        .map(|c| channel_kind_str(*c))
        .collect::<Vec<_>>()
        .join(",")
}

fn channel_kind_str(kind: argo_capability::ChannelKind) -> &'static str {
    use argo_capability::ChannelKind;
    match kind {
        ChannelKind::Rehearsal => "Rehearsal",
        ChannelKind::WebEmbed => "WebEmbed",
        ChannelKind::LiveMeeting => "LiveMeeting",
        ChannelKind::Chat => "Chat",
    }
}

fn escalation_kind_str(kind: EscalationKind) -> &'static str {
    match kind {
        EscalationKind::Approver => "Approver",
        EscalationKind::Human => "Human",
        EscalationKind::Webhook => "Webhook",
    }
}

/// Render a string as a TOML basic string with the two escapes a policy field
/// can plausibly contain. Class names + addresses are simple tokens, but a
/// free-text-derived topic could in principle carry a quote/backslash.
fn toml_str(raw: &str) -> String {
    let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
#[path = "policy_synth_tests.rs"]
mod tests;
