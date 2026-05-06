//! Two-tier skill lookup: workspace → library (H-4b).
//!
//! The brief models skill discovery as two-layered:
//!
//! - **Workspace** — mutable, per-tenant. Skills the operator
//!   installs, the runtime learns, or the worker authors. Always
//!   consulted first.
//! - **Library** — read-mostly, cross-tenant. Skills shipped by
//!   the OCI marketplace (verified or community). Consulted only
//!   when the workspace doesn't have a match.
//!
//! [`LayeredLookup`] is the read-side wrapper around the two
//! [`SkillStore`] handles. It walks workspace first; falls back
//! to library when a name is not found in workspace. Trust-grade
//! filtering (H-4a's [`crate::trust::is_skill_eligible`]) is
//! applied at the [`LayeredLookup::list_for_action`] surface so
//! the runtime only sees skills it's allowed to surface for the
//! current action's risk class.
//!
//! Writes never reach the library — every mutating method on
//! [`SkillStore`] (`install`, `supersede`, `promote`, …) targets
//! the workspace store directly. The library is a read-only
//! lookup view from the runtime's perspective; how it gets
//! populated (OCI pulls, scheduled refresh, …) is out of scope
//! for H-4b.

use crate::skill::Skill;
use crate::store::SkillStore;
use crate::trust::is_skill_eligible;
use argo_types::ArgoResult;
use serde::{Deserialize, Serialize};

/// Which layer surfaced a skill in a layered lookup.
///
/// ```
/// use argo_skill::library::SkillLayer;
/// assert_ne!(SkillLayer::Workspace, SkillLayer::Library);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillLayer {
    /// Tenant-mutable skills installed in this workspace.
    Workspace,
    /// Read-mostly cross-tenant marketplace skills.
    Library,
}

/// A skill plus the layer it surfaced from.
///
/// The runtime needs the layer label so it can show the operator
/// where a learned-skill candidate would override a marketplace
/// skill (and vice versa) when the supersession review screen
/// surfaces both.
#[derive(Debug, Clone)]
pub struct LayeredSkill {
    /// The skill row.
    pub skill: Skill,
    /// Which layer it surfaced from.
    pub layer: SkillLayer,
}

/// Read-side wrapper that walks workspace → library on lookup.
///
/// Constructed via [`LayeredLookup::workspace_only`] when no
/// library is configured (the v0 / development shape) or via
/// [`LayeredLookup::with_library`] when both layers exist.
pub struct LayeredLookup<'a> {
    workspace: &'a SkillStore,
    library: Option<&'a SkillStore>,
}

impl<'a> LayeredLookup<'a> {
    /// Build a lookup that consults only the workspace. The
    /// fallback path is a no-op; behaves identically to calling
    /// the workspace `SkillStore` directly.
    #[must_use]
    pub fn workspace_only(workspace: &'a SkillStore) -> Self {
        Self {
            workspace,
            library: None,
        }
    }

    /// Build a lookup that walks workspace → library.
    #[must_use]
    pub fn with_library(workspace: &'a SkillStore, library: &'a SkillStore) -> Self {
        Self {
            workspace,
            library: Some(library),
        }
    }

    /// Find a skill by name, preferring the workspace.
    ///
    /// Returns `Ok(Some((layer, skill)))` when found, or
    /// `Ok(None)` if neither layer has an active match. Errors
    /// from the workspace lookup short-circuit; errors from the
    /// library lookup also short-circuit (a flaky library should
    /// surface as a real error rather than silently degrade to
    /// "not found", which would be a security smell).
    pub async fn find(&self, name: &str) -> ArgoResult<Option<LayeredSkill>> {
        if let Some(skill) = self.workspace.find_active_by_name(name).await? {
            return Ok(Some(LayeredSkill {
                skill,
                layer: SkillLayer::Workspace,
            }));
        }
        if let Some(library) = self.library {
            if let Some(skill) = library.find_active_by_name(name).await? {
                return Ok(Some(LayeredSkill {
                    skill,
                    layer: SkillLayer::Library,
                }));
            }
        }
        Ok(None)
    }

    /// List every active skill across both layers, filtered to
    /// those eligible for an action of the given `risk` level
    /// (see [`crate::trust::is_skill_eligible`]).
    ///
    /// Workspace results come first; library results are appended
    /// after. Duplicate names (workspace overriding library) are
    /// resolved in favour of the workspace — the library entry
    /// is dropped, mirroring the behaviour of [`Self::find`].
    pub async fn list_for_action(&self, risk: &str) -> ArgoResult<Vec<LayeredSkill>> {
        let mut out = Vec::new();
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

        for skill in self.workspace.list_active().await? {
            if !is_skill_eligible(&skill.frontmatter.provenance, risk) {
                continue;
            }
            seen_names.insert(skill.frontmatter.name.clone());
            out.push(LayeredSkill {
                skill,
                layer: SkillLayer::Workspace,
            });
        }

        if let Some(library) = self.library {
            for skill in library.list_active().await? {
                if seen_names.contains(&skill.frontmatter.name) {
                    continue;
                }
                if !is_skill_eligible(&skill.frontmatter.provenance, risk) {
                    continue;
                }
                out.push(LayeredSkill {
                    skill,
                    layer: SkillLayer::Library,
                });
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SkillStore;
    use argo_store::SessionStore;
    use std::sync::Arc;

    fn skill_md(name: &str, source: &str) -> String {
        format!(
            "---\n\
             name: {name}\n\
             version: 0.1.0\n\
             description: a test skill\n\
             provenance:\n  \
             source: {source}\n  \
             hash: sha256-deadbeef\n\
             ---\n\
             body of {name}\n"
        )
    }

    async fn store() -> SkillStore {
        let session = SessionStore::open(":memory:")
            .await
            .expect("session store");
        session.migrate().await.expect("migrate");
        SkillStore::new(Arc::new(session))
    }

    #[tokio::test]
    async fn find_returns_workspace_when_present() {
        let ws = store().await;
        ws.install(&skill_md("alpha", "user"), None)
            .await
            .expect("install");
        let lookup = LayeredLookup::workspace_only(&ws);
        let found = lookup.find("alpha").await.expect("find").expect("present");
        assert_eq!(found.layer, SkillLayer::Workspace);
        assert_eq!(found.skill.frontmatter.name, "alpha");
    }

    #[tokio::test]
    async fn find_falls_back_to_library() {
        let ws = store().await;
        let lib = store().await;
        lib.install(&skill_md("alpha", "published:verified:vendor"), None)
            .await
            .expect("install lib");
        let lookup = LayeredLookup::with_library(&ws, &lib);
        let found = lookup.find("alpha").await.expect("find").expect("present");
        assert_eq!(found.layer, SkillLayer::Library);
    }

    #[tokio::test]
    async fn find_workspace_overrides_library() {
        let ws = store().await;
        let lib = store().await;
        ws.install(&skill_md("alpha", "user"), None)
            .await
            .expect("install ws");
        lib.install(&skill_md("alpha", "published:verified:vendor"), None)
            .await
            .expect("install lib");
        let lookup = LayeredLookup::with_library(&ws, &lib);
        let found = lookup.find("alpha").await.expect("find").expect("present");
        assert_eq!(found.layer, SkillLayer::Workspace);
    }

    #[tokio::test]
    async fn find_returns_none_when_neither_layer_has_match() {
        let ws = store().await;
        let lib = store().await;
        let lookup = LayeredLookup::with_library(&ws, &lib);
        let found = lookup.find("missing").await.expect("find");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn list_for_action_filters_by_trust() {
        let ws = store().await;
        // workspace has a learned (lowest trust) skill — eligible
        // for low risk only.
        ws.install(&skill_md("learned1", "learned"), None)
            .await
            .expect("install learned");
        // workspace has a tenant skill — eligible for low+medium.
        ws.install(&skill_md("user1", "user"), None)
            .await
            .expect("install user");
        // workspace has a builtin skill — eligible everywhere.
        ws.install(&skill_md("builtin1", "builtin"), None)
            .await
            .expect("install builtin");

        let lookup = LayeredLookup::workspace_only(&ws);

        let low = lookup.list_for_action("low").await.expect("low");
        assert_eq!(low.len(), 3, "low-risk should see all three");

        let medium = lookup.list_for_action("medium").await.expect("medium");
        let medium_names: std::collections::HashSet<_> = medium
            .iter()
            .map(|s| s.skill.frontmatter.name.clone())
            .collect();
        assert!(medium_names.contains("user1"));
        assert!(medium_names.contains("builtin1"));
        assert!(
            !medium_names.contains("learned1"),
            "learned skill must NOT surface for medium-risk; saw {medium_names:?}"
        );

        let high = lookup.list_for_action("high").await.expect("high");
        let high_names: std::collections::HashSet<_> = high
            .iter()
            .map(|s| s.skill.frontmatter.name.clone())
            .collect();
        assert_eq!(high_names.len(), 1);
        assert!(high_names.contains("builtin1"));
    }

    #[tokio::test]
    async fn list_for_action_dedupes_workspace_overriding_library() {
        let ws = store().await;
        let lib = store().await;
        ws.install(&skill_md("conflict", "user"), None)
            .await
            .expect("install ws");
        lib.install(&skill_md("conflict", "published:verified:vendor"), None)
            .await
            .expect("install lib");

        let lookup = LayeredLookup::with_library(&ws, &lib);
        let listed = lookup.list_for_action("low").await.expect("list");

        // Both layers have a "conflict" skill but only the
        // workspace one surfaces.
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].layer, SkillLayer::Workspace);
    }

    #[tokio::test]
    async fn list_for_action_appends_library_after_workspace() {
        let ws = store().await;
        let lib = store().await;
        ws.install(&skill_md("ws_only", "user"), None)
            .await
            .expect("install ws");
        lib.install(&skill_md("lib_only", "published:verified:vendor"), None)
            .await
            .expect("install lib");

        let lookup = LayeredLookup::with_library(&ws, &lib);
        let listed = lookup.list_for_action("medium").await.expect("list");

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].layer, SkillLayer::Workspace);
        assert_eq!(listed[0].skill.frontmatter.name, "ws_only");
        assert_eq!(listed[1].layer, SkillLayer::Library);
        assert_eq!(listed[1].skill.frontmatter.name, "lib_only");
    }

    #[tokio::test]
    async fn workspace_only_constructor_skips_library_walk() {
        let ws = store().await;
        ws.install(&skill_md("alpha", "user"), None)
            .await
            .expect("install");
        let lookup = LayeredLookup::workspace_only(&ws);
        let listed = lookup.list_for_action("medium").await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].layer, SkillLayer::Workspace);
    }
}
