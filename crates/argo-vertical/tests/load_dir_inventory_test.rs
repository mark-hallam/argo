use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate path has workspace parent")
        .to_path_buf()
}

fn immediate_subdirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = fs::read_dir(root)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<non-utf8>")
        .to_string()
}

#[test]
fn repository_vertical_directories_are_loadable() {
    let verticals_dir = repo_root().join("verticals");
    let subdirs = immediate_subdirs(&verticals_dir);
    assert!(
        !subdirs.is_empty(),
        "expected repo verticals under {}",
        verticals_dir.display()
    );

    let missing_manifest = subdirs
        .iter()
        .filter(|path| !path.join("vertical.toml").is_file())
        .map(|path| dir_name(path))
        .collect::<Vec<_>>();
    assert!(
        missing_manifest.is_empty(),
        "every verticals/* directory must contain vertical.toml; archive demo-only dirs instead: {missing_manifest:?}"
    );

    let (registry, errors) =
        argo_vertical::load_dir(&verticals_dir).expect("workspace verticals dir loads");
    assert!(
        errors.is_empty(),
        "all repo vertical manifests must parse and validate: {}",
        errors
            .iter()
            .map(|err| format!("{}: {}", err.dir.display(), err.error))
            .collect::<Vec<_>>()
            .join("; ")
    );

    let manifest_dirs = subdirs
        .iter()
        .filter(|path| path.join("vertical.toml").is_file())
        .map(|path| dir_name(path))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        registry.len(),
        manifest_dirs.len(),
        "load_dir should load every manifest-bearing directory exactly once"
    );
}

/// Closes onboarding-hire-an-ai-teammate F16 [P3]: every shipped
/// vertical pack must declare ≥1 workflow file so the gateway's
/// `install-vertical` returns `workflows_added ≥ 1`. Pre-fix, every
/// non-(gtm-engineer|pr-checker|receptionist) pack returned 0 and
/// the freshly hired worker's Overview Schedules card had nothing
/// to "Run now" on. Walks the on-disk catalog so adding a new
/// vertical without a workflow trips this gate.
#[test]
fn every_shipped_vertical_declares_at_least_one_workflow() {
    let verticals_dir = repo_root().join("verticals");
    let (registry, errors) =
        argo_vertical::load_dir(&verticals_dir).expect("workspace verticals dir loads");
    assert!(errors.is_empty(), "load errors: {errors:?}");
    let mut missing: Vec<String> = Vec::new();
    for (id, loaded) in registry.list() {
        // Library-tier packs (tagged `library` and intended to be
        // referenced by OTHER verticals — e.g. `_starter-library/`)
        // have no role/agent of their own, so the install-vertical
        // path never targets them and they don't need a workflow.
        if loaded
            .manifest
            .vertical
            .tags
            .iter()
            .any(|t| t == "library")
        {
            continue;
        }
        let declared = loaded
            .manifest
            .workflows
            .as_ref()
            .map_or(0, |w| w.files.len());
        if declared == 0 {
            missing.push(id.to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "verticals missing [workflows].files entries (add at least one workflows/<name>.workflow.toml \
         and declare it in vertical.toml's [workflows] block): {missing:?}"
    );
}
