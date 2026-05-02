//! In-memory store of loaded verticals. The loader populates a
//! [`Registry`] from the on-disk `verticals/` directory; the rest
//! of argo (CLI, gateway, session creation) reads from it.
//!
//! The registry is intentionally read-mostly. Insertion fails on
//! duplicate IDs — never silent-overwrite — so two verticals
//! claiming the same `id` are caught at startup, not at first
//! request.
use crate::manifest::VerticalManifest;
use argo_types::{ArgoError, ArgoResult};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// One vertical loaded from disk: its parsed manifest plus the
/// directory it came from (used to resolve relative file refs at
/// runtime — e.g., reading `[role].system_prompt` markdown).
#[derive(Debug, Clone)]
pub struct LoadedVertical {
    pub manifest: VerticalManifest,
    pub root: PathBuf,
}

/// In-memory map of `vertical.id -> LoadedVertical`. BTreeMap so
/// `list()` returns deterministic order (handy for tests + JSON
/// output).
#[derive(Debug, Clone, Default)]
pub struct Registry {
    entries: BTreeMap<String, LoadedVertical>,
}

impl Registry {
    /// Empty registry. Most callers want [`Registry::default()`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert. Returns an error if the manifest's `id` is already
    /// present. The caller decides whether to halt or accumulate.
    pub fn insert(&mut self, loaded: LoadedVertical) -> ArgoResult<()> {
        let id = loaded.manifest.vertical.id.clone();
        if self.entries.contains_key(&id) {
            return Err(ArgoError::Schema(format!(
                "vertical id '{id}' is already registered (loaded from {})",
                self.entries
                    .get(&id)
                    .map(|e| e.root.display().to_string())
                    .unwrap_or_default()
            )));
        }
        self.entries.insert(id, loaded);
        Ok(())
    }

    /// Lookup by id. Returns `None` if not present.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&LoadedVertical> {
        self.entries.get(id)
    }

    /// Iterate over loaded verticals in deterministic id order.
    pub fn list(&self) -> impl Iterator<Item = (&str, &LoadedVertical)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of loaded verticals.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{VerticalIdentity, VerticalManifest};

    fn fixture(id: &str) -> LoadedVertical {
        LoadedVertical {
            manifest: VerticalManifest {
                schema_version: "1".into(),
                vertical: VerticalIdentity {
                    id: id.into(),
                    name: format!("Vertical {id}"),
                    description: String::new(),
                    version: "0.1.0".into(),
                    authors: vec![],
                    tags: vec![],
                },
                ..Default::default()
            },
            root: PathBuf::from(format!("/fake/{id}")),
        }
    }

    #[test]
    fn insert_and_get() {
        let mut r = Registry::new();
        r.insert(fixture("alpha")).unwrap();
        r.insert(fixture("beta")).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r.get("alpha").is_some());
        assert!(r.get("missing").is_none());
    }

    #[test]
    fn duplicate_id_fails() {
        let mut r = Registry::new();
        r.insert(fixture("alpha")).unwrap();
        let err = r.insert(fixture("alpha")).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("already registered"), "got: {msg}");
        assert!(msg.contains("alpha"), "got: {msg}");
        // Original entry untouched.
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn list_is_deterministic_order() {
        let mut r = Registry::new();
        r.insert(fixture("zeta")).unwrap();
        r.insert(fixture("alpha")).unwrap();
        r.insert(fixture("beta")).unwrap();
        let ids: Vec<&str> = r.list().map(|(id, _)| id).collect();
        assert_eq!(ids, vec!["alpha", "beta", "zeta"]);
    }

    #[test]
    fn empty_helpers() {
        let r = Registry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }
}
