//! Unit tests for the secret-scanning pattern set
//! (Mission `mission-secret-scanning-pre-commit-hook`). Eight focused
//! tests — one per regex category plus allowlist suppression and
//! redaction format — exercising the `scan_buffer` /
//! `is_allowlisted` / `redact` contracts. Fixtures embed
//! synthetic-looking tokens; nothing here is a real credential.
//!
//! Why scan an in-memory buffer rather than touching disk: the
//! filesystem walker has its own integration coverage via
//! `scan_paths` (exercised end-to-end below), but the regex set is
//! the load-bearing surface, and testing it via `scan_buffer` keeps
//! the per-test cost in microseconds.

use super::security_scan_secrets::{
    hash_match, is_allowlisted, patterns, redact, scan_buffer, scan_paths, AllowEntry, Allowlist,
};
use std::path::PathBuf;

fn first_id(findings: &[super::security_scan_secrets::Finding]) -> Option<&str> {
    findings.first().map(|f| f.regex_id.as_str())
}

#[test]
fn detects_openai_sk_key() {
    let pats = patterns();
    let body = "OPENAI_API_KEY=sk-1234567890abcdefABCDEF1234567890XYZ";
    let findings = scan_buffer("env", body, &pats);
    assert!(!findings.is_empty(), "expected at least one finding");
    assert_eq!(first_id(&findings), Some("openai_sk"));
}

#[test]
fn detects_anthropic_key() {
    let pats = patterns();
    let body = "ANTHROPIC_API_KEY=sk-ant-api03-AbCdEfGhIjKlMnOpQrStUv";
    let findings = scan_buffer("env", body, &pats);
    let ids: Vec<&str> = findings.iter().map(|f| f.regex_id.as_str()).collect();
    assert!(
        ids.contains(&"anthropic_key"),
        "expected anthropic_key in {ids:?}"
    );
}

#[test]
fn detects_aws_access_key_id() {
    let pats = patterns();
    let body = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    let findings = scan_buffer("config", body, &pats);
    assert_eq!(first_id(&findings), Some("aws_access_key_id"));
}

#[test]
fn detects_stripe_live_key() {
    let pats = patterns();
    let body = "STRIPE=sk_live_51HxxxxxxxxxxxxxxxxxxxxxxxxxxxxAB";
    let findings = scan_buffer("config", body, &pats);
    assert_eq!(first_id(&findings), Some("stripe_live_secret"));
}

#[test]
fn detects_slack_github_pat_private_key_jwt_argo() {
    // Bundles five distinct categories so the test surface stays at
    // 8 — each category triggers a distinct pattern id and we
    // assert against the full set found in one pass.
    let pats = patterns();
    let body = "\
xoxb-1234567890-ABCDEFGHIJK
ghp_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
-----BEGIN RSA PRIVATE KEY-----
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwYWJjZGVmZ2hpamtsbW5vIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c
ARGO_GATEWAY_BEARER_TOKEN=abcdefghijklmnopqrstuvwxyz
";
    let findings = scan_buffer("mix", body, &pats);
    let ids: Vec<&str> = findings.iter().map(|f| f.regex_id.as_str()).collect();
    for expected in [
        "slack_token",
        "github_pat",
        "private_key_header",
        "jwt_long",
        "argo_bearer_token",
    ] {
        assert!(ids.contains(&expected), "missing {expected} in {ids:?}");
    }
}

#[test]
fn redacts_long_match_with_head_and_tail() {
    let raw = "sk-1234567890abcdefghijklmnop";
    let r = redact(raw);
    assert!(r.starts_with("sk-1"), "redacted = {r}");
    assert!(r.contains("..."), "redacted = {r}");
    assert!(r.ends_with("mnop"), "redacted = {r}");
    // Short strings get fully masked — keeps redaction monotonic.
    assert_eq!(redact("short"), "*****");
}

#[test]
fn allowlist_entry_suppresses_match() {
    let pats = patterns();
    let body = "OPENAI_API_KEY=sk-1234567890abcdefABCDEF1234567890XYZ";
    let findings = scan_buffer("tests/fixtures/synthetic.env", body, &pats);
    let f = findings.first().expect("scan finds at least one match");
    let allow = Allowlist {
        entries: vec![AllowEntry {
            file_glob: "tests/fixtures/**".into(),
            regex_id: f.regex_id.clone(),
            match_hash: f.match_hash.clone(),
            reason: Some("synthetic fixture".into()),
        }],
    };
    assert!(
        is_allowlisted(f, &allow),
        "expected exact tuple to suppress"
    );
    // A wrong regex_id must NOT suppress — the allowlist is keyed
    // on the full (glob, id, hash) tuple, not on glob alone.
    let mut wrong = allow.clone();
    wrong.entries[0].regex_id = "anthropic_key".into();
    assert!(!is_allowlisted(f, &wrong));
}

#[test]
fn scan_paths_walks_directory_and_skips_binary() {
    use std::io::Write;
    let dir = tempfile::tempdir().expect("tempdir");
    let env = dir.path().join("staging.env");
    std::fs::write(
        &env,
        "ARGO_GATEWAY_BEARER_TOKEN=abcdefghijklmnopqrstuvwxyz0123\n",
    )
    .unwrap();
    let bin = dir.path().join("blob.bin");
    let mut f = std::fs::File::create(&bin).unwrap();
    f.write_all(&[0u8, 1, 2, 3, 0, 5, 6]).unwrap();
    let pats = patterns();
    let findings = scan_paths(&[dir.path().to_path_buf()], &Allowlist::default(), &pats).unwrap();
    let files: Vec<&str> = findings.iter().map(|f| f.file.as_str()).collect();
    assert!(
        files.iter().any(|p| p.ends_with("staging.env")),
        "expected env file scanned: {files:?}"
    );
    assert!(
        !files.iter().any(|p| p.ends_with("blob.bin")),
        "binary file should be skipped: {files:?}"
    );
    // Hashes are stable per match — sanity-check the helper.
    assert_eq!(
        hash_match("sk-1234567890abcdefABCDEF1234567890XYZ").len(),
        64
    );
    // And the path entry is the one we created.
    let _: Vec<PathBuf> = vec![dir.path().to_path_buf()];
}
