use argo_marketplace::{
    build_bundle, build_vertical_bundle, manifests_match, resolve_registry_auth,
    unpack_vertical_bundle, BearerChallenge, BundleEntry, BundleManifest, MemoryRegistry,
    OciDescriptor, OciDistributionRegistry, OciImageManifest, OciReference, OciRegistry,
    ResolvedRegistryAuth, StubRegistry, BUNDLE_PATHS, CONFIG_MEDIA_TYPE, LAYER_MEDIA_TYPE,
    MANIFEST_MEDIA_TYPE,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn lib_public_surface_inventory_is_intentional() {
    let lib = include_str!("../src/lib.rs");
    let normalized = lib.split_whitespace().collect::<Vec<_>>().join(" ");

    let modules: Vec<&str> = lib
        .lines()
        .filter_map(|line| line.trim().strip_prefix("mod "))
        .map(|tail| tail.trim_end_matches(';'))
        .collect();
    assert_eq!(
        modules,
        vec![
            "bundle",
            "bundle_vertical",
            "oci_auth",
            "oci_distribution",
            "registry",
        ],
        "crate root module inventory changed"
    );

    for expected in [
        "pub use bundle::{build_bundle, manifests_match, BundleEntry, BundleManifest, BUNDLE_PATHS};",
        "pub use bundle_vertical::{build_vertical_bundle, unpack_vertical_bundle};",
        "pub use oci_auth::{resolve_registry_auth, BearerChallenge, ResolvedRegistryAuth};",
        "pub use oci_distribution::{ OciDescriptor, OciDistributionRegistry, OciImageManifest, CONFIG_MEDIA_TYPE, LAYER_MEDIA_TYPE, MANIFEST_MEDIA_TYPE, };",
        "pub use registry::{MemoryRegistry, OciReference, OciRegistry, StubRegistry};",
    ] {
        assert!(
            normalized.contains(expected),
            "missing or changed crate-root export group: {expected}"
        );
    }
}

#[tokio::test]
async fn workspace_bundle_round_trips_through_public_registry_surface() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    write_workspace_fixture(&workspace);

    assert_eq!(
        BUNDLE_PATHS,
        &[
            "argo.toml",
            "secrets.example.toml",
            "MEMORY.md",
            "USER.md",
            "SOUL.md",
            "AGENTS.md",
            "personas",
            "skills",
            "cron",
        ]
    );

    let tar_path = dir.path().join("workspace.tar");
    let manifest = build_bundle(&workspace, "team-workspace", "2026.05", &tar_path)
        .await
        .unwrap();
    assert_eq!(manifest.workspace_name, "team-workspace");
    assert_eq!(manifest.tag, "2026.05");
    assert_eq!(
        entry_paths(&manifest),
        vec![
            "argo.toml",
            "secrets.example.toml",
            "MEMORY.md",
            "USER.md",
            "SOUL.md",
            "AGENTS.md",
            "personas/default.md",
            "skills/alpha.md",
            "skills/nested/beta.md",
            "cron/daily.md",
        ]
    );
    assert!(
        !entry_paths(&manifest).contains(&"rubrics/not-bundled.md"),
        "workspace bundles must stay scoped to BUNDLE_PATHS"
    );

    let tar_bytes = fs::read(&tar_path).unwrap();
    let registry = MemoryRegistry::new();
    assert!(registry.is_empty().await);
    let reference = OciReference::parse("registry.local/argo/team-workspace:2026.05").unwrap();
    registry
        .push(&reference, &tar_bytes, &manifest)
        .await
        .unwrap();
    assert_eq!(registry.len().await, 1);

    let (pulled_tar, pulled_manifest) = registry.pull(&reference).await.unwrap();
    assert_eq!(pulled_tar, tar_bytes);
    assert_eq!(entry_paths(&pulled_manifest), entry_paths(&manifest));

    let second_tar = dir.path().join("workspace-again.tar");
    let second = build_bundle(&workspace, "team-workspace", "2026.05", &second_tar)
        .await
        .unwrap();
    assert!(manifests_match(&manifest, &second));
}

#[tokio::test]
async fn vertical_bundle_round_trips_and_rejects_tampered_tar() {
    let dir = tempfile::tempdir().unwrap();
    let vertical = dir.path().join("vertical");
    write_vertical_fixture(&vertical);

    let tar_path = dir.path().join("vertical.tar");
    let manifest = build_vertical_bundle(&vertical, "seller", "1.2.3", &tar_path)
        .await
        .unwrap();
    assert_eq!(manifest.workspace_name, "seller");
    assert_eq!(manifest.tag, "1.2.3");
    assert_eq!(
        entry_paths(&manifest),
        vec![
            "eval/golden.json",
            "memory/seed.md",
            "prompts/safety.md",
            "prompts/system.md",
            "skills/post.md",
            "vertical.toml",
            "worker.toml",
        ],
        "vertical bundles should pack every file under the vertical root in sorted order"
    );

    let tar_bytes = fs::read(&tar_path).unwrap();
    let registry = MemoryRegistry::new();
    let reference = OciReference::parse("ghcr.io/argo/seller:1.2.3").unwrap();
    registry
        .push(&reference, &tar_bytes, &manifest)
        .await
        .unwrap();
    let (pulled_tar, pulled_manifest) = registry.pull(&reference).await.unwrap();
    assert_eq!(pulled_tar, tar_bytes);
    assert_eq!(pulled_manifest.digest, manifest.digest);

    let restored = dir.path().join("restored");
    unpack_vertical_bundle(&pulled_tar, &pulled_manifest, &restored)
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(restored.join("prompts/system.md")).unwrap(),
        "sell safely"
    );

    let rebundled_tar = dir.path().join("vertical-again.tar");
    let rebundled = build_vertical_bundle(&restored, "seller", "1.2.3", &rebundled_tar)
        .await
        .unwrap();
    assert_eq!(rebundled.digest, manifest.digest);

    let mut tampered = pulled_tar;
    let mid = tampered.len() / 2;
    tampered[mid] ^= 0xff;
    let err = unpack_vertical_bundle(&tampered, &manifest, &dir.path().join("tampered"))
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("digest mismatch"));
}

#[tokio::test]
async fn registry_auth_and_oci_descriptor_contracts_are_stable_without_network() {
    let reference = OciReference::parse("registry.local:5000/team/seller:v1").unwrap();
    assert_eq!(reference.host, "registry.local:5000");
    assert_eq!(reference.repo, "team/seller");
    assert_eq!(reference.tag, "v1");
    assert_eq!(
        reference.to_canonical(),
        "registry.local:5000/team/seller:v1"
    );
    assert_eq!(
        OciReference::parse("ghcr.io/team/seller")
            .unwrap()
            .to_canonical(),
        "ghcr.io/team/seller:latest"
    );

    let challenge = BearerChallenge::parse(
        r#"Bearer realm="https://ghcr.io/token",service="ghcr.io",scope="repository:team/seller:pull,push""#,
    )
    .unwrap();
    assert_eq!(
        challenge.token_url(),
        "https://ghcr.io/token?service=ghcr.io&scope=repository:team/seller:pull,push"
    );

    assert_eq!(
        resolve_registry_auth("example.invalid", Some(" explicit-token ")),
        ResolvedRegistryAuth::token("explicit-token")
    );

    let config = OciDescriptor {
        media_type: CONFIG_MEDIA_TYPE.to_string(),
        digest: "sha256:config".to_string(),
        size: 42,
    };
    let layer = OciDescriptor {
        media_type: LAYER_MEDIA_TYPE.to_string(),
        digest: "sha256:layer".to_string(),
        size: 128,
    };
    let image_manifest = OciImageManifest {
        schema_version: 2,
        media_type: Some(MANIFEST_MEDIA_TYPE.to_string()),
        config,
        layers: vec![layer],
    };
    assert_eq!(
        serde_json::to_value(&image_manifest).unwrap(),
        json!({
            "schemaVersion": 2,
            "mediaType": MANIFEST_MEDIA_TYPE,
            "config": {
                "mediaType": CONFIG_MEDIA_TYPE,
                "digest": "sha256:config",
                "size": 42
            },
            "layers": [{
                "mediaType": LAYER_MEDIA_TYPE,
                "digest": "sha256:layer",
                "size": 128
            }]
        })
    );

    let registry = OciDistributionRegistry::with_resolved_auth(
        "http://registry.local:5000/",
        ResolvedRegistryAuth::basic("user", "pass"),
    )
    .with_basic_auth("override", "secret");
    assert!(
        format!("{registry:?}").contains("http://registry.local:5000"),
        "debug output should expose the configured endpoint for diagnostics"
    );

    let stub = StubRegistry;
    let err = stub
        .pull(&reference)
        .await
        .expect_err("stub must not perform network IO");
    assert!(format!("{err}").contains(&reference.to_canonical()));
}

fn entry_paths(manifest: &BundleManifest) -> Vec<&str> {
    manifest
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect()
}

fn write_workspace_fixture(root: &Path) {
    fs::create_dir_all(root).unwrap();
    for (path, body) in [
        ("argo.toml", "[core]\nname = \"team-workspace\"\n"),
        ("secrets.example.toml", "TOKEN = \"example\"\n"),
        ("MEMORY.md", "memory"),
        ("USER.md", "user"),
        ("SOUL.md", "soul"),
        ("AGENTS.md", "agents"),
        ("personas/default.md", "persona"),
        ("skills/alpha.md", "alpha"),
        ("skills/nested/beta.md", "beta"),
        ("cron/daily.md", "cron"),
        ("rubrics/not-bundled.md", "ignored"),
    ] {
        write_file(root.join(path), body);
    }
}

fn write_vertical_fixture(root: &Path) {
    fs::create_dir_all(root).unwrap();
    for (path, body) in [
        (
            "vertical.toml",
            "[vertical]\nid = \"seller\"\nversion = \"1.2.3\"\n",
        ),
        ("worker.toml", "[worker]\nname = \"seller\"\n"),
        ("prompts/system.md", "sell safely"),
        ("prompts/safety.md", "no PII"),
        ("memory/seed.md", "seed"),
        ("skills/post.md", "post listing"),
        ("eval/golden.json", "{\"steps\":[]}"),
    ] {
        write_file(root.join(path), body);
    }
}

fn write_file(path: PathBuf, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

#[test]
fn manifest_entry_shape_remains_serde_round_trippable() {
    let manifest = BundleManifest {
        digest: "sha256:manifest".into(),
        workspace_name: "seller".into(),
        tag: "v1".into(),
        entries: vec![BundleEntry {
            path: "vertical.toml".into(),
            size: 11,
            digest: "sha256:entry".into(),
        }],
    };
    let round_tripped: BundleManifest =
        serde_json::from_str(&serde_json::to_string(&manifest).unwrap()).unwrap();
    assert_eq!(round_tripped.entries[0].path, "vertical.toml");
    assert_eq!(round_tripped.entries[0].digest, "sha256:entry");
}
