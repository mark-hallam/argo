//! Frontmatter + body parser for `SKILL.md`.
use crate::skill::SkillFrontmatter;
use argo_types::{ArgoError, ArgoResult};

/// Frontmatter delimiter line per §14.2 — `---` (three dashes), surrounded
/// by line breaks.
pub const FRONTMATTER_DELIM: &str = "---";

/// Split a `SKILL.md` document into `(frontmatter, body)`.
///
/// ```
/// use argo_skill::parse_skill_md;
/// let raw = r#"---
/// name: jupyter-live-kernel
/// version: "0.3.1"
/// description: "Use when the user wants an interactive Jupyter kernel."
/// provenance:
///   source: builtin
///   hash: sha256-abc
/// ---
///
/// [skill body]
/// "#;
/// let (front, body) = parse_skill_md(raw).unwrap();
/// assert_eq!(front.name, "jupyter-live-kernel");
/// assert!(body.contains("[skill body]"));
/// ```
pub fn parse_skill_md(raw: &str) -> ArgoResult<(SkillFrontmatter, String)> {
    let body = raw.trim_start_matches('\u{feff}').trim_start_matches('\n');
    let after_open = body.strip_prefix(FRONTMATTER_DELIM).ok_or_else(|| {
        ArgoError::Schema("SKILL.md must start with a `---` frontmatter block".into())
    })?;
    let after_open = after_open.strip_prefix('\n').ok_or_else(|| {
        ArgoError::Schema("SKILL.md frontmatter `---` must be followed by a newline".into())
    })?;
    let close = after_open
        .find("\n---")
        .ok_or_else(|| ArgoError::Schema("SKILL.md frontmatter has no closing `---`".into()))?;
    let yaml = &after_open[..close];
    let mut rest = &after_open[close + 4..];
    if let Some(stripped) = rest.strip_prefix('\n') {
        rest = stripped;
    }
    let front: SkillFrontmatter = serde_yaml::from_str(yaml)
        .map_err(|e| ArgoError::Schema(format!("SKILL.md frontmatter YAML: {e}")))?;
    if front.name.is_empty() {
        return Err(ArgoError::Schema(
            "SKILL.md frontmatter: name is empty".into(),
        ));
    }
    if front.version.is_empty() {
        return Err(ArgoError::Schema(
            "SKILL.md frontmatter: version is empty".into(),
        ));
    }
    Ok((front, rest.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_skill() {
        let raw = "---\nname: x\nversion: \"0.1\"\ndescription: \"d\"\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\nbody";
        let (front, body) = parse_skill_md(raw).unwrap();
        assert_eq!(front.name, "x");
        assert_eq!(front.version, "0.1");
        assert_eq!(body, "body");
    }

    #[test]
    fn rejects_missing_open_delim() {
        let err = parse_skill_md("name: x\nversion: 0.1\n---\nbody").unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn rejects_missing_close_delim() {
        let err = parse_skill_md("---\nname: x\nversion: 0.1\nbody").unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn rejects_empty_name() {
        let raw = "---\nname: \"\"\nversion: \"0.1\"\ndescription: d\nprovenance:\n  source: x\n  hash: y\n---\nbody";
        let err = parse_skill_md(raw).unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn lock_default_is_none() {
        let raw = "---\nname: x\nversion: \"0.1\"\ndescription: d\nprovenance:\n  source: x\n  hash: y\n---\n";
        let (front, _) = parse_skill_md(raw).unwrap();
        assert_eq!(front.lock, argo_types::Lock::None);
    }

    #[test]
    fn lock_user_protected_round_trips() {
        let raw = "---\nname: x\nversion: \"0.1\"\ndescription: d\nlock: user-protected\nprovenance:\n  source: x\n  hash: y\n---\n";
        let (front, _) = parse_skill_md(raw).unwrap();
        assert_eq!(front.lock, argo_types::Lock::UserProtected);
    }

    #[test]
    fn handles_leading_byte_order_mark() {
        let raw = "\u{feff}---\nname: x\nversion: \"0.1\"\ndescription: d\nprovenance:\n  source: x\n  hash: y\n---\nbody";
        let (front, _) = parse_skill_md(raw).unwrap();
        assert_eq!(front.name, "x");
    }
}
