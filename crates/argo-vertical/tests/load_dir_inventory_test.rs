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
