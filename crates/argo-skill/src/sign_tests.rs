//! Pure-function unit tests for the cosign-compatible ed25519 signing
//! primitives in [`crate::sign`].

use super::*;
use argo_types::ArgoError;
use std::fs;
use std::path::PathBuf;

fn tmp_dir(label: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "argo-skill-sign-{label}-{}-{seq}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn keypair_roundtrips_through_pem() {
    let kp = generate_keypair();
    let priv_pem = encode_private_pem(&kp);
    let pub_pem = encode_public_pem(&kp.verifying_key());
    let decoded_priv = decode_private_pem(&priv_pem).unwrap();
    let decoded_pub = decode_public_pem(&pub_pem).unwrap();
    assert_eq!(decoded_priv.to_bytes(), kp.to_bytes());
    assert_eq!(decoded_pub.to_bytes(), kp.verifying_key().to_bytes());
}

#[test]
fn write_keypair_to_dir_creates_files_at_expected_paths() {
    let dir = tmp_dir("write-keypair");
    let kp = generate_keypair();
    let paths = write_keypair_to_dir(&dir, &kp).unwrap();
    assert!(paths.private.ends_with(COSIGN_PRIVATE_FILE));
    assert!(paths.public.ends_with(COSIGN_PUBLIC_FILE));
    let priv_pem = fs::read_to_string(&paths.private).unwrap();
    let pub_pem = fs::read_to_string(&paths.public).unwrap();
    assert!(priv_pem.contains("BEGIN COSIGN PRIVATE KEY"));
    assert!(pub_pem.contains("BEGIN COSIGN PUBLIC KEY"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn sign_then_verify_roundtrip_passes() {
    let kp = generate_keypair();
    let payload = b"some SKILL.md bytes";
    let (_digest, sig_bytes) = sign_payload(&kp, payload);
    verify_payload(&kp.verifying_key(), payload, &sig_bytes).unwrap();
}

#[test]
fn verify_rejects_tampered_payload() {
    let kp = generate_keypair();
    let (_digest, sig) = sign_payload(&kp, b"original");
    let err = verify_payload(&kp.verifying_key(), b"tampered", &sig).unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn artifact_roundtrip_ok_through_mock_registry() {
    let root = tmp_dir("artifact");
    fs::create_dir_all(&root).unwrap();
    let registry = format!("mock://{}", root.display());
    let kp = generate_keypair();
    let payload = b"---\nname: foo\nversion: \"0.1.0\"\n---\nbody\n";
    let sig = build_signature_artifact("foo", "0.1.0", &kp, payload);
    write_signature_artifact(&registry, &sig, "0.1.0").unwrap();
    let outcome =
        verify_skill_signature(&registry, "foo", "0.1.0", &kp.verifying_key(), payload)
            .unwrap();
    assert_eq!(outcome, SkillVerifyOutcome::Ok);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn tampered_payload_yields_fail_outcome() {
    let root = tmp_dir("tamper");
    fs::create_dir_all(&root).unwrap();
    let registry = format!("mock://{}", root.display());
    let kp = generate_keypair();
    let payload = b"original SKILL.md";
    let sig = build_signature_artifact("foo", "0.1.0", &kp, payload);
    write_signature_artifact(&registry, &sig, "0.1.0").unwrap();
    let outcome =
        verify_skill_signature(&registry, "foo", "0.1.0", &kp.verifying_key(), b"tampered")
            .unwrap();
    assert!(matches!(outcome, SkillVerifyOutcome::Fail(_)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn missing_signature_yields_missing_outcome() {
    let root = tmp_dir("missing");
    fs::create_dir_all(&root).unwrap();
    let registry = format!("mock://{}", root.display());
    let kp = generate_keypair();
    let outcome =
        verify_skill_signature(&registry, "no-such", "0.1.0", &kp.verifying_key(), b"x")
            .unwrap();
    assert_eq!(outcome, SkillVerifyOutcome::Missing);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_keypair_from_disk_roundtrips() {
    let dir = tmp_dir("load");
    let kp = generate_keypair();
    let paths = write_keypair_to_dir(&dir, &kp).unwrap();
    let signing = load_private_key(&paths.private).unwrap();
    let verifying = load_public_key(&paths.public).unwrap();
    assert_eq!(signing.to_bytes(), kp.to_bytes());
    assert_eq!(verifying.to_bytes(), kp.verifying_key().to_bytes());
    let _ = fs::remove_dir_all(dir);
}
