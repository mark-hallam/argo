//! Unit tests for the cli-side policy handlers (Stream
//! `claude-policy-as-code`). The data-model + chain-resolver
//! tests live next to the type definitions in
//! `argo-identity/src/policy_tests.rs`; here we cover the
//! cli-only surface (sha256 + parse_path + applies_to_label).
use super::{applies_to_label, parse_path, sha256_hex};
use argo_identity::policy::PolicyApplies;

#[test]
fn sha256_hex_returns_64_char_hex_and_differs_per_input() {
    let a = sha256_hex("policy");
    let b = sha256_hex("not policy");
    let empty = sha256_hex("");
    // empty-string sha256 is well-known: e3b0c44298fc1c14...
    assert_eq!(empty, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(a.len(), 64);
    assert_eq!(b.len(), 64);
    assert_ne!(a, b);
    assert_ne!(a, empty);
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn parse_path_returns_clear_error_when_file_missing() {
    let err = parse_path(std::path::Path::new("/tmp/nope-policy-test.yaml"))
        .expect_err("missing file");
    assert!(err.to_string().contains("policy: read"));
}

#[test]
fn parse_path_uses_basename_as_policy_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("finance-strict.yaml");
    std::fs::write(
        &path,
        "
applies_to:
  - !role finance-*
rules:
  - action_kind: stripe_charge
    require_signatures: 2
",
    )
    .expect("write");
    let file = parse_path(&path).expect("parse");
    assert_eq!(file.name, "finance-strict");
}

#[test]
fn parse_path_returns_validation_error_on_empty_rules() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.yaml");
    std::fs::write(
        &path,
        "
applies_to:
  - !role any
rules: []
",
    )
    .expect("write");
    let err = parse_path(&path).expect_err("empty rules");
    assert!(err.to_string().contains("rules"));
}

#[test]
fn applies_to_label_renders_role_and_tenant() {
    assert_eq!(
        applies_to_label(&PolicyApplies::Role("finance-*".into())),
        "role=finance-*"
    );
    assert_eq!(
        applies_to_label(&PolicyApplies::Tenant("acme-prod".into())),
        "tenant=acme-prod"
    );
}
