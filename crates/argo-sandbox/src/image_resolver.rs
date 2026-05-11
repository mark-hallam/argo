//! `image_for_executor` — lookup the configured OCI image for a named
//! executor (e.g. `"browser"` → `"skycot/marketplace-browser:latest"`).
//!
//! The resolver is the single seam between `argo-config`'s
//! `[sandbox.image]` table and whichever sandbox backend launches a
//! container. Backends that need an image consult the resolver
//! before falling back to their hard-coded default; callers that
//! don't care can ignore the resolver entirely.
//!
//! ```
//! use argo_sandbox::ImageResolver;
//! use std::collections::BTreeMap;
//! let mut map = BTreeMap::new();
//! map.insert("browser".to_string(), "skycot/marketplace-browser:latest".to_string());
//! let resolver = ImageResolver::from_map(map);
//! assert_eq!(resolver.image_for_executor("browser"),
//!     Some("skycot/marketplace-browser:latest"));
//! assert_eq!(resolver.image_for_executor("missing"), None);
//! ```
use std::collections::BTreeMap;

/// Resolves executor names → OCI image references. Wraps a
/// `BTreeMap` so iteration order is deterministic for snapshot
/// tests, and so the type signature is independent of the config
/// crate (callers can build a resolver from any source — config
/// file, env override, tenant metadata).
#[derive(Debug, Clone, Default)]
pub struct ImageResolver {
    map: BTreeMap<String, String>,
}

impl ImageResolver {
    /// Build a resolver from a name → image map (e.g. the parsed
    /// `[sandbox.image]` table from `argo-config`).
    #[must_use]
    pub fn from_map(map: BTreeMap<String, String>) -> Self {
        Self { map }
    }

    /// Lookup the image configured for `executor`. Returns `None`
    /// when the executor has no override and the backend should fall
    /// back to its built-in default.
    #[must_use]
    pub fn image_for_executor(&self, executor: &str) -> Option<&str> {
        self.map.get(executor).map(String::as_str)
    }

    /// Whether any executor has an override configured. Backends use
    /// this to short-circuit when no `[sandbox.image]` table is set
    /// at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Number of configured overrides. Primarily for diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
#[path = "image_resolver_tests.rs"]
mod image_resolver_tests;
