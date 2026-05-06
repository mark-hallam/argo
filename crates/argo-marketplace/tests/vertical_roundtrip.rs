//! Vertical bundle round-trip — Stream
//! `claude-marketplace-vertical-publish`. Uses `MemoryRegistry`
//! so the test runs hermetically; the exact same surface
//! (`build_vertical_bundle` → `OciRegistry::push` →
//! `OciRegistry::pull` → `unpack_vertical_bundle`) plugs into a
//! real `OciDistributionRegistry` against a `registry:2` test
//! container under the gated `OCI_REGISTRY_INTEGRATION=1`
//! environment variable in the integration suite.
//!
//! Flow:
//!   1. Materialize a fixture vertical under a temp dir.
//!   2. `build_vertical_bundle` to a tar + manifest.
//!   3. Push to `MemoryRegistry`.
//!   4. Pull back; assert byte-for-byte equality on the tar.
//!   5. `unpack_vertical_bundle` to a fresh target dir; verify
//!      every original file is present with the right body.
//!   6. Tamper the tar mid-stream and confirm
//!      `unpack_vertical_bundle` rejects the digest mismatch
//!      without writing any file.
use argo_marketplace::{
    build_vertical_bundle, unpack_vertical_bundle, MemoryRegistry, OciReference, OciRegistry,
};
use std::fs;
use std::path::Path;

fn write_vertical_fixture(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("vertical.toml"),
        "[meta]\nid=\"selling-skates\"\nname=\"Selling skates\"\nversion=\"0.1.0\"\n",
    )
    .unwrap();
    fs::write(root.join("worker.toml"), "[worker]\nname=\"seller\"\n").unwrap();
    fs::create_dir_all(root.join("prompts")).unwrap();
    fs::write(root.join("prompts/system.md"), "you are a selling-skates worker").unwrap();
    fs::write(root.join("prompts/safety.md"), "do not share PII").unwrap();
    fs::create_dir_all(root.join("memory")).unwrap();
    fs::write(root.join("memory/seed.md"), "initial price points: $50-$200").unwrap();
    fs::create_dir_all(root.join("skills")).unwrap();
    fs::write(root.join("skills/post.md"), "post a listing").unwrap();
    fs::create_dir_all(root.join("eval")).unwrap();
    fs::write(root.join("eval/golden.json"), "{\"steps\":[]}").unwrap();
}

#[tokio::test]
async fn vertical_round_trip_through_memory_registry() {
    let dir = tempfile::tempdir().unwrap();
    let vroot = dir.path().join("v");
    write_vertical_fixture(&vroot);
    let tar_path = dir.path().join("v.tar");

    let manifest = build_vertical_bundle(&vroot, "selling-skates", "0.1.0", &tar_path)
        .await
        .unwrap();
    let tar_bytes = tokio::fs::read(&tar_path).await.unwrap();
    assert!(!tar_bytes.is_empty());
    assert_eq!(manifest.workspace_name, "selling-skates");
    assert_eq!(manifest.tag, "0.1.0");

    let registry = MemoryRegistry::new();
    let reference = OciReference::parse("ghcr.io/test/selling-skates:0.1.0").unwrap();
    registry
        .push(&reference, &tar_bytes, &manifest)
        .await
        .unwrap();

    let (pulled_bytes, pulled_manifest) = registry.pull(&reference).await.unwrap();
    assert_eq!(pulled_bytes, tar_bytes);
    assert_eq!(pulled_manifest.digest, manifest.digest);

    let target = dir.path().join("restored");
    unpack_vertical_bundle(&pulled_bytes, &pulled_manifest, &target)
        .await
        .unwrap();
    assert!(target.join("vertical.toml").exists());
    assert!(target.join("prompts/system.md").exists());
    assert!(target.join("eval/golden.json").exists());
    assert_eq!(
        std::fs::read_to_string(target.join("memory/seed.md")).unwrap(),
        "initial price points: $50-$200"
    );

    // Re-bundle the unpacked tree and confirm digest matches —
    // the §23.3 deterministic round-trip property scoped to
    // verticals.
    let tar2 = dir.path().join("v2.tar");
    let manifest2 = build_vertical_bundle(&target, "selling-skates", "0.1.0", &tar2)
        .await
        .unwrap();
    assert_eq!(manifest2.digest, manifest.digest);
}

#[tokio::test]
async fn tampered_tar_is_rejected_before_extraction() {
    let dir = tempfile::tempdir().unwrap();
    let vroot = dir.path().join("v");
    write_vertical_fixture(&vroot);
    let tar_path = dir.path().join("v.tar");
    let manifest = build_vertical_bundle(&vroot, "v", "0.1.0", &tar_path)
        .await
        .unwrap();
    let mut tar_bytes = tokio::fs::read(&tar_path).await.unwrap();
    let mid = tar_bytes.len() / 2;
    tar_bytes[mid] ^= 0xFF;
    let target = dir.path().join("restored");
    let err = unpack_vertical_bundle(&tar_bytes, &manifest, &target)
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("digest mismatch"),
        "expected digest mismatch err, got: {msg}"
    );
    assert!(
        !target.join("vertical.toml").exists(),
        "must not write any file when digest fails"
    );
}
