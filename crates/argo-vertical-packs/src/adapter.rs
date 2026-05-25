//! Pack → vertical translation bridge.
//!
//! The repo's `packs/` entries (Track M marketplace format) describe
//! a worker via `worker.toml` plus a sibling `pack.toml` that names
//! the embedded `[[verticals]]` entries. The runtime template loader
//! (`argo_vertical::load_dir`) only reads `vertical.toml` shapes.
//! This module is the narrow translator that lets one pack feed
//! straight into the vertical loader without a parallel `verticals/`
//! entry.
//!
//! Scope is intentionally tight (see brief
//! `mission-pack-format-adapter-to-vertical-loader`):
//!
//! - identity only — `vertical.id`, `vertical.name`,
//!   `vertical.version`, `vertical.description`, `vertical.authors`;
//! - no silent defaulting — missing required pack fields surface as
//!   a specific `ArgoError::Schema` naming the gap;
//! - delegation / approval / skill / eval bodies do NOT auto-promote
//!   into the vertical manifest. They belong to follow-up adapter
//!   widening; this brief is the bridge layer.
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{validate, LoadedVertical, VerticalIdentity, VerticalManifest, SCHEMA_VERSION};
use serde::Deserialize;
use std::path::Path;

use crate::manifest::{PackManifest, VerticalPackEntry};

/// Minimal worker.toml header the adapter consumes. Other sections
/// (delegation, escalation, policy, memory, tools, skills, evals)
/// are accepted but unused at this layer.
#[derive(Debug, Deserialize)]
struct WorkerHeader {
    worker: WorkerIdentitySection,
}

#[derive(Debug, Deserialize)]
struct WorkerIdentitySection {
    id: String,
    #[serde(default)]
    default_display_name: Option<String>,
}

/// Translate one pack `[[verticals]]` entry into a `LoadedVertical`
/// that the runtime `argo_vertical` loader is willing to register.
///
/// `pack_root` is the unpacked pack directory (the one holding
/// `pack.toml`). `entry.path` is taken relative to `pack_root`; the
/// adapter expects to find `worker.toml` at that sub-path.
///
/// Returns `ArgoError::Schema` with a specific message when:
///
/// - the pack entry's `worker.toml` is missing;
/// - the pack entry's `worker.toml` fails to parse;
/// - `[worker].id` does not match the pack entry id (catches
///   accidental cross-wiring during pack authoring);
/// - no display name can be derived (neither
///   `pack.display_name` nor `[worker].default_display_name`
///   present).
///
/// The synthesized [`VerticalManifest`] is run through
/// [`argo_vertical::validate`] before return, so callers get the same
/// `id` and isolation checks as a native `vertical.toml` load.
pub fn adapt_pack_entry(
    pack_root: &Path,
    manifest: &PackManifest,
    entry: &VerticalPackEntry,
) -> ArgoResult<LoadedVertical> {
    let vertical_root = pack_root.join(&entry.path);
    if !vertical_root.is_dir() {
        return Err(ArgoError::Schema(format!(
            "pack entry '{}': resolved vertical root '{}' is not a directory",
            entry.id,
            vertical_root.display()
        )));
    }
    let worker_toml = vertical_root.join("worker.toml");
    if !worker_toml.is_file() {
        return Err(ArgoError::Schema(format!(
            "pack entry '{}': worker.toml is missing at {}",
            entry.id,
            worker_toml.display()
        )));
    }
    let body = std::fs::read_to_string(&worker_toml)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", worker_toml.display())))?;
    let header: WorkerHeader = toml::from_str(&body)
        .map_err(|e| ArgoError::Schema(format!("parse {}: {e}", worker_toml.display())))?;
    if header.worker.id != entry.id {
        return Err(ArgoError::Schema(format!(
            "pack entry '{}': worker.toml declares [worker].id = '{}'; ids must match so adapter never silently re-keys a vertical",
            entry.id, header.worker.id
        )));
    }

    let name = header
        .worker
        .default_display_name
        .or_else(|| manifest.pack.display_name.clone())
        .ok_or_else(|| {
            ArgoError::Schema(format!(
                "pack entry '{}': neither [worker].default_display_name nor [pack].display_name is set; adapter refuses to default vertical.name to the id slug",
                entry.id
            ))
        })?;

    let authors = manifest
        .pack
        .author
        .clone()
        .map(|a| vec![a])
        .unwrap_or_default();
    let description = manifest.pack.description.clone().unwrap_or_default();

    let vertical_manifest = VerticalManifest {
        schema_version: SCHEMA_VERSION.into(),
        vertical: VerticalIdentity {
            id: entry.id.clone(),
            name,
            description,
            version: manifest.pack.version.clone(),
            authors,
            tags: vec![],
        },
        ..Default::default()
    };
    validate(&vertical_manifest, &vertical_root)?;

    Ok(LoadedVertical {
        manifest: vertical_manifest,
        root: vertical_root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PackMetadata, VerticalPackEntry};
    use std::path::Path;
    use tempfile::tempdir;

    fn write_minimal_pack(root: &Path, id: &str, version: &str) {
        std::fs::write(
            root.join("pack.toml"),
            format!(
                "[pack]\nname='{id}'\nversion='{version}'\ndisplay_name='Demo {id}'\n\n[[verticals]]\nid='{id}'\npath='.'\n",
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("worker.toml"),
            format!("[worker]\nid='{id}'\ndefault_display_name='Demo {id}'\n"),
        )
        .unwrap();
    }

    fn parsed_pack(root: &Path) -> PackManifest {
        let body = std::fs::read_to_string(root.join("pack.toml")).unwrap();
        PackManifest::parse(&body).unwrap()
    }

    #[test]
    fn round_trips_minimal_pack_into_vertical() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        write_minimal_pack(&root, "ap-demo", "1.0.0");
        let manifest = parsed_pack(&root);
        let entry = &manifest.verticals[0];

        let loaded = adapt_pack_entry(&root, &manifest, entry).unwrap();
        assert_eq!(loaded.manifest.vertical.id, "ap-demo");
        assert_eq!(loaded.manifest.vertical.version, "1.0.0");
        assert_eq!(loaded.manifest.vertical.name, "Demo ap-demo");
        assert_eq!(loaded.root, root);
    }

    #[test]
    fn worker_toml_missing_is_specific_error() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::write(
            root.join("pack.toml"),
            "[pack]\nname='x'\nversion='1.0.0'\ndisplay_name='X'\n\n[[verticals]]\nid='x'\npath='.'\n",
        )
        .unwrap();
        let manifest = parsed_pack(&root);
        let err = adapt_pack_entry(&root, &manifest, &manifest.verticals[0]).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("worker.toml is missing"), "got: {msg}");
        assert!(msg.contains("'x'"), "got: {msg}");
    }

    #[test]
    fn worker_id_mismatch_is_specific_error() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::write(
            root.join("pack.toml"),
            "[pack]\nname='x'\nversion='1.0.0'\ndisplay_name='X'\n\n[[verticals]]\nid='x'\npath='.'\n",
        )
        .unwrap();
        std::fs::write(
            root.join("worker.toml"),
            "[worker]\nid='actually-y'\ndefault_display_name='Y'\n",
        )
        .unwrap();
        let manifest = parsed_pack(&root);
        let err = adapt_pack_entry(&root, &manifest, &manifest.verticals[0]).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("actually-y"), "got: {msg}");
        assert!(msg.contains("ids must match"), "got: {msg}");
    }

    #[test]
    fn missing_display_name_is_specific_error() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::write(
            root.join("pack.toml"),
            "[pack]\nname='x'\nversion='1.0.0'\n\n[[verticals]]\nid='x'\npath='.'\n",
        )
        .unwrap();
        std::fs::write(root.join("worker.toml"), "[worker]\nid='x'\n").unwrap();
        let manifest = parsed_pack(&root);
        let err = adapt_pack_entry(&root, &manifest, &manifest.verticals[0]).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("display_name"), "got: {msg}");
    }

    #[test]
    fn extra_worker_toml_sections_are_tolerated() {
        // Real packs carry [delegation] / [memory] / [tools] / [skills]
        // / [evals]; the adapter must not block on them at this stage.
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::write(
            root.join("pack.toml"),
            "[pack]\nname='x'\nversion='1.0.0'\ndisplay_name='X'\nauthor='ACME'\ndescription='Demo'\n\n[[verticals]]\nid='x'\npath='.'\n",
        )
        .unwrap();
        std::fs::write(
            root.join("worker.toml"),
            "[worker]\nid='x'\ndefault_display_name='X'\n\n[delegation]\ncharter='do things'\n\n[tools]\nmcp=['gmail']\n",
        )
        .unwrap();
        let manifest = parsed_pack(&root);
        let loaded = adapt_pack_entry(&root, &manifest, &manifest.verticals[0]).unwrap();
        assert_eq!(loaded.manifest.vertical.authors, vec!["ACME".to_string()]);
        assert_eq!(loaded.manifest.vertical.description, "Demo");
    }

    #[test]
    fn missing_vertical_root_directory_is_specific_error() {
        // Synthesize the pack in code so we can declare a path that
        // intentionally does not exist on disk. `PackManifest::parse`
        // would refuse `path='..'`, but a sub-directory that simply
        // hasn't been materialized is a legitimate "broken pack"
        // shape the adapter should surface clearly.
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let entry = VerticalPackEntry {
            id: "x".into(),
            path: "no-such-subdir".into(),
        };
        let synthetic = PackManifest {
            pack: PackMetadata {
                name: "x".into(),
                version: "1.0.0".into(),
                author: None,
                display_name: Some("X".into()),
                description: None,
            },
            fetcher: crate::manifest::FetcherConfig::default(),
            verticals: vec![entry.clone()],
        };
        let err = adapt_pack_entry(&root, &synthetic, &entry).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not a directory"), "got: {msg}");
    }
}
