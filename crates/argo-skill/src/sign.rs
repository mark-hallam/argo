//! Cosign-compatible ed25519 signing for skill artifacts.
//!
//! Provides the pure signing primitives that back
//! `argo skill {sign, verify-sig, keygen}`. Keys are stored as
//! cosign-style PEM blocks with the raw 32-byte ed25519 seed / verifying
//! key base64-encoded in the body. Detached signatures land as JSON
//! sibling artifacts inside the OCI registry layout from
//! [`crate::publish`].
//!
//! Encryption of the private-key file is deferred to follow-up
//! (today the seed is base64-encoded plaintext inside the PEM body).
//! Sigstore/Fulcio keyless signing is also follow-up — only `--key`
//! is wired for v1.

use crate::publish::mock_registry_root;
use argo_types::{ArgoError, ArgoResult};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey, SIGNATURE_LENGTH};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// PEM block header for an unencrypted cosign-style ed25519 private key.
pub const COSIGN_PRIVATE_PEM_LABEL: &str = "COSIGN PRIVATE KEY";

/// PEM block header for the matching public verifying key.
pub const COSIGN_PUBLIC_PEM_LABEL: &str = "COSIGN PUBLIC KEY";

/// Default file name written by [`write_keypair_to_dir`] for the
/// private signing key.
pub const COSIGN_PRIVATE_FILE: &str = "cosign.key";

/// Default file name written by [`write_keypair_to_dir`] for the
/// public verifying key.
pub const COSIGN_PUBLIC_FILE: &str = "cosign.pub";

/// File name of the detached signature artifact stored as an OCI sibling.
pub const SKILL_SIGNATURE_FILE: &str = "signature.json";

/// Schema version stamped into the signature artifact.
pub const SKILL_SIGNATURE_SCHEMA_VERSION: u32 = 1;

/// Artifact kind stamped into the signature artifact.
pub const SKILL_SIGNATURE_KIND: &str = "argo.skill.signature";

/// Signature algorithm identifier (only `ed25519` is supported in v1).
pub const SKILL_SIGNATURE_ALGORITHM: &str = "ed25519";

/// Outcome of [`verify_skill_signature`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillVerifyOutcome {
    /// Signature is present and verifies against the supplied public key
    /// and the reconstructed payload.
    Ok,
    /// Signature is present but failed verification. The wrapped string
    /// is a short reason that can be displayed to the operator.
    Fail(String),
    /// No signature artifact found at the sibling location.
    Missing,
}

/// Paths written by [`write_keypair_to_dir`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeypairFiles {
    /// Path to the cosign-style private-key PEM file.
    pub private: PathBuf,
    /// Path to the matching public-key PEM file.
    pub public: PathBuf,
}

/// Detached signature artifact persisted alongside a published skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSignature {
    /// Schema version.
    pub schema_version: u32,
    /// Artifact kind (`argo.skill.signature`).
    pub kind: String,
    /// Signature algorithm (`ed25519`).
    pub algorithm: String,
    /// Skill name covered by this signature.
    pub name: String,
    /// Skill version covered by this signature.
    pub version: String,
    /// Hex SHA-256 digest of the canonical SKILL.md bytes.
    pub payload_sha256: String,
    /// Base64-encoded 64-byte ed25519 signature over `payload_sha256` bytes.
    pub signature_b64: String,
    /// RFC-3339 timestamp the signature was produced.
    pub signed_at: String,
}

/// Generate a fresh ed25519 keypair using `OsRng`.
///
/// ```
/// let kp = argo_skill::sign::generate_keypair();
/// assert_eq!(kp.verifying_key().to_bytes().len(), 32);
/// ```
pub fn generate_keypair() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

/// Encode a [`SigningKey`] as a cosign-style PEM block. The body is the
/// base64 encoding of the 32-byte ed25519 seed.
///
/// ```
/// let kp = argo_skill::sign::generate_keypair();
/// let pem = argo_skill::sign::encode_private_pem(&kp);
/// assert!(pem.starts_with("-----BEGIN COSIGN PRIVATE KEY-----"));
/// ```
pub fn encode_private_pem(signing: &SigningKey) -> String {
    encode_pem(COSIGN_PRIVATE_PEM_LABEL, &signing.to_bytes())
}

/// Encode a [`VerifyingKey`] as a cosign-style PEM block.
///
/// ```
/// let kp = argo_skill::sign::generate_keypair();
/// let pem = argo_skill::sign::encode_public_pem(&kp.verifying_key());
/// assert!(pem.starts_with("-----BEGIN COSIGN PUBLIC KEY-----"));
/// ```
pub fn encode_public_pem(verifying: &VerifyingKey) -> String {
    encode_pem(COSIGN_PUBLIC_PEM_LABEL, &verifying.to_bytes())
}

/// Parse a PEM-encoded private key written by [`encode_private_pem`].
pub fn decode_private_pem(pem: &str) -> ArgoResult<SigningKey> {
    let body = decode_pem(pem, COSIGN_PRIVATE_PEM_LABEL)?;
    let seed: [u8; 32] = body.as_slice().try_into().map_err(|_| {
        ArgoError::Schema(format!(
            "cosign private key body must decode to 32 bytes, got {}",
            body.len()
        ))
    })?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Parse a PEM-encoded public key written by [`encode_public_pem`].
pub fn decode_public_pem(pem: &str) -> ArgoResult<VerifyingKey> {
    let body = decode_pem(pem, COSIGN_PUBLIC_PEM_LABEL)?;
    let key: [u8; 32] = body.as_slice().try_into().map_err(|_| {
        ArgoError::Schema(format!(
            "cosign public key body must decode to 32 bytes, got {}",
            body.len()
        ))
    })?;
    VerifyingKey::from_bytes(&key)
        .map_err(|e| ArgoError::Schema(format!("cosign public key parse: {e}")))
}

/// Write a fresh keypair to `dir`, returning the absolute paths.
pub fn write_keypair_to_dir(dir: &Path, keypair: &SigningKey) -> ArgoResult<KeypairFiles> {
    fs::create_dir_all(dir)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", dir.display())))?;
    let private = dir.join(COSIGN_PRIVATE_FILE);
    let public = dir.join(COSIGN_PUBLIC_FILE);
    write_secret_file(&private, encode_private_pem(keypair).as_bytes())?;
    write_file(&public, encode_public_pem(&keypair.verifying_key()).as_bytes())?;
    Ok(KeypairFiles { private, public })
}

/// Load a private key from a PEM file on disk.
pub fn load_private_key(path: &Path) -> ArgoResult<SigningKey> {
    let body = fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    decode_private_pem(&body)
}

/// Load a public verifying key from a PEM file on disk.
pub fn load_public_key(path: &Path) -> ArgoResult<VerifyingKey> {
    let body = fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    decode_public_pem(&body)
}

/// Sign the SHA-256 digest of `payload` and return the 64-byte ed25519
/// signature plus the hex-encoded digest that was signed.
pub fn sign_payload(signing: &SigningKey, payload: &[u8]) -> (String, [u8; SIGNATURE_LENGTH]) {
    let digest_hex = sha256_hex(payload);
    let sig = signing.sign(digest_hex.as_bytes());
    (digest_hex, sig.to_bytes())
}

/// Verify a 64-byte ed25519 signature over the SHA-256 digest of `payload`.
pub fn verify_payload(
    verifying: &VerifyingKey,
    payload: &[u8],
    signature_bytes: &[u8],
) -> ArgoResult<()> {
    let sig_arr: [u8; SIGNATURE_LENGTH] = signature_bytes.try_into().map_err(|_| {
        ArgoError::Schema(format!(
            "ed25519 signature must be {SIGNATURE_LENGTH} bytes, got {}",
            signature_bytes.len()
        ))
    })?;
    let sig = Signature::from_bytes(&sig_arr);
    let digest_hex = sha256_hex(payload);
    verifying
        .verify(digest_hex.as_bytes(), &sig)
        .map_err(|e| ArgoError::Schema(format!("ed25519 verify: {e}")))
}

/// Build a [`SkillSignature`] for `payload` signed with `signing`.
pub fn build_signature_artifact(
    name: &str,
    version: &str,
    signing: &SigningKey,
    payload: &[u8],
) -> SkillSignature {
    let (digest_hex, sig_bytes) = sign_payload(signing, payload);
    SkillSignature {
        schema_version: SKILL_SIGNATURE_SCHEMA_VERSION,
        kind: SKILL_SIGNATURE_KIND.into(),
        algorithm: SKILL_SIGNATURE_ALGORITHM.into(),
        name: name.into(),
        version: version.into(),
        payload_sha256: digest_hex,
        signature_b64: STANDARD.encode(sig_bytes),
        signed_at: Utc::now().to_rfc3339(),
    }
}

/// Path the detached signature artifact is written to, relative to the
/// registry root, for `name:tag`.
pub fn signature_artifact_path(root: &Path, name: &str, tag: &str) -> PathBuf {
    root.join("oci").join(name).join(tag).join(SKILL_SIGNATURE_FILE)
}

/// Write a signature artifact to the mock registry sibling location.
pub fn write_signature_artifact(
    registry: &str,
    sig: &SkillSignature,
    tag: &str,
) -> ArgoResult<PathBuf> {
    let root = mock_registry_root(registry)?;
    let path = signature_artifact_path(&root, &sig.name, tag);
    let body = serde_json::to_vec_pretty(sig)
        .map_err(|e| ArgoError::Provider(format!("serialize skill signature: {e}")))?;
    write_file(&path, &body)?;
    Ok(path)
}

/// Read a signature artifact from the mock registry, or `None` if no
/// sibling exists at the requested `name:tag` path.
pub fn read_signature_artifact(
    registry: &str,
    name: &str,
    tag: &str,
) -> ArgoResult<Option<SkillSignature>> {
    let root = mock_registry_root(registry)?;
    let path = signature_artifact_path(&root, name, tag);
    if !path.exists() {
        return Ok(None);
    }
    let body = fs::read_to_string(&path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let sig: SkillSignature = serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    Ok(Some(sig))
}

/// Verify the detached signature for `name:tag` in `registry` against
/// the canonical `payload` bytes and a `verifying` key.
///
/// Returns [`SkillVerifyOutcome::Missing`] when no signature sibling exists,
/// [`SkillVerifyOutcome::Fail`] when the payload digest or signature do
/// not check out, and [`SkillVerifyOutcome::Ok`] on success.
pub fn verify_skill_signature(
    registry: &str,
    name: &str,
    tag: &str,
    verifying: &VerifyingKey,
    payload: &[u8],
) -> ArgoResult<SkillVerifyOutcome> {
    let Some(sig) = read_signature_artifact(registry, name, tag)? else {
        return Ok(SkillVerifyOutcome::Missing);
    };
    if sig.algorithm != SKILL_SIGNATURE_ALGORITHM {
        return Ok(SkillVerifyOutcome::Fail(format!(
            "unsupported algorithm '{}': only {SKILL_SIGNATURE_ALGORITHM} is supported",
            sig.algorithm
        )));
    }
    let computed = sha256_hex(payload);
    if computed != sig.payload_sha256 {
        return Ok(SkillVerifyOutcome::Fail(format!(
            "payload digest mismatch: signed={} actual={}",
            sig.payload_sha256, computed
        )));
    }
    let sig_bytes = STANDARD
        .decode(sig.signature_b64.as_bytes())
        .map_err(|e| ArgoError::Schema(format!("decode signature base64: {e}")))?;
    match verify_payload(verifying, payload, &sig_bytes) {
        Ok(()) => Ok(SkillVerifyOutcome::Ok),
        Err(e) => Ok(SkillVerifyOutcome::Fail(e.to_string())),
    }
}

fn encode_pem(label: &str, body: &[u8]) -> String {
    let encoded = STANDARD.encode(body);
    format!("-----BEGIN {label}-----\n{encoded}\n-----END {label}-----\n")
}

fn decode_pem(pem: &str, label: &str) -> ArgoResult<Vec<u8>> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = pem
        .find(&begin)
        .ok_or_else(|| ArgoError::Schema(format!("missing PEM header '{begin}'")))?;
    let after_header = start + begin.len();
    let end_pos = pem[after_header..]
        .find(&end)
        .ok_or_else(|| ArgoError::Schema(format!("missing PEM footer '{end}'")))?;
    let body = pem[after_header..after_header + end_pos]
        .replace(['\n', '\r', ' '], "");
    STANDARD
        .decode(body.as_bytes())
        .map_err(|e| ArgoError::Schema(format!("decode PEM body: {e}")))
}

fn write_file(path: &Path, bytes: &[u8]) -> ArgoResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ArgoError::Io(format!("create {}: {e}", parent.display())))?;
    }
    fs::write(path, bytes).map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))
}

#[cfg(unix)]
fn write_secret_file(path: &Path, bytes: &[u8]) -> ArgoResult<()> {
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ArgoError::Io(format!("create {}: {e}", parent.display())))?;
    }
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true).mode(0o600);
    let mut file = opts
        .open(path)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", path.display())))?;
    use std::io::Write;
    file.write_all(bytes)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))
}

#[cfg(not(unix))]
fn write_secret_file(path: &Path, bytes: &[u8]) -> ArgoResult<()> {
    write_file(path, bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
