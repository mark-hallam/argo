//! Workspace handoff policy (Stream `claude-worker-handoff-policy`).
//!
//! Workers either escalate too eagerly (every aging invoice goes to
//! the senior accountant) or too rarely (entry-level worker churns
//! retries instead of asking for help). Routing weights answer
//! "which worker?", not "should we hand off at all?". This module
//! loads `.skycot/handoff-policy.toml` from the workspace root and
//! answers the second question via a first-match-wins rule list.
//!
//! Surface: `argo handoff-policy {show [--explain <action>], validate
//! [<file>]}`. Wired into `argo-orchestrator::HandoffWalker` as a
//! [`HandoffPolicyHook`] impl that re-routes only when a rule fires
//! with [`Decision::EscalateTo`]; everything else falls through to
//! the existing routing weights, preserving pre-policy behaviour.

use crate::cli::cli_handoff_cmds::HandoffPolicyCmd;
use argo_identity::WorkerId;
use argo_orchestrator::HandoffPolicyHook;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default workspace-relative location of the policy file.
pub const POLICY_PATH: &str = ".skycot/handoff-policy.toml";

/// Top-level parsed policy.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct HandoffPolicy {
    /// `[defaults]` table — workspace-wide tunables (informational
    /// today; surfaced via `argo handoff-policy show`).
    #[serde(default)]
    pub defaults: Defaults,
    /// Ordered `[[rules]]` list. First matching rule wins.
    #[serde(default)]
    pub rules: Vec<Rule>,
}

/// `[defaults]` table.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Defaults {
    /// Inline retry budget before escalation auto-fires.
    pub inline_max_retries: Option<u32>,
    /// Inline latency budget (seconds) before escalation auto-fires.
    pub inline_max_latency_seconds: Option<u32>,
    /// Confidence floor below which escalation auto-fires.
    pub escalate_on_low_confidence: Option<f32>,
}

/// Single `[[rules]]` entry.
#[derive(Debug, Deserialize, Serialize)]
pub struct Rule {
    /// `match = { ... }` predicate.
    #[serde(rename = "match")]
    pub match_spec: MatchSpec,
    /// `policy = "..."` decision tag.
    pub policy: PolicyKind,
}

/// Predicate against an incoming action. All present fields must
/// match (logical AND); absent fields are ignored.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MatchSpec {
    /// Action kind (`"*"` = match-all wildcard).
    pub action_kind: Option<String>,
    /// Strict less-than confidence threshold.
    pub confidence_lt: Option<f32>,
    /// Greater-than-or-equal retry-count threshold.
    pub retry_count_gte: Option<u32>,
}

/// String-tagged decision as it appears in TOML.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyKind {
    /// Handle inline; do not escalate.
    Inline,
    /// Escalate via the existing routing chain.
    Escalate,
    /// Alias for `Escalate` — surfaces operator intent in the file.
    AlwaysEscalate,
    /// Alias for `Escalate` that signals "to a more senior peer".
    /// Walker re-routing requires a concrete worker id, which the
    /// rules DSL does not yet encode, so this resolves to
    /// `Decision::Escalate` (chain-driven) today.
    EscalateToSenior,
}

/// Resolved decision the orchestrator hook acts on. Mirrors the
/// brief's `Decision::{ Inline, Escalate, EscalateTo }` shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Do not escalate; let inline retry/latency budget run.
    Inline,
    /// Escalate via the existing routing weights / chain.
    Escalate,
    /// Re-route the next handoff to a specific worker. Reserved
    /// for a future rule shape that names a worker id; the
    /// `HandoffPolicyHook` impl already routes this variant when
    /// produced.
    #[allow(dead_code)]
    EscalateTo(WorkerId),
}

/// Context the policy evaluates against. The walker only hands
/// the hook `expected_outcome`, so the integration site populates
/// just `action_kind`; richer call sites (CLI `--explain`) can
/// fill in the optional fields.
#[derive(Debug, Default)]
pub struct ActionContext {
    /// Action kind / expected outcome string.
    pub action_kind: String,
    /// Optional model confidence in `[0, 1]`.
    pub confidence: Option<f32>,
    /// Number of inline retries already consumed.
    pub retry_count: u32,
}

impl HandoffPolicy {
    /// Parse from a TOML string. Empty input → empty policy.
    pub fn from_toml(s: &str) -> ArgoResult<Self> {
        toml::from_str(s)
            .map_err(|e| ArgoError::Provider(format!("handoff-policy parse: {e}")))
    }

    /// Load from `.skycot/handoff-policy.toml` under `root`. A
    /// missing file resolves to an empty policy (graceful absence,
    /// per the acceptance criteria — no-policy = current routing
    /// weights behaviour).
    pub fn load_from_workspace(root: &Path) -> ArgoResult<Self> {
        let path = root.join(POLICY_PATH);
        if !path.exists() {
            return Ok(Self::default());
        }
        let body = std::fs::read_to_string(&path)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
        Self::from_toml(&body)
    }

    /// First-match-wins evaluation. Returns the matched rule's
    /// index plus resolved [`Decision`]; `None` falls through to
    /// existing routing weights.
    pub fn evaluate(&self, ctx: &ActionContext) -> Option<(usize, Decision)> {
        for (idx, rule) in self.rules.iter().enumerate() {
            if rule.match_spec.matches(ctx) {
                return Some((idx, rule.policy.resolve()));
            }
        }
        None
    }
}

impl MatchSpec {
    fn matches(&self, ctx: &ActionContext) -> bool {
        if let Some(kind) = &self.action_kind {
            if kind != "*" && kind != &ctx.action_kind {
                return false;
            }
        }
        if let Some(threshold) = self.confidence_lt {
            match ctx.confidence {
                Some(c) if c < threshold => {}
                _ => return false,
            }
        }
        if let Some(threshold) = self.retry_count_gte {
            if ctx.retry_count < threshold {
                return false;
            }
        }
        true
    }
}

impl PolicyKind {
    fn resolve(&self) -> Decision {
        match self {
            Self::Inline => Decision::Inline,
            Self::Escalate | Self::AlwaysEscalate | Self::EscalateToSenior => Decision::Escalate,
        }
    }
}

/// [`HandoffPolicyHook`] impl: re-routes only when a rule fires
/// with [`Decision::EscalateTo`]; everything else returns `None`
/// so the walker falls back to its existing routing engine.
///
/// Public for direct construction by the gateway sweeper wire-up
/// (follow-up brief), and consumed by the orchestrator integration
/// test today.
#[allow(dead_code)]
pub struct PolicyHook {
    policy: HandoffPolicy,
}

#[allow(dead_code)]
impl PolicyHook {
    /// Wrap a parsed policy as a hook the walker can consult.
    #[must_use]
    pub fn new(policy: HandoffPolicy) -> Self {
        Self { policy }
    }
}

impl HandoffPolicyHook for PolicyHook {
    fn override_target(&self, expected_outcome: &str) -> Option<WorkerId> {
        let ctx = ActionContext {
            action_kind: expected_outcome.to_string(),
            ..Default::default()
        };
        match self.policy.evaluate(&ctx)? {
            (_, Decision::EscalateTo(w)) => Some(w),
            (_, Decision::Inline | Decision::Escalate) => None,
        }
    }
}

// ---------- CLI dispatch ----------

/// Top-level `argo handoff-policy ...` dispatch.
pub async fn run(cmd: HandoffPolicyCmd) -> ArgoResult<()> {
    match cmd {
        HandoffPolicyCmd::Show { explain, json } => show(explain.as_deref(), json),
        HandoffPolicyCmd::Validate { path, json } => validate(path.as_deref(), json),
    }
}

fn cwd() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))
}

fn show(explain: Option<&str>, json: bool) -> ArgoResult<()> {
    let policy = HandoffPolicy::load_from_workspace(&cwd()?)?;
    let trace = explain.map(|kind| {
        let ctx = ActionContext {
            action_kind: kind.to_string(),
            ..Default::default()
        };
        match policy.evaluate(&ctx) {
            Some((idx, d)) => format!("rule #{idx} matched → {d:?}"),
            None => "no match — using routing weights".into(),
        }
    });
    if json {
        let payload = serde_json::json!({
            "policy": &policy,
            "explain": trace,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        println!("rules: {}", policy.rules.len());
        for (i, r) in policy.rules.iter().enumerate() {
            println!("  #{i}: {:?} → {:?}", r.match_spec, r.policy);
        }
        if let Some(t) = trace {
            println!("--explain: {t}");
        }
    }
    Ok(())
}

fn validate(path: Option<&Path>, json: bool) -> ArgoResult<()> {
    let target = match path {
        Some(p) => p.to_path_buf(),
        None => cwd()?.join(POLICY_PATH),
    };
    let outcome: Result<usize, String> = if !target.exists() {
        Err(format!("file not found: {}", target.display()))
    } else {
        match std::fs::read_to_string(&target) {
            Err(e) => Err(format!("read {}: {e}", target.display())),
            Ok(body) => HandoffPolicy::from_toml(&body)
                .map(|p| p.rules.len())
                .map_err(|e| e.to_string()),
        }
    };
    match outcome {
        Ok(n) => {
            if json {
                println!("{{\"ok\":true,\"rules\":{n}}}");
            } else {
                println!("ok — {n} rule(s)");
            }
            Ok(())
        }
        Err(msg) => {
            if json {
                let escaped = serde_json::Value::String(msg.clone());
                println!("{{\"ok\":false,\"error\":{escaped}}}");
            } else {
                eprintln!("invalid: {msg}");
            }
            Err(ArgoError::Provider(msg))
        }
    }
}

#[cfg(test)]
#[path = "handoff_policy_tests.rs"]
mod tests;
