//! Unit tests for the J3 JWT verifier. Sibling-file split keeps
//! `jwt.rs` under the brief's 200-LOC budget.
//!
//! Tests share one `#[test]` per concern but mutate process-global
//! env vars (`ARGO_INBOX_WS_JWT_PUBKEY*`). To avoid cargo's parallel
//! runner racing the env, every test sets+removes its own env var in
//! a single body; we never split a "set" and "verify" across tests.

use super::*;
use base64::Engine;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

const PUBKEY_ENV: &str = "ARGO_INBOX_WS_JWT_PUBKEY";
const PUBKEY_PEM_ENV: &str = "ARGO_INBOX_WS_JWT_PUBKEY_PEM";

fn fresh_keypair_and_set_env() -> SigningKey {
    let signing = SigningKey::generate(&mut OsRng);
    let vk = signing.verifying_key();
    let b64 = base64::engine::general_purpose::STANDARD.encode(vk.to_bytes());
    std::env::set_var(PUBKEY_ENV, b64);
    std::env::remove_var(PUBKEY_PEM_ENV);
    signing
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[test]
fn round_trip_verifies_and_yields_claims() {
    let signing = fresh_keypair_and_set_env();
    let exp = now() + 600;
    let claims = Claims {
        sub: "op-1".into(),
        tnt: "tenant-a".into(),
        iat: now(),
        exp,
    };
    let token = sign_jwt_for_test(&signing, &claims);
    let decoded = verify_jwt(&token, now()).expect("good token verifies");
    assert_eq!(decoded.sub, "op-1");
    assert_eq!(decoded.tnt, "tenant-a");
    assert_eq!(decoded.exp, exp);
    std::env::remove_var(PUBKEY_ENV);
}

#[test]
fn rejects_when_no_pubkey_configured() {
    std::env::remove_var(PUBKEY_ENV);
    std::env::remove_var(PUBKEY_PEM_ENV);
    // Even a syntactically perfect token must fail closed.
    let signing = SigningKey::generate(&mut OsRng);
    let token = sign_jwt_for_test(
        &signing,
        &Claims {
            sub: "x".into(),
            tnt: "y".into(),
            iat: now(),
            exp: now() + 60,
        },
    );
    assert_eq!(verify_jwt(&token, now()), Err(JwtError::NoPubkey));
}

#[test]
fn rejects_expired_token() {
    let signing = fresh_keypair_and_set_env();
    let claims = Claims {
        sub: "op-1".into(),
        tnt: "tenant-a".into(),
        iat: now() - 7200,
        exp: now() - 60,
    };
    let token = sign_jwt_for_test(&signing, &claims);
    assert_eq!(verify_jwt(&token, now()), Err(JwtError::Expired));
    std::env::remove_var(PUBKEY_ENV);
}

#[test]
fn rejects_tampered_signature() {
    let signing = fresh_keypair_and_set_env();
    let claims = Claims {
        sub: "op-1".into(),
        tnt: "tenant-a".into(),
        iat: now(),
        exp: now() + 60,
    };
    let token = sign_jwt_for_test(&signing, &claims);
    // Replace the entire signature segment with one made by a
    // different key over the same input. Same length, valid
    // base64url, but won't verify against the configured pubkey.
    let other = SigningKey::generate(&mut OsRng);
    let mut parts = token.split('.');
    let h = parts.next().unwrap();
    let p = parts.next().unwrap();
    let signing_input = format!("{h}.{p}");
    use ed25519_dalek::Signer;
    let bad_sig = other.sign(signing_input.as_bytes());
    let bad_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bad_sig.to_bytes());
    let tampered = format!("{h}.{p}.{bad_b64}");
    assert_eq!(verify_jwt(&tampered, now()), Err(JwtError::BadSignature));
    std::env::remove_var(PUBKEY_ENV);
}

#[test]
fn rejects_unsupported_algorithm() {
    let _ = fresh_keypair_and_set_env();
    // Hand-craft a token with alg=HS256 — the signature won't matter
    // because the alg gate fires before signature verification.
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{}");
    let sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0u8; 64]);
    let token = format!("{header}.{payload}.{sig}");
    assert_eq!(verify_jwt(&token, now()), Err(JwtError::UnsupportedAlg));
    std::env::remove_var(PUBKEY_ENV);
}

#[test]
fn rejects_malformed_token_shape() {
    let _ = fresh_keypair_and_set_env();
    assert_eq!(verify_jwt("just.two", now()), Err(JwtError::Malformed));
    assert_eq!(
        verify_jwt("one.two.three.four", now()),
        Err(JwtError::Malformed)
    );
    assert_eq!(verify_jwt("", now()), Err(JwtError::Malformed));
    // Non-base64url in any segment surfaces as `Base64`.
    assert_eq!(
        verify_jwt("!!!.!!!.!!!", now()),
        Err(JwtError::Base64)
    );
    std::env::remove_var(PUBKEY_ENV);
}

#[test]
fn pem_pubkey_path_verifies() {
    let signing = SigningKey::generate(&mut OsRng);
    let vk = signing.verifying_key();
    let b64 = base64::engine::general_purpose::STANDARD.encode(vk.to_bytes());
    let pem = format!(
        "-----BEGIN ARGO RELEASE PUBKEY-----\n{b64}\n-----END ARGO RELEASE PUBKEY-----\n"
    );
    std::env::remove_var(PUBKEY_ENV);
    std::env::set_var(PUBKEY_PEM_ENV, pem);
    let claims = Claims {
        sub: "op-1".into(),
        tnt: "t".into(),
        iat: now(),
        exp: now() + 60,
    };
    let token = sign_jwt_for_test(&signing, &claims);
    assert!(verify_jwt(&token, now()).is_ok());
    std::env::remove_var(PUBKEY_PEM_ENV);
}
