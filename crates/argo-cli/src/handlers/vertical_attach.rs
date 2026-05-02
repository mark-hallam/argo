//! Session-create-time helpers for vertical-bound sessions
//! (#653+). Right now: install + attach the vertical's
//! `[skills].bundle`. Future PRs in the runtime-activation arc
//! (#654) will add system-prompt loading + audit payload
//! enrichment alongside.
//!
//! Pulled into a sibling file so `vertical.rs` stays focused on
//! discovery / rendering (#650 surface) and this stays focused
//! on side-effects against the SessionStore.
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use argo_vertical::LoadedVertical;
use std::path::Path;
use std::sync::Arc;

/// Install (or reuse) every `*.skill.md` in the vertical's
/// `[skills].bundle` directory and attach each to the freshly-
/// created session.
///
/// Idempotent on `install` — if a skill with the same name is
/// already in the store from a prior session, we reuse the
/// existing row rather than erroring. Attachment is also
/// idempotent (`INSERT OR IGNORE` at the SQL layer).
///
/// Returns the count of skills attached. Returns `Ok(0)` and
/// short-circuits if the vertical didn't declare a `[skills]`
/// section, or if the bundle dir is empty. Errors are scoped
/// per-skill — one bad markdown file shouldn't blind the rest.
pub async fn install_and_attach_bundle(
    store: &Arc<SessionStore>,
    vertical: &LoadedVertical,
    session_id: SessionId,
) -> ArgoResult<usize> {
    let Some(skills_section) = &vertical.manifest.skills else {
        return Ok(0);
    };
    let bundle_dir = vertical.root.join(&skills_section.bundle);
    if !bundle_dir.is_dir() {
        // The validator catches this at load time, but be
        // defensive at runtime in case the dir got removed
        // between load + session-create.
        return Err(ArgoError::Provider(format!(
            "vertical '{}': skills bundle dir '{}' is not a directory",
            vertical.manifest.vertical.id,
            bundle_dir.display()
        )));
    }

    let skill_store = SkillStore::new(Arc::clone(store));
    let entries: Vec<std::path::PathBuf> = list_skill_files(&bundle_dir)?;

    let mut attached = 0_usize;
    for entry in entries {
        match attach_one(&skill_store, &entry, session_id).await {
            Ok(()) => attached += 1,
            Err(e) => {
                // Per-skill errors don't abort the whole bundle —
                // log and continue. Once we have a structured
                // logger plumbed, swap eprintln! for tracing::warn.
                eprintln!(
                    "vertical '{}': skill '{}' failed to attach: {e}",
                    vertical.manifest.vertical.id,
                    entry.display()
                );
            }
        }
    }
    Ok(attached)
}

fn list_skill_files(bundle_dir: &Path) -> ArgoResult<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(bundle_dir)
        .map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", bundle_dir.display())))?
    {
        let entry = entry.map_err(|e| ArgoError::Io(format!("read_dir entry: {e}")))?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".skill.md"))
        {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

async fn attach_one(
    skill_store: &SkillStore,
    skill_path: &Path,
    session_id: SessionId,
) -> ArgoResult<()> {
    let raw = std::fs::read_to_string(skill_path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", skill_path.display())))?;
    // Pull the skill's name out of its frontmatter so we can
    // look up an existing row before attempting install.
    let name = parse_skill_name(&raw)?;
    let existing = skill_store.find_active_by_name(&name).await?;
    let skill_id = match existing {
        Some(s) => s.id,
        None => skill_store.install(&raw, None).await?.id,
    };
    skill_store.attach_to_session(session_id, skill_id).await?;
    Ok(())
}

/// Tiny frontmatter scraper — just pulls `name: <value>` out of
/// the YAML-ish header without dragging in a full YAML parser.
/// argo-skill does the strict parse on install; we only need
/// the name for the existing-skill lookup.
fn parse_skill_name(raw: &str) -> ArgoResult<String> {
    let mut in_frontmatter = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
                continue;
            }
            break;
        }
        if !in_frontmatter {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name:") {
            let name = rest.trim().trim_matches('"').trim().to_string();
            if name.is_empty() {
                return Err(ArgoError::Schema(
                    "skill frontmatter `name:` is empty".to_string(),
                ));
            }
            return Ok(name);
        }
    }
    Err(ArgoError::Schema(
        "skill frontmatter missing `name:` field".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_name() {
        let raw = "---\nname: \"foo_bar\"\nversion: 0.1.0\n---\n# body\n";
        assert_eq!(parse_skill_name(raw).unwrap(), "foo_bar");
    }

    #[test]
    fn parses_unquoted_name() {
        let raw = "---\nname: hello\n---\n# body";
        assert_eq!(parse_skill_name(raw).unwrap(), "hello");
    }

    #[test]
    fn rejects_missing_name() {
        let raw = "---\nversion: 0.1.0\n---\n# body";
        let err = parse_skill_name(raw).unwrap_err();
        assert!(format!("{err}").contains("missing"));
    }

    #[test]
    fn rejects_no_frontmatter() {
        let raw = "# just a body\nno frontmatter\n";
        let err = parse_skill_name(raw).unwrap_err();
        assert!(format!("{err}").contains("missing"));
    }

    #[test]
    fn skips_lines_outside_frontmatter() {
        let raw = "name: foo\n---\nname: real_one\n---\n";
        assert_eq!(parse_skill_name(raw).unwrap(), "real_one");
    }
}
