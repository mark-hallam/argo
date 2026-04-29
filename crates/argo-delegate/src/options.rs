//! Options carried per-delegation.
use argo_types::{Budget, ProviderModel};
use serde::{Deserialize, Serialize};

/// Per-call options for a subagent delegation (§16.3 — Amendment 7).
///
/// `inherit_fallback` controls fallback-chain inheritance:
/// - `true` (default) + `fallback_override = None` → child uses
///   parent's chain.
/// - `Some(vec![...])` → override wins; an empty Vec disables fallback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateOptions {
    /// Self-contained task statement for the child.
    pub prompt: String,
    /// Toolset id (the parent's registry filtered by the toolset
    /// definition; toolset resolution is the agent loop's job).
    pub toolset: String,
    /// Hard budget for the child (a slice of the parent's budget).
    pub budget: Budget,
    /// Override the parent's model. `None` → use parent's.
    #[serde(default)]
    pub model: Option<ProviderModel>,
    /// Inherit the parent's fallback chain by default (Amendment 7).
    #[serde(default = "default_inherit_fallback")]
    pub inherit_fallback: bool,
    /// Explicit fallback override. `Some(empty)` disables fallback.
    #[serde(default)]
    pub fallback_override: Option<Vec<ProviderModel>>,
}

fn default_inherit_fallback() -> bool {
    true
}

impl DelegateOptions {
    /// Resolve the effective fallback chain given a parent's chain.
    ///
    /// - If `fallback_override` is set, it wins (even if empty).
    /// - Otherwise, if `inherit_fallback` is true, return the parent
    ///   chain.
    /// - Otherwise, return an empty Vec (no fallback).
    #[must_use]
    pub fn effective_fallback(&self, parent_chain: &[ProviderModel]) -> Vec<ProviderModel> {
        if let Some(ovr) = &self.fallback_override {
            return ovr.clone();
        }
        if self.inherit_fallback {
            parent_chain.to_vec()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pm(model: &str) -> ProviderModel {
        ProviderModel {
            provider: "openai".into(),
            model: model.into(),
        }
    }

    fn opts() -> DelegateOptions {
        DelegateOptions {
            prompt: "do x".into(),
            toolset: "default".into(),
            budget: Budget::default(),
            model: None,
            inherit_fallback: true,
            fallback_override: None,
        }
    }

    #[test]
    fn defaults_inherit_parent_chain() {
        let parent = vec![pm("gpt-4o-mini"), pm("gpt-3.5-turbo")];
        assert_eq!(opts().effective_fallback(&parent), parent);
    }

    #[test]
    fn override_wins_when_some() {
        let parent = vec![pm("gpt-4o-mini")];
        let mut o = opts();
        o.fallback_override = Some(vec![pm("o1-mini")]);
        let eff = o.effective_fallback(&parent);
        assert_eq!(eff.len(), 1);
        assert_eq!(eff[0].model, "o1-mini");
    }

    #[test]
    fn empty_override_disables_fallback() {
        let parent = vec![pm("gpt-4o-mini"), pm("gpt-3.5-turbo")];
        let mut o = opts();
        o.fallback_override = Some(vec![]);
        assert!(o.effective_fallback(&parent).is_empty());
    }

    #[test]
    fn inherit_false_yields_empty() {
        let parent = vec![pm("gpt-4o-mini")];
        let mut o = opts();
        o.inherit_fallback = false;
        assert!(o.effective_fallback(&parent).is_empty());
    }
}
