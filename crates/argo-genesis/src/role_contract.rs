//! Role contract — the operator-confirmable restatement of a teammate's
//! scope (DEC-025 #4 + #1, Genesis P4).
//!
//! Genesis P4's comprehension gate ([`crate::comprehension`]) proves the
//! teammate's *prompt* encodes its governance boundaries. This module adds
//! the other half DEC-025 #4 calls for: a **structured, human-readable
//! restatement of the role** — what it does, the scope/boundaries it works
//! within, which actions need approval, when it escalates, and (crucially)
//! **what it will NOT do** — that is surfaced for **operator confirmation
//! before the teammate activates** (DEC-025 #1, human-review-before-activation
//! ON by default).
//!
//! ## Truthfulness is the load-bearing property
//!
//! The contract is derived **only** from the teammate's *actual bound
//! capability profile* — the `[actions]` table (with each action's
//! `requires_approval` flag + risk + executor) and the `[mcp_servers]`
//! draft-integration list of the materialized manifest, the SAME source the
//! P2 capability fence derives from. It is NOT a free-text LLM restatement
//! that could understate the teammate's reach. [`verify_contract_matches`]
//! re-derives the contract from the manifest and asserts byte-equality, so a
//! benign-looking contract presented alongside a broader manifest is a
//! detectable forgery — the gate fails closed.
//!
//! ## The confirmation digest
//!
//! [`RoleContract::digest`] is a SHA-256 over the truthful capability
//! projection (the approval-gated set, the autonomous set, the draft
//! integrations, the escalation routing). The operator confirms a specific
//! contract by echoing this digest back at the hire seam; the gate confirms
//! activation only when the echoed digest equals the digest of the contract
//! freshly re-derived from the bound manifest. A teammate whose capabilities
//! changed since the operator looked has a different digest, so a stale
//! confirmation can never activate a mutated teammate.

use argo_vertical::{ActionRiskLevel, ExecutorDeclaration, VerticalManifest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One action in the teammate's bound vocabulary, projected for the operator
/// review. Every field is read straight from the manifest's `[actions]`
/// entry — never inferred — so the rendered contract cannot misstate the
/// action's real governance posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractAction {
    /// The manifest `[actions]` key (the action_type the runtime gates on).
    pub action: String,
    /// One-line human description from the manifest.
    pub description: String,
    /// `true` iff the manifest marks this action `requires_approval` — i.e.
    /// it cannot autonomously transition past `pending`.
    pub requires_approval: bool,
    /// The action's risk tier, lowercased (`low` / `medium` / `high`).
    pub risk: String,
    /// The executor kind the manifest binds (`human` / `mcp` /
    /// `computer_use` / `api`). At birth this is `human` for every generated
    /// action (DEC-025 #6, no live integration auto-bound).
    pub executor: String,
}

/// The structured, human-readable role contract the operator confirms before
/// a Genesis teammate activates. Built by [`RoleContract::derive_from_manifest`]
/// from the teammate's actual bound capability profile so every clause is
/// truthful.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleContract {
    /// The teammate's persona/role name (manifest `vertical.name`).
    pub role_name: String,
    /// What the teammate does, in plain language (manifest
    /// `vertical.description`, which carries the operator's charter).
    pub does: String,
    /// The full bound action vocabulary, each with its real governance
    /// posture. The teammate can do *only* these — anything else is blocked
    /// by the manifest vocabulary gate.
    pub actions: Vec<ContractAction>,
    /// Actions that ALWAYS require operator approval before they dispatch
    /// (the union of every `requires_approval` action + the manifest's
    /// `[escalation].human_approval_required`). Sorted, de-duplicated.
    pub approval_required: Vec<String>,
    /// Actions the teammate may perform autonomously (NOT approval-gated).
    /// Sorted. The honest converse of [`Self::approval_required`].
    pub autonomous: Vec<String>,
    /// The escalation trigger restated for the operator (e.g. "escalates to
    /// the operator after 3 consecutive failures, or when a task is outside
    /// this charter").
    pub escalation: String,
    /// What the teammate will NOT do — the explicit boundary list. Derived,
    /// not invented: integrations are draft-only (cannot transmit), no action
    /// outside the bound vocabulary, no autonomous dispatch of an
    /// approval-gated action.
    pub will_not: Vec<String>,
    /// Integrations the role *referenced* but that are NOT live at birth
    /// (manifest `[mcp_servers]` keys, recorded draft-only per DEC-025 #6).
    /// Surfaced so the operator sees the teammate physically cannot transmit
    /// through them until a deliberate post-hire grant.
    pub draft_integrations: Vec<String>,
}

impl RoleContract {
    /// Derive the role contract from a teammate's bound vertical manifest.
    ///
    /// Truthful by construction: every clause reads the manifest's actual
    /// `[actions]` / `[escalation]` / `[mcp_servers]` — the SAME source the
    /// P2 capability fence reads — so the contract reflects the teammate's
    /// real reach, never a more-benign restatement.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn demo() -> Result<(), argo_genesis::GenesisError> {
    /// let plan = argo_genesis::plan("be my marketing intern, run weekly campaigns", "maya")?;
    /// let contract = argo_genesis::RoleContract::derive_from_manifest(&plan.manifest);
    /// // The contract names every bound action and segregates the
    /// // approval-gated ones from the autonomous ones.
    /// assert!(!contract.actions.is_empty());
    /// assert_eq!(
    ///     contract.actions.len(),
    ///     contract.approval_required.len() + contract.autonomous.len(),
    /// );
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn derive_from_manifest(manifest: &VerticalManifest) -> Self {
        let mut actions: Vec<ContractAction> = Vec::new();
        if let Some(section) = manifest.actions.as_ref() {
            for (name, decl) in &section.actions {
                actions.push(ContractAction {
                    action: name.clone(),
                    description: decl.description.clone(),
                    requires_approval: decl.requires_approval,
                    risk: risk_str(decl.risk).to_string(),
                    executor: executor_kind(&decl.executor).to_string(),
                });
            }
        }

        // Approval-gated set = every action flagged requires_approval, UNION
        // the manifest's escalation.human_approval_required (an action can be
        // gated by either surface; the contract must reflect the union so it
        // can't understate what needs approval).
        let mut approval_required: Vec<String> = actions
            .iter()
            .filter(|a| a.requires_approval)
            .map(|a| a.action.clone())
            .collect();
        if let Some(esc) = manifest.escalation.as_ref() {
            approval_required.extend(esc.human_approval_required.iter().cloned());
        }
        approval_required.sort();
        approval_required.dedup();

        // Autonomous set = bound actions NOT in the approval-gated set.
        let mut autonomous: Vec<String> = actions
            .iter()
            .map(|a| a.action.clone())
            .filter(|name| !approval_required.contains(name))
            .collect();
        autonomous.sort();
        autonomous.dedup();

        let draft_integrations: Vec<String> = manifest
            .mcp_servers
            .as_ref()
            .map(|m| {
                let mut v: Vec<String> = m.servers.keys().cloned().collect();
                v.sort();
                v
            })
            .unwrap_or_default();

        let escalation = escalation_clause(manifest);
        let will_not = will_not_clauses(&approval_required, &draft_integrations);

        Self {
            role_name: manifest.vertical.name.clone(),
            does: manifest.vertical.description.clone(),
            actions,
            approval_required,
            autonomous,
            escalation,
            will_not,
            draft_integrations,
        }
    }

    /// A SHA-256 digest over the truthful capability projection — the
    /// approval-gated set, the autonomous set, the draft integrations, and
    /// the escalation routing. This is the value the operator echoes back to
    /// confirm a *specific* contract; the gate re-derives the contract from
    /// the bound manifest and compares digests, so a confirmation cannot bind
    /// to a teammate whose capabilities have since changed.
    ///
    /// The digest deliberately covers the *governance-bearing* projection
    /// (what needs approval, what runs free, what integrations exist, how it
    /// escalates) rather than cosmetic prose, so a description reword does not
    /// invalidate a confirmation while any capability change does.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn demo() -> Result<(), argo_genesis::GenesisError> {
    /// let plan = argo_genesis::plan("be my marketing intern, run weekly campaigns", "maya")?;
    /// let c = argo_genesis::RoleContract::derive_from_manifest(&plan.manifest);
    /// assert_eq!(c.digest().len(), 64, "sha256 hex");
    /// assert_eq!(c.digest(), c.digest(), "digest is deterministic");
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        // A stable, field-tagged projection. Newlines + tags keep distinct
        // fields from colliding (e.g. an action moving from autonomous to
        // approval changes the bytes).
        hasher.update(b"approval_required\n");
        for a in &self.approval_required {
            hasher.update(a.as_bytes());
            hasher.update(b"\n");
        }
        hasher.update(b"autonomous\n");
        for a in &self.autonomous {
            hasher.update(a.as_bytes());
            hasher.update(b"\n");
        }
        hasher.update(b"draft_integrations\n");
        for d in &self.draft_integrations {
            hasher.update(d.as_bytes());
            hasher.update(b"\n");
        }
        hasher.update(b"escalation\n");
        hasher.update(self.escalation.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Verify a presented [`RoleContract`] truthfully reflects a teammate's bound
/// capability profile: re-derive the contract from `manifest` and require the
/// two to be byte-identical.
///
/// This is the anti-forgery check. A teammate cannot present a benign
/// contract (e.g. one that hides an approval-gated `wire_funds` action, or
/// omits a draft integration) while its manifest binds broader capability —
/// the re-derived contract would differ and this returns `false`.
///
/// # Examples
///
/// ```
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let plan = argo_genesis::plan("be my marketing intern, run weekly campaigns", "maya")?;
/// let truthful = argo_genesis::RoleContract::derive_from_manifest(&plan.manifest);
/// assert!(argo_genesis::verify_contract_matches(&truthful, &plan.manifest));
///
/// // A forged contract that hides the approval-gated actions does NOT match.
/// let mut forged = truthful.clone();
/// forged.approval_required.clear();
/// assert!(!argo_genesis::verify_contract_matches(&forged, &plan.manifest));
/// # Ok(()) }
/// ```
#[must_use]
pub fn verify_contract_matches(contract: &RoleContract, manifest: &VerticalManifest) -> bool {
    &RoleContract::derive_from_manifest(manifest) == contract
}

/// Map the manifest risk enum to its lowercase wire token.
fn risk_str(risk: ActionRiskLevel) -> &'static str {
    match risk {
        ActionRiskLevel::Low => "low",
        ActionRiskLevel::Medium => "medium",
        ActionRiskLevel::High => "high",
    }
}

/// The executor kind string, regardless of legacy/tagged form.
fn executor_kind(executor: &ExecutorDeclaration) -> &str {
    executor.kind_str()
}

/// Restate the manifest's `[escalation]` routing in one human-readable line.
fn escalation_clause(manifest: &VerticalManifest) -> String {
    let esc = manifest.escalation.as_ref();
    let to = esc
        .and_then(|e| e.escalate_to.clone())
        .unwrap_or_else(|| "the operator".to_string());
    let after = esc.and_then(|e| e.escalate_after_failures);
    match after {
        Some(n) => format!(
            "Escalates to {to} after {n} consecutive failures, or when a task is \
             outside this charter or the teammate is uncertain."
        ),
        None => format!(
            "Escalates to {to} when a task is outside this charter or the \
             teammate is uncertain."
        ),
    }
}

/// Build the explicit "what this teammate will NOT do" boundary list. Every
/// clause is derived from real manifest facts, never invented.
fn will_not_clauses(approval_required: &[String], draft_integrations: &[String]) -> Vec<String> {
    let mut out = vec![
        "Perform any action outside its declared vocabulary — undeclared \
         actions are blocked at emission, not merely discouraged."
            .to_string(),
    ];
    if !approval_required.is_empty() {
        out.push(format!(
            "Autonomously dispatch an approval-gated action ({}). These always \
             wait for operator approval.",
            approval_required.join(", ")
        ));
    }
    if draft_integrations.is_empty() {
        out.push(
            "Transmit through any live external integration — it has none bound \
             at birth and can only draft and propose."
                .to_string(),
        );
    } else {
        out.push(format!(
            "Transmit through {} — these are draft-only at birth and require a \
             deliberate, audited operator grant before any live use.",
            draft_integrations.join(", ")
        ));
    }
    out
}

#[cfg(test)]
#[path = "role_contract_tests.rs"]
mod tests;
