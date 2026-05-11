//! Unit tests for the self-update handler. Split from
//! `self_update.rs` to keep that file under the 500-LOC ceiling.

use super::self_update::{
    expected_asset_name, is_newer, parse_pubkey_pem, parse_releases, pick_asset,
    pick_signature_asset, verify_signature, Asset, Channel,
};
use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;

const RELEASE_PUBKEY_PEM: &[u8] = include_bytes!("../../assets/release-pubkey.pem");

#[test]
fn channel_parses_known_values() {
    assert_eq!(Channel::parse("stable").unwrap(), Channel::Stable);
    assert_eq!(Channel::parse("beta").unwrap(), Channel::Beta);
    assert!(Channel::parse("nightly").is_err());
    assert!(!Channel::Stable.allows_prerelease());
    assert!(Channel::Beta.allows_prerelease());
}

#[test]
fn embedded_pubkey_parses() {
    let key = parse_pubkey_pem(RELEASE_PUBKEY_PEM).expect("embedded pubkey valid");
    assert_eq!(key.to_bytes().len(), 32);
}

#[test]
fn signature_round_trip_verifies() {
    let signing = SigningKey::generate(&mut OsRng);
    let verifying = signing.verifying_key();
    let msg = b"argo-darwin-aarch64.tar.gz contents";
    let sig = signing.sign(msg);
    verify_signature(&verifying, msg, &sig.to_bytes()).expect("good sig verifies");

    let mut bad = sig.to_bytes();
    bad[0] ^= 0xff;
    assert!(verify_signature(&verifying, msg, &bad).is_err());

    assert!(verify_signature(&verifying, b"different bytes", &sig.to_bytes()).is_err());
}

#[test]
fn asset_name_matches_current_platform() {
    let name = expected_asset_name();
    assert!(name.starts_with("argo-"));
    assert!(name.ends_with(".tar.gz"));
    assert!(!name.contains(' '));
    assert!(!name.contains('/'));
}

#[test]
fn version_comparison_uses_semver_then_string_fallback() {
    assert!(is_newer("v0.0.2", "0.0.1"));
    assert!(is_newer("0.1.0", "0.0.9"));
    assert!(!is_newer("0.0.1", "0.0.1"));
    assert!(!is_newer("v0.0.1", "0.0.2"));
    assert!(is_newer("nightly-b", "nightly-a"));
}

#[test]
fn parse_releases_picks_latest_stable_skipping_prerelease() {
    let body = serde_json::json!([
        { "tag_name": "v0.2.0-rc.1", "prerelease": true, "draft": false, "assets": [] },
        { "tag_name": "v0.1.0", "prerelease": false, "draft": false, "assets": [
            { "name": "argo-darwin-aarch64.tar.gz", "browser_download_url": "https://example/x.tgz" }
        ] }
    ]);
    let r = parse_releases(&body, Channel::Stable).unwrap();
    assert_eq!(r.tag, "v0.1.0");
    assert_eq!(r.assets.len(), 1);

    let r_beta = parse_releases(&body, Channel::Beta).unwrap();
    assert_eq!(r_beta.tag, "v0.2.0-rc.1");
}

#[test]
fn pick_asset_requires_exact_name_match() {
    let assets = vec![
        Asset {
            name: "argo-linux-x86_64.tar.gz".to_string(),
            url: "u1".to_string(),
        },
        Asset {
            name: "argo-darwin-aarch64.tar.gz".to_string(),
            url: "u2".to_string(),
        },
    ];
    let picked = pick_asset(&assets, "argo-darwin-aarch64.tar.gz").unwrap();
    assert_eq!(picked.url, "u2");
    assert!(pick_asset(&assets, "argo-windows-x86_64.zip").is_err());
}

#[test]
fn pick_signature_asset_appends_dot_sig() {
    let assets = vec![
        Asset {
            name: "argo-linux-x86_64.tar.gz".to_string(),
            url: "u1".to_string(),
        },
        Asset {
            name: "argo-linux-x86_64.tar.gz.sig".to_string(),
            url: "u2".to_string(),
        },
    ];
    let sig = pick_signature_asset(&assets, "argo-linux-x86_64.tar.gz").unwrap();
    assert_eq!(sig.url, "u2");
    assert!(pick_signature_asset(&assets, "argo-darwin-aarch64.tar.gz").is_none());
}
