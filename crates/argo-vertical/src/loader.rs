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

/// Canonical tenant-id → on-disk slug mapping for the
/// `verticals/.genesis/<slug>/` tree. Lowercases, keeps `[a-z0-9-]`,
/// maps every other byte to `-`, trims leading/trailing `-`, and falls
/// back to `anonymous` for an empty result.
///
/// This is the SINGLE source of truth for the slug a Genesis-generated
/// vertical is written under and read back under. The gateway SSE
/// writer, the hire gate, the run gate, and the agent-runner fence all
/// converge here, so the write-slug can never drift from the read-slug.
///
/// ```
/// assert_eq!(argo_vertical::sanitize_tenant("Op-ABC123"), "op-abc123");
/// assert_eq!(argo_vertical::sanitize_tenant("a b!c"), "a-b-c");
/// assert_eq!(argo_vertical::sanitize_tenant(""), "anonymous");
/// ```
#[must_use]
pub fn sanitize_tenant(tenant_id: &str) -> String {
    let cleaned: String = tenant_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "anonymous".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Resolve a single worker's vertical by `vertical_id`, **tenant-scoped**.
///
/// Looks first in the flat `root/<…>/` catalog (matched by manifest id,
/// exactly as [`load_dir`] does — a dir name may differ from the id it
/// declares), then in the tenant-scoped Genesis tree
/// `root/.genesis/<sanitize_tenant(tenant_id)>/<vertical_id>/`. Returns
/// the loaded vertical from whichever path resolves, or `None` when the
/// id is unknown to BOTH.
///
/// **Fail-closed cross-tenant.** The Genesis lookup is scoped to the
/// caller's OWN tenant segment only: a vertical materialized for a
/// *different* tenant resolves to `None`, never another tenant's pack.
/// This mirrors the gateway hire-gate's
/// `genesis_vertical_dir_for_tenant` semantics so the run path can never
/// resolve a teammate the caller was never allowed to hire.
///
/// ```no_run
/// let dir = argo_vertical::default_dir();
/// // Resolves a flat OR a `.genesis/<tenant>/<id>/` vertical, scoped to
/// // `tenant_id`; `None` if unknown or owned by a different tenant.
/// let loaded = argo_vertical::load_for_worker(&dir, "op-alice", "genesis-intern");
/// assert!(loaded.is_none() || loaded.is_some());
/// ```
#[must_use]
pub fn load_for_worker(root: &Path, tenant_id: &str, vertical_id: &str) -> Option<LoadedVertical> {
    // Flat catalog first — preserves exact existing flat semantics
    // (registry keys by the manifest's `id`, not the directory name).
    // cu1h (DEC-050 #5): the flat catalog is best-effort here (a Genesis
    // tree below may still resolve the worker), but surface a `load_dir`
    // failure to stderr rather than swallowing it — the old non-worker
    // path logged it, and a silent flat-catalog failure otherwise looks
    // identical to "vertical simply not present." Matches the crate's
    // documented `eprintln!`-on-load-error idiom (see lib.rs).
    match load_dir(root) {
        Ok((registry, _errors)) => {
            if let Some(loaded) = registry.get(vertical_id) {
                return Some(loaded.clone());
            }
        }
        Err(e) => {
            eprintln!(
                "load_for_worker: flat catalog at {} failed to load: {e}",
                root.display()
            );
        }
    }
    // Tenant-scoped Genesis tree. Exact dir — never a cross-tenant scan.
    let slug = sanitize_tenant(tenant_id);
    let dir = root.join(".genesis").join(slug).join(vertical_id);
    let manifest_path = dir.join("vertical.toml");
    if manifest_path.is_file() {
        if let Ok(loaded) = load_one(&dir, &manifest_path) {
            return Some(loaded);
        }
    }
    None
}

#[cfg(test)]
#[path = "loader_edge_tests.rs"]
mod edge_tests;

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
        fs::write(dir.join("vertical.toml"), MIN_TOML.replace("{id}", id)).unwrap();
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

    fn write_genesis_vertical(root: &Path, tenant_slug: &str, id: &str) {
        let dir = root.join(".genesis").join(tenant_slug).join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vertical.toml"), MIN_TOML.replace("{id}", id)).unwrap();
    }

    #[test]
    fn sanitize_tenant_canonicalizes() {
        assert_eq!(sanitize_tenant("Op-ABC123"), "op-abc123");
        assert_eq!(sanitize_tenant("a b!c"), "a-b-c");
        assert_eq!(sanitize_tenant("---"), "anonymous");
        assert_eq!(sanitize_tenant(""), "anonymous");
        assert_eq!(sanitize_tenant("-mid-"), "mid");
    }

    #[test]
    fn load_for_worker_resolves_flat_vertical() {
        let dir = tempdir().unwrap();
        write_vertical(dir.path(), "customer-support");
        let got = load_for_worker(dir.path(), "any-tenant", "customer-support");
        assert!(got.is_some(), "flat vertical must resolve for any tenant");
    }

    #[test]
    fn load_for_worker_resolves_genesis_vertical_for_own_tenant() {
        let dir = tempdir().unwrap();
        // Materialized under the OWN tenant's segment.
        let slug = sanitize_tenant("op-alice");
        write_genesis_vertical(dir.path(), &slug, "genesis-personal-019eaf");
        let got = load_for_worker(dir.path(), "op-alice", "genesis-personal-019eaf");
        assert!(got.is_some(), "genesis vertical must resolve for its own tenant");
        assert_eq!(
            got.unwrap().manifest.vertical.id,
            "genesis-personal-019eaf"
        );
    }

    #[test]
    fn load_for_worker_fail_closed_cross_tenant() {
        let dir = tempdir().unwrap();
        // Materialized for op-alice only.
        let slug = sanitize_tenant("op-alice");
        write_genesis_vertical(dir.path(), &slug, "genesis-personal-019eaf");
        // A DIFFERENT tenant must NOT resolve it — fail-closed.
        let got = load_for_worker(dir.path(), "op-bob", "genesis-personal-019eaf");
        assert!(
            got.is_none(),
            "another tenant's genesis pack must never resolve"
        );
    }

    #[test]
    fn load_for_worker_none_for_unknown_id() {
        let dir = tempdir().unwrap();
        write_vertical(dir.path(), "alpha");
        assert!(load_for_worker(dir.path(), "t", "ghost").is_none());
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
