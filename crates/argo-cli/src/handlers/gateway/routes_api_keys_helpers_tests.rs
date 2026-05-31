//! Unit tests for `routes_api_keys_helpers` — the pure create-path
//! helpers: token minting, hashing, masked-prefix derivation, body
//! parsing + validation, expiry resolution, and metadata-only row JSON.

use super::*;
use argo_store::api_key::ApiKeyRow;

fn req_with_body(body: &str) -> String {
    format!("POST /api/v1/api-keys HTTP/1.1\r\nHost: x\r\n\r\n{body}")
}

#[test]
fn mint_token_is_high_entropy_and_prefixed() {
    let a = mint_token();
    let b = mint_token();
    assert!(a.starts_with("sk_live_"));
    assert_ne!(a, b, "two mints must differ");
    assert!(a.len() > 30, "32 random bytes base64url ≈ 43 chars + prefix");
}

#[test]
fn sha256_hex_is_stable_and_not_the_input() {
    let h = sha256_hex("sk_live_example");
    assert_eq!(h.len(), 64, "sha-256 hex is 64 chars");
    assert_eq!(h, sha256_hex("sk_live_example"), "deterministic");
    assert_ne!(h, "sk_live_example", "hash is not the plaintext");
}

#[test]
fn masked_prefix_hides_the_middle() {
    let token = "sk_live_abcdefghijklmnopqrstuvwxyz1234";
    let p = masked_prefix(token);
    assert!(p.contains("..."), "prefix is masked: {p}");
    assert!(p.starts_with("sk_live_"), "{p}");
    assert_ne!(p, token, "masked prefix is never the full token");
    assert!(p.len() < token.len(), "shorter than the token");
}

#[test]
fn parse_create_happy_path() {
    let body = r#"{"name":"CI","scopes":["read:workers"],"cohort_ids":["c1"],"all_cohorts":false,"expiry":"never"}"#;
    let c = parse_create(&req_with_body(body)).expect("parsed");
    assert_eq!(c.name, "CI");
    assert_eq!(c.scopes, vec!["read:workers".to_string()]);
    assert_eq!(c.cohort_ids, vec!["c1".to_string()]);
    assert!(!c.all_cohorts);
    assert_eq!(c.expires_at, None);
}

#[test]
fn parse_create_rejects_missing_name() {
    let body = r#"{"scopes":[],"cohort_ids":[],"all_cohorts":true,"expiry":"never"}"#;
    let (status, _b) = parse_create(&req_with_body(body)).unwrap_err();
    assert_eq!(status, "400 Bad Request");
}

#[test]
fn parse_create_rejects_unknown_scope() {
    let body = r#"{"name":"x","scopes":["delete:universe"],"cohort_ids":[],"all_cohorts":true,"expiry":"never"}"#;
    let (status, _b) = parse_create(&req_with_body(body)).unwrap_err();
    assert_eq!(status, "400 Bad Request");
}

#[test]
fn parse_create_resolves_relative_expiry() {
    let body = r#"{"name":"x","scopes":[],"cohort_ids":[],"all_cohorts":true,"expiry":"30d"}"#;
    let c = parse_create(&req_with_body(body)).expect("parsed");
    assert!(c.expires_at.is_some(), "30d resolves to an absolute timestamp");
}

#[test]
fn parse_create_accepts_custom_expiry() {
    let body = r#"{"name":"x","scopes":[],"cohort_ids":[],"all_cohorts":true,"expiry":{"custom":"2027-01-01T00:00:00Z"}}"#;
    let c = parse_create(&req_with_body(body)).expect("parsed");
    assert_eq!(c.expires_at.as_deref(), Some("2027-01-01T00:00:00Z"));
}

#[test]
fn row_to_json_is_metadata_only() {
    let row = ApiKeyRow {
        id: "key_1".into(),
        tenant_id: "t".into(),
        name: "CI".into(),
        prefix: "sk_live_a4b8...c2e1".into(),
        scopes_json: "[\"read:workers\"]".into(),
        cohort_ids_json: "[]".into(),
        all_cohorts: true,
        created_at: "2026-06-01T00:00:00Z".into(),
        last_used_at: None,
        expires_at: None,
        revoked: false,
    };
    let j = row_to_json(&row);
    assert_eq!(j["id"], "key_1");
    assert_eq!(j["prefix"], "sk_live_a4b8...c2e1");
    assert!(j["scopes"].is_array());
    assert_eq!(j["scopes"][0], "read:workers");
    assert!(j["cohort_ids"].as_array().unwrap().is_empty());
    assert_eq!(j["all_cohorts"], true);
    assert_eq!(j["last_used_at"], serde_json::Value::Null);
    // No secret-derived fields ever.
    assert!(j.get("token").is_none());
    assert!(j.get("token_hash").is_none());
}

#[test]
fn row_to_json_recovers_from_malformed_scope_json() {
    let row = ApiKeyRow {
        id: "key_1".into(),
        tenant_id: "t".into(),
        name: "CI".into(),
        prefix: "p".into(),
        scopes_json: "not json".into(),
        cohort_ids_json: "also not json".into(),
        all_cohorts: false,
        created_at: "now".into(),
        last_used_at: None,
        expires_at: None,
        revoked: false,
    };
    let j = row_to_json(&row);
    assert!(j["scopes"].as_array().unwrap().is_empty(), "malformed → []");
    assert!(j["cohort_ids"].as_array().unwrap().is_empty(), "malformed → []");
}

#[test]
fn new_key_id_is_prefixed_and_unique() {
    let a = new_key_id();
    let b = new_key_id();
    assert!(a.starts_with("key_"));
    assert_ne!(a, b);
}
