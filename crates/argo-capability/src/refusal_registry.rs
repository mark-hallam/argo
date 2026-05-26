//! Refusal-template lookup table for truth-boundary enforcement.
//!
//! P-01 defines the [`crate::truth_boundary::RefusalTemplate`] data
//! shape. Runtime enforcement needs a small registry so a
//! `refusal_template_id` from a policy can resolve to the wording
//! variants that are safe to show to the user.

use crate::truth_boundary::RefusalTemplate;
use std::collections::HashMap;

/// In-memory lookup for truth-boundary refusal templates.
#[derive(Debug, Clone, Default)]
pub struct RefusalTemplateRegistry {
    templates: HashMap<String, RefusalTemplate>,
}

impl RefusalTemplateRegistry {
    /// Build a registry from loaded templates. Later entries with the
    /// same id replace earlier entries.
    #[must_use]
    pub fn new(templates: Vec<RefusalTemplate>) -> Self {
        let templates = templates
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect::<HashMap<_, _>>();
        Self { templates }
    }

    /// Empty registry for sessions with no active truth-boundary policy.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return a template by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&RefusalTemplate> {
        self.templates.get(id)
    }

    /// Number of registered templates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Whether the registry contains no templates.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}
