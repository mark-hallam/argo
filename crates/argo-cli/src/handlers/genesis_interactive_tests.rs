//! Tests for the `argo genesis interactive` cli wrapper. The Q&A
//! loop has its own test file; here we cover the YAML-loading
//! path that backs `--non-interactive`.
use super::load_dialogue_yaml;

fn write_tmp(name: &str, body: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = dir.path().join(name);
    std::fs::write(&p, body).expect("write yaml");
    // Leak the tempdir so callers can read the file path; the OS
    // cleans up at process exit which is fine for tests.
    std::mem::forget(dir);
    p
}

#[test]
fn load_dialogue_yaml_parses_full_payload() {
    let yaml = "
role: marketing intern
name: Maya
industry: B2B SaaS
duties:
  - Track competitor product launches
  - Draft tweets and LinkedIn posts
  - Respond to inbound demo requests
channels:
  - slack-dm
  - email
spend_cap_per_day_usd: 50
";
    let path = write_tmp("dialogue.yaml", yaml);
    let dialogue = load_dialogue_yaml(&path).expect("parse");
    assert_eq!(dialogue.role, "marketing intern");
    assert_eq!(dialogue.name, "Maya");
    assert_eq!(dialogue.industry, "B2B SaaS");
    assert_eq!(dialogue.duties.len(), 3);
    assert_eq!(dialogue.channels, vec!["slack-dm", "email"]);
    assert!((dialogue.spend_cap_per_day_usd - 50.0).abs() < f64::EPSILON);
}

#[test]
fn load_dialogue_yaml_omits_industry_when_unset() {
    let yaml = "
role: support
name: Sam
duties: [tickets]
channels: [email]
spend_cap_per_day_usd: 5
";
    let path = write_tmp("nodialog.yaml", yaml);
    let dialogue = load_dialogue_yaml(&path).expect("parse");
    assert_eq!(dialogue.industry, "");
}

#[test]
fn load_dialogue_yaml_returns_clear_error_on_missing_file() {
    let err = load_dialogue_yaml(std::path::Path::new("/nope/missing.yaml"))
        .expect_err("missing file");
    let s = err.to_string();
    assert!(s.contains("genesis interactive"));
    assert!(s.contains("missing.yaml"));
}

#[test]
fn load_dialogue_yaml_returns_clear_error_on_invalid_yaml() {
    let path = write_tmp("bad.yaml", "this is: not\n  - valid: yaml");
    let err = load_dialogue_yaml(&path).expect_err("invalid yaml");
    assert!(err.to_string().contains("parse"));
}
