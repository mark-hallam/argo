//! Unit coverage for the CLI-side `session approve` handler
//! (mission-pm-approval-target-scope).
//!
//! Store-layer behaviour (target_glob resolution, most-specific
//! match, target-mismatch error, migration idempotence) is
//! exercised in `crates/argo-store/src/grants_tests.rs`. The
//! cases below cover the handler-specific surface that survives
//! without a real `~/.argo` directory: input parsing.
#![cfg(test)]

use super::parse_session_id;

#[test]
fn parse_session_id_accepts_uuid_v7() {
    // uuid::Uuid::now_v7 is what `add_session_grant` issues, so the
    // CLI handler must round-trip its string form back into a
    // SessionId without mutation.
    let original = uuid::Uuid::now_v7();
    let parsed = parse_session_id(&original.to_string()).expect("uuid v7 must parse");
    assert_eq!(parsed.0, original);
}

#[test]
fn parse_session_id_rejects_garbage_with_schema_error() {
    // The handler maps Uuid::parse_str's error onto
    // ArgoError::Schema so the CLI surface reports "invalid
    // session id" rather than leaking the underlying parser
    // crate's wording.
    let err = parse_session_id("not-a-uuid").expect_err("garbage must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid session id"),
        "expected user-facing message, got: {msg}"
    );
    assert!(
        msg.contains("not-a-uuid"),
        "error must echo the operator's input, got: {msg}"
    );
}

#[test]
fn parse_session_id_rejects_empty_string() {
    // Empty string is an easy way for an operator to fat-finger
    // `argo session approve "" --tool ...`; refuse early rather
    // than open a store and discover no rows.
    let err = parse_session_id("").expect_err("empty input must fail");
    assert!(err.to_string().contains("invalid session id"));
}
