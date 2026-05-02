//! Walks a `verticals/` directory, parses + validates each
//! `vertical.toml`, and populates a [`Registry`].
//!
//! Failure model: per-vertical errors are *collected* rather than
//! raised, so one malformed vertical doesn't blind the rest of the
//! catalogue. The caller decides whether to surface them as fatal
//! or as warnings. A duplicate-id collision IS surfaced as an
//! error, since silently dropping one vertical would be worse than
//! halting.
use crate::manifest::{parse, VerticalManifest};
use crate::registry::{LoadedVertical, Registry};
use crate::validator::validate;
use argo_types::{ArgoError, ArgoResult};
use std::fs;
use std::path::{Path, PathBuf};

/// One vertical that failed to load — parse error, validate
/// error, missing manifest file, or registry-collision. Carried
/// alongside the populated registry so callers can log them.
#[derive(Debug)]
pub struct LoadError {
    /// Directory the loader was looking at.
    pub dir: PathBuf,
    /// Why it failed.
    pub error: ArgoError,
}

/// Walk `dir`'s immediate subdirectories. Each subdir is expected
/// to contain a `vertical.toml`; subdirs without one are silently
/// ignored (so `verticals/README.md` etc. don't trip the loader).
///
/// Returns the populated [`Registry`] together with a list of
/// per-vertical errors. The registry contains every vertical that
/// loaded cleanly; the errors describe everything else.
pub fn load_dir(dir: &Path) -> ArgoResult<(Registry, Vec<LoadError>)> {
    let mut registry = Registry::new();
    let mut errors: Vec<LoadError> = Vec::new();

    if !dir.exists() {
        // No verticals dir is fine — empty registry, no errors.
        return Ok((registry, errors));
    }
    if !dir.is_dir() {
        return Err(ArgoError::Schema(format!(
            "verticals path '{}' is not a directory",
            dir.display()
        )));
    }

    let mut subdirs: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", dir.display())))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    // Deterministic order so the caller (and tests) see a stable
    // sequence of errors.
    subdirs.sort();

    for sub in subdirs {
        let manifest_path = sub.join("vertical.toml");
        if !manifest_path.is_file() {
            // Not a vertical — skip silently.
            continue;
        }
        match load_one(&sub, &manifest_path) {
            Ok(loaded) => {
                if let Err(e) = registry.insert(loaded) {
                    errors.push(LoadError {
                        dir: sub.clone(),
                        error: e,
                    });
                }
            }
            Err(e) => errors.push(LoadError { dir: sub, error: e }),
        }
    }

    Ok((registry, errors))
}

fn load_one(root: &Path, manifest_path: &Path) -> ArgoResult<LoadedVertical> {
    let src = fs::read_to_string(manifest_path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", manifest_path.display())))?;
    let manifest: VerticalManifest = parse(&src)?;
    validate(&manifest, root)?;
    Ok(LoadedVertical {
        manifest,
        root: root.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const MIN_TOML: &str = r#"schema_version = "1"

[vertical]
id = "{id}"
name = "Demo {id}"
version = "0.1.0"
"#;

    fn write_vertical(root: &Path, id: &str) {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("vertical.toml"),
            MIN_TOML.replace("{id}", id),
        )
        .unwrap();
    }

    #[test]
    fn missing_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let nonexistent = dir.path().join("does-not-exist");
        let (reg, errs) = load_dir(&nonexistent).unwrap();
        assert_eq!(reg.len(), 0);
        assert!(errs.is_empty());
    }

    #[test]
    fn empty_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let (reg, errs) = load_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 0);
        assert!(errs.is_empty());
    }

    #[test]
    fn loads_two_clean_verticals() {
        let dir = tempdir().unwrap();
        write_vertical(dir.path(), "alpha");
        write_vertical(dir.path(), "beta");
        let (reg, errs) = load_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 2, "errors: {errs:?}");
        assert!(errs.is_empty());
        assert!(reg.get("alpha").is_some());
        assert!(reg.get("beta").is_some());
    }

    #[test]
    fn skips_subdirs_without_manifest() {
        let dir = tempdir().unwrap();
        write_vertical(dir.path(), "alpha");
        // A scratch subdir + a stray file at the top level.
        fs::create_dir_all(dir.path().join("scratch")).unwrap();
        fs::write(dir.path().join("README.md"), "# verticals").unwrap();
        let (reg, errs) = load_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 1);
        assert!(errs.is_empty());
    }

    #[test]
    fn collects_per_vertical_parse_error() {
        let dir = tempdir().unwrap();
        write_vertical(dir.path(), "alpha");
        // bad: missing `[vertical]` table entirely.
        let bad_dir = dir.path().join("broken");
        fs::create_dir_all(&bad_dir).unwrap();
        fs::write(bad_dir.join("vertical.toml"), "schema_version = \"1\"\n").unwrap();
        let (reg, errs) = load_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 1, "alpha should still load");
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].dir, bad_dir);
    }

    #[test]
    fn collects_per_vertical_validate_error() {
        let dir = tempdir().unwrap();
        write_vertical(dir.path(), "alpha");
        // bad: declares a system_prompt that doesn't exist.
        let bad_dir = dir.path().join("missing-role");
        fs::create_dir_all(&bad_dir).unwrap();
        fs::write(
            bad_dir.join("vertical.toml"),
            "schema_version = \"1\"\n\n[vertical]\nid = \"missing-role\"\nname = \"M\"\nversion = \"0.1.0\"\n\n[role]\nsystem_prompt = \"prompts/system.md\"\n",
        )
        .unwrap();
        let (reg, errs) = load_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 1);
        assert_eq!(errs.len(), 1);
        assert!(format!("{}", errs[0].error).contains("system_prompt"));
    }

    #[test]
    fn collects_duplicate_id_collision() {
        let dir = tempdir().unwrap();
        // Both subdirs declare id="dup".
        for sub in ["one", "two"] {
            let p = dir.path().join(sub);
            fs::create_dir_all(&p).unwrap();
            fs::write(
                p.join("vertical.toml"),
                "schema_version = \"1\"\n\n[vertical]\nid = \"dup\"\nname = \"D\"\nversion = \"0.1.0\"\n",
            )
            .unwrap();
        }
        let (reg, errs) = load_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 1, "first wins");
        assert_eq!(errs.len(), 1);
        assert!(format!("{}", errs[0].error).contains("already registered"));
    }
}
