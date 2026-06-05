//! Tenant-segmented on-disk persistence for an in-progress conversational
//! intake (DEC-039 "Talk it through").
//!
//! A conversation spans many HTTP turns, so its captured slots + transcript must
//! survive between calls. This reuses Genesis's existing **slot-file** model
//! (disk, NOT a new DB table — mirrors [`crate::sop_store`] /
//! [`crate::hire_gate_disk`]): each conversation is one JSON file under a
//! **tenant-segmented** directory.
//!
//! ## Tenant isolation (the load-bearing guarantee)
//!
//! The path is `<root>/.genesis-intake/<tenant_slug>/<conversation_id>.json`,
//! where BOTH the tenant slug and the conversation id are sanitised to a safe
//! `[a-z0-9-]` segment (so a hostile `tenant_id`/`conversation_id` like
//! `../other` can never traverse out of its own tenant subtree). A different
//! tenant resolves a different `<tenant_slug>` directory and therefore can never
//! read — or overwrite — another tenant's intake. The leading dot keeps the tree
//! out of the global vertical listing, exactly like the generated-vertical tree.

use std::path::{Path, PathBuf};

use crate::intake_machine::IntakeState;

/// The dot-prefixed root subdirectory holding all per-tenant intake trees.
const INTAKE_ROOT: &str = ".genesis-intake";

/// Disk-backed store for in-progress intake conversations, rooted at a base
/// directory the gateway supplies (typically the verticals root).
#[derive(Debug, Clone)]
pub struct IntakeStore {
    root: PathBuf,
}

impl IntakeStore {
    /// Build a store rooted at `root`. The `.genesis-intake/` tree is created
    /// lazily on the first [`Self::save`].
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The tenant-segmented directory holding `tenant_id`'s conversations.
    fn tenant_dir(&self, tenant_id: &str) -> PathBuf {
        self.root.join(INTAKE_ROOT).join(sanitize_segment(tenant_id))
    }

    /// The JSON file for one conversation under one tenant. Both segments are
    /// sanitised, so a hostile id cannot escape the tenant subtree.
    fn conversation_path(&self, tenant_id: &str, conversation_id: &str) -> PathBuf {
        self.tenant_dir(tenant_id)
            .join(format!("{}.json", sanitize_segment(conversation_id)))
    }

    /// Load a previously-saved conversation, or `None` when none exists for this
    /// `(tenant_id, conversation_id)` pair (a brand-new conversation, or a probe
    /// from a DIFFERENT tenant — which resolves a different directory and so
    /// always reads `None`). An unreadable / corrupt file also yields `None`, so
    /// a bad file restarts the conversation rather than crashing the turn.
    #[must_use]
    pub fn load(&self, tenant_id: &str, conversation_id: &str) -> Option<IntakeState> {
        let path = self.conversation_path(tenant_id, conversation_id);
        let body = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&body).ok()
    }

    /// Persist a conversation's state. Idempotent-by-overwrite within the
    /// `(tenant_id, conversation_id)` pair.
    ///
    /// # Errors
    ///
    /// [`crate::GenesisError::Io`] on a directory-create / write failure — a
    /// conversation that cannot be persisted must fail the turn loudly, not
    /// silently drop the operator's answers.
    pub fn save(
        &self,
        tenant_id: &str,
        conversation_id: &str,
        state: &IntakeState,
    ) -> Result<(), crate::GenesisError> {
        let dir = self.tenant_dir(tenant_id);
        std::fs::create_dir_all(&dir).map_err(|e| crate::GenesisError::Io {
            what: "intake.dir.create",
            source: e,
        })?;
        let body = serde_json::to_string_pretty(state).map_err(|e| crate::GenesisError::Json {
            what: "intake.serialise",
            source: e,
        })?;
        let path = self.conversation_path(tenant_id, conversation_id);
        std::fs::write(path, body).map_err(|e| crate::GenesisError::Io {
            what: "intake.write",
            source: e,
        })
    }

    /// The base directory this store writes under (for diagnostics / tests).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Reduce an arbitrary id to ONE safe path segment: lowercase ASCII
/// alphanumerics + `-`, every other run collapsed to a single `-`, trimmed. A
/// segment that sanitises to empty falls back to `"anonymous"`. This is what
/// makes a hostile `tenant_id`/`conversation_id` (e.g. `../escape`,
/// `a/b/../../c`) unable to traverse out of its tenant subtree — every `/`, `.`,
/// and `\` becomes a `-`.
fn sanitize_segment(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_dash = true; // suppress a leading dash
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "anonymous".to_string()
    } else {
        out
    }
}

#[cfg(test)]
#[path = "intake_store_tests.rs"]
mod tests;
