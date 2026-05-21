//! Local development links for pack roots.

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Persistent dev-link store backed by `dev-links.toml`.
///
/// ```
/// use argo_vertical_packs::DevLinkStore;
/// let root = tempfile::tempdir().unwrap();
/// let pack = tempfile::tempdir().unwrap();
/// std::fs::write(pack.path().join("pack.toml"), "[pack]\nname='ap'\nversion='1'\n\n[[verticals]]\nid='ap'\npath='verticals/ap'\n").unwrap();
/// DevLinkStore::new(root.path()).set("ap", pack.path()).unwrap();
/// assert!(DevLinkStore::new(root.path()).resolve("ap").unwrap().is_some());
/// ```
pub struct DevLinkStore {
    path: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DevLinks {
    #[serde(default)]
    packs: BTreeMap<String, PathBuf>,
}

impl DevLinkStore {
    /// Create a store rooted under the pack cache directory.
    pub fn new(cache_root: &Path) -> Self {
        Self {
            path: cache_root.join("dev-links.toml"),
        }
    }

    /// Set a local path override for a pack name.
    pub fn set(&self, name: &str, path: &Path) -> ArgoResult<PathBuf> {
        if !path.join("pack.toml").is_file() {
            return Err(ArgoError::Schema(format!(
                "dev-link target {} must contain pack.toml",
                path.display()
            )));
        }
        let mut links = self.load()?;
        links.packs.insert(name.to_string(), path.to_path_buf());
        self.save(&links)?;
        Ok(path.to_path_buf())
    }

    /// Remove a local path override.
    pub fn remove(&self, name: &str) -> ArgoResult<bool> {
        let mut links = self.load()?;
        let removed = links.packs.remove(name).is_some();
        self.save(&links)?;
        Ok(removed)
    }

    /// Resolve the override path if one exists.
    pub fn resolve(&self, name: &str) -> ArgoResult<Option<PathBuf>> {
        Ok(self.load()?.packs.get(name).cloned())
    }

    fn load(&self) -> ArgoResult<DevLinks> {
        if !self.path.exists() {
            return Ok(DevLinks::default());
        }
        let body = std::fs::read_to_string(&self.path)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", self.path.display())))?;
        toml::from_str(&body)
            .map_err(|e| ArgoError::Schema(format!("parse {}: {e}", self.path.display())))
    }

    fn save(&self, links: &DevLinks) -> ArgoResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(links)
            .map_err(|e| ArgoError::Invariant(format!("serialize dev-links.toml: {e}")))?;
        std::fs::write(&self.path, body)
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", self.path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_link_overrides_cache_resolution() {
        let cache = tempfile::tempdir().unwrap();
        let pack = tempfile::tempdir().unwrap();
        std::fs::write(
            pack.path().join("pack.toml"),
            "[pack]\nname='x'\nversion='1'\n\n[[verticals]]\nid='x'\npath='verticals/x'\n",
        )
        .unwrap();
        let store = DevLinkStore::new(cache.path());
        store.set("x", pack.path()).unwrap();
        assert_eq!(store.resolve("x").unwrap().unwrap(), pack.path());
        assert!(store.remove("x").unwrap());
        assert!(store.resolve("x").unwrap().is_none());
    }

    /// Malformed `dev-links.toml` must surface the on-disk path so
    /// the operator can see which override file is broken.
    #[test]
    fn load_includes_file_path_on_parse_error() {
        let cache = tempfile::tempdir().unwrap();
        let dev_links_path = cache.path().join("dev-links.toml");
        std::fs::write(&dev_links_path, "this is = not [valid").unwrap();
        let store = DevLinkStore::new(cache.path());
        let err = store.resolve("x").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains(&dev_links_path.display().to_string()),
            "expected error to include {} — got {msg}",
            dev_links_path.display()
        );
    }
}
