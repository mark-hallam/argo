//! Workspace ↔ library skill-name collision detection (Stream
//! `claude-skill-conflict-resolution`).
//!
//! [`SkillStore::list_active`] surfaces every active skill in
//! one layer. The H-4b `LayeredLookup` picks workspace > library
//! on collision but doesn't expose which collisions exist for an
//! operator to review. This module compares the two layers'
//! active-skill lists by name and emits one [`SkillConflict`]
//! per shared name.
//!
//! Resolution is a separate two-step:
//! 1. Detect via [`detect_conflicts`].
//! 2. Resolve by flipping the loser's lifecycle to `Archived`
//!    via the existing
//!    [`SkillStore::transition_lifecycle`] surface.
use crate::library::SkillLayer;
use crate::skill::Skill;
use crate::store::SkillStore;
use argo_types::ArgoResult;

/// One workspace ↔ library name collision.
#[derive(Debug, Clone)]
pub struct SkillConflict {
    /// The colliding skill name.
    pub name: String,
    /// Workspace-side row.
    pub workspace: Skill,
    /// Library-side row.
    pub library: Skill,
}

impl SkillConflict {
    /// Pick the row from the layer the operator wants to keep.
    /// The OTHER row is the one that will be archived.
    #[must_use]
    pub fn keeper(&self, keep: SkillLayer) -> &Skill {
        match keep {
            SkillLayer::Workspace => &self.workspace,
            SkillLayer::Library => &self.library,
        }
    }

    /// The row that should be archived to resolve the conflict
    /// in favour of `keep`.
    #[must_use]
    pub fn loser(&self, keep: SkillLayer) -> &Skill {
        match keep {
            SkillLayer::Workspace => &self.library,
            SkillLayer::Library => &self.workspace,
        }
    }
}

/// Walk both layers' active-skill lists and emit one
/// [`SkillConflict`] per shared name. Returns an empty vec when
/// the layers don't overlap.
pub async fn detect_conflicts(
    workspace: &SkillStore,
    library: &SkillStore,
) -> ArgoResult<Vec<SkillConflict>> {
    use crate::skill::LifecycleState;
    let ws_skills = workspace.list_active().await?;
    let lib_skills = library.list_active().await?;
    let mut conflicts = Vec::new();
    for ws in ws_skills
        .iter()
        .filter(|s| matches!(s.lifecycle_state, LifecycleState::Active))
    {
        if let Some(lib) = lib_skills
            .iter()
            .filter(|s| matches!(s.lifecycle_state, LifecycleState::Active))
            .find(|s| s.frontmatter.name == ws.frontmatter.name)
        {
            conflicts.push(SkillConflict {
                name: ws.frontmatter.name.clone(),
                workspace: ws.clone(),
                library: lib.clone(),
            });
        }
    }
    conflicts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(conflicts)
}

/// Render a unified diff between the two layers' bodies for the
/// `argo skill conflicts show <name>` view.
#[must_use]
pub fn render_diff(conflict: &SkillConflict) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "--- workspace/{}@{}\n+++ library/{}@{}\n",
        conflict.workspace.frontmatter.name,
        conflict.workspace.frontmatter.version,
        conflict.library.frontmatter.name,
        conflict.library.frontmatter.version,
    ));
    let ws_lines: Vec<&str> = conflict.workspace.body.lines().collect();
    let lib_lines: Vec<&str> = conflict.library.body.lines().collect();
    let max = ws_lines.len().max(lib_lines.len());
    for i in 0..max {
        match (ws_lines.get(i), lib_lines.get(i)) {
            (Some(ws), Some(lib)) if ws == lib => {
                out.push_str(&format!(" {ws}\n"));
            }
            (Some(ws), Some(lib)) => {
                out.push_str(&format!("-{ws}\n"));
                out.push_str(&format!("+{lib}\n"));
            }
            (Some(ws), None) => out.push_str(&format!("-{ws}\n")),
            (None, Some(lib)) => out.push_str(&format!("+{lib}\n")),
            (None, None) => {}
        }
    }
    out
}

/// Parse a `--keep` flag value into a `SkillLayer`. Accepts
/// `workspace` / `library` (case-insensitive). Returns `None`
/// for any other input so callers can surface a helpful error.
#[must_use]
pub fn parse_keep_flag(s: &str) -> Option<SkillLayer> {
    match s.to_lowercase().as_str() {
        "workspace" | "ws" => Some(SkillLayer::Workspace),
        "library" | "lib" => Some(SkillLayer::Library),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{Provenance, Skill, SkillFrontmatter};
    use argo_types::{CapabilitySet, Lock, SkillId};
    use chrono::Utc;

    fn skill(name: &str, version: &str, body: &str) -> Skill {
        Skill {
            id: SkillId::new(),
            frontmatter: SkillFrontmatter {
                name: name.to_string(),
                version: version.to_string(),
                description: format!("{name} description"),
                capabilities: CapabilitySet::default(),
                provenance: Provenance {
                    source: "test".into(),
                    hash: "sha256-stub".into(),
                },
                lock: Lock::None,
            },
            body: body.to_string(),
            hash: [0u8; 32],
            learned_in: None,
            superseded_by: None,
            created_at: Utc::now(),
            vertical_id: None,
            lifecycle_state: crate::skill::LifecycleState::Active,
        }
    }

    #[test]
    fn parse_keep_flag_accepts_aliases() {
        assert_eq!(parse_keep_flag("workspace"), Some(SkillLayer::Workspace));
        assert_eq!(parse_keep_flag("WS"), Some(SkillLayer::Workspace));
        assert_eq!(parse_keep_flag("library"), Some(SkillLayer::Library));
        assert_eq!(parse_keep_flag("Lib"), Some(SkillLayer::Library));
        assert_eq!(parse_keep_flag(""), None);
        assert_eq!(parse_keep_flag("garbage"), None);
    }

    #[test]
    fn keeper_and_loser_pivot_on_keep() {
        let c = SkillConflict {
            name: "summariser".into(),
            workspace: skill("summariser", "0.4", "ws body"),
            library: skill("summariser", "0.3", "lib body"),
        };
        assert_eq!(c.keeper(SkillLayer::Workspace).body, "ws body");
        assert_eq!(c.loser(SkillLayer::Workspace).body, "lib body");
        assert_eq!(c.keeper(SkillLayer::Library).body, "lib body");
        assert_eq!(c.loser(SkillLayer::Library).body, "ws body");
    }

    #[test]
    fn render_diff_marks_changed_lines() {
        let c = SkillConflict {
            name: "x".into(),
            workspace: skill("x", "0.1", "alpha\nshared\nbeta"),
            library: skill("x", "0.2", "alpha\nshared\ngamma"),
        };
        let d = render_diff(&c);
        assert!(d.starts_with("--- workspace/x@0.1"));
        assert!(d.contains(" alpha"));
        assert!(d.contains(" shared"));
        assert!(d.contains("-beta"));
        assert!(d.contains("+gamma"));
    }

    #[test]
    fn render_diff_handles_unequal_line_counts() {
        let c = SkillConflict {
            name: "x".into(),
            workspace: skill("x", "0.1", "only-ws"),
            library: skill("x", "0.2", "shared-prefix\nextra-lib"),
        };
        let d = render_diff(&c);
        assert!(d.contains("-only-ws"));
        assert!(d.contains("+shared-prefix"));
        assert!(d.contains("+extra-lib"));
    }
}
