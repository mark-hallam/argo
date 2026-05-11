//! Unit tests for the `argo wizard onboard` handler. The 8 tests
//! required by the brief, each exercising one slice of the wizard
//! against a `MockPrompt` + a `MockKeyVerifier` so no live HTTP / no
//! stdin is touched. Integration is covered by
//! `tests/wizard_onboard_smoke.rs`.

use super::{
    is_plausible_email, is_valid_worker_name, run_steps, write_operator_block, Answers,
    KeyVerifier, PromptIo,
};
use argo_types::ArgoResult;
use std::cell::RefCell;
use std::path::Path;

struct MockPrompt {
    inputs: RefCell<Vec<String>>,
}

impl MockPrompt {
    fn new(inputs: Vec<&str>) -> Self {
        Self {
            inputs: RefCell::new(inputs.into_iter().map(String::from).collect()),
        }
    }
}

impl PromptIo for MockPrompt {
    fn read_line(&mut self, _prompt: &str) -> ArgoResult<Option<String>> {
        Ok(if self.inputs.borrow().is_empty() {
            None
        } else {
            Some(self.inputs.borrow_mut().remove(0))
        })
    }
    fn write_line(&mut self, _line: &str) -> ArgoResult<()> {
        Ok(())
    }
}

struct MockKeyVerifier {
    good: &'static str,
}

impl KeyVerifier for MockKeyVerifier {
    fn verify(&self, key: &str) -> Result<(), String> {
        if key == self.good {
            Ok(())
        } else {
            Err(format!("mock: '{key}' is not the known-good key"))
        }
    }
}

fn good_answers(tmp: &Path) -> Answers {
    let _ = tmp; // for layout parity with future fixtures
    Answers {
        provider_api_key: Some("sk-ant-good".into()),
        operator_name: Some("Ada Lovelace".into()),
        operator_email: Some("ada@example.com".into()),
        vertical: Some("marketplace-seller-browser".into()),
        worker_name: Some("ada".into()),
        worker_persona: Some("first worker for marketplace-seller-browser".into()),
        skip_key_test: false,
        overwrite_existing: false,
    }
}

fn fake_vertical(cwd: &Path, id: &str) {
    let dir = cwd.join("verticals").join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("vertical.toml"), "[meta]\nid = \"x\"\n").unwrap();
}

#[test]
fn worker_name_regex_matches_brief() {
    assert!(is_valid_worker_name("ada"));
    assert!(is_valid_worker_name("ada_smith-01"));
    assert!(!is_valid_worker_name("Ad")); // capital
    assert!(!is_valid_worker_name("ab")); // too short
    assert!(!is_valid_worker_name("1ada")); // leading digit
    assert!(!is_valid_worker_name(&"a".repeat(32))); // too long
    assert!(is_plausible_email("ada@example.com"));
    assert!(!is_plausible_email("no-at-sign"));
    assert!(!is_plausible_email("a@b"));
}

#[test]
fn non_interactive_happy_path_writes_argo_toml() {
    let tmp = tempfile::tempdir().unwrap();
    fake_vertical(tmp.path(), "marketplace-seller-browser");
    let answers = good_answers(tmp.path());
    let mut io = MockPrompt::new(vec![]);
    let verifier = MockKeyVerifier { good: "sk-ant-good" };
    let outcome = run_steps(
        &mut io,
        &verifier,
        tmp.path(),
        &tmp.path().join("argo.toml"),
        &answers,
        false,
    )
    .unwrap()
    .expect("happy path produces an outcome");
    assert_eq!(outcome.operator_name, "Ada Lovelace");
    assert_eq!(outcome.worker_name, "ada");
    let toml = std::fs::read_to_string(tmp.path().join("argo.toml")).unwrap();
    assert!(toml.contains("[operator]"));
    assert!(toml.contains("ada@example.com"));
}

#[test]
fn non_interactive_bad_input_fails_clearly() {
    let tmp = tempfile::tempdir().unwrap();
    fake_vertical(tmp.path(), "marketplace-seller-browser");
    let mut answers = good_answers(tmp.path());
    answers.worker_name = Some("Ada".into()); // capital — invalid
    let mut io = MockPrompt::new(vec![]);
    let verifier = MockKeyVerifier { good: "sk-ant-good" };
    let err = run_steps(
        &mut io,
        &verifier,
        tmp.path(),
        &tmp.path().join("argo.toml"),
        &answers,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("worker name"), "err = {err}");
    assert!(err.contains("/^[a-z]"), "err = {err}");
}

#[test]
fn existing_operator_block_prompts_and_defaults_no() {
    let tmp = tempfile::tempdir().unwrap();
    fake_vertical(tmp.path(), "marketplace-seller-browser");
    let cfg = tmp.path().join("argo.toml");
    std::fs::write(
        &cfg,
        "[operator]\nname = \"old\"\nemail = \"old@example.com\"\n",
    )
    .unwrap();
    let answers = good_answers(tmp.path());
    let mut io = MockPrompt::new(vec![]);
    let verifier = MockKeyVerifier { good: "sk-ant-good" };
    let out = run_steps(&mut io, &verifier, tmp.path(), &cfg, &answers, false).unwrap();
    assert!(out.is_none(), "default is No; should abort");
    let body = std::fs::read_to_string(&cfg).unwrap();
    assert!(body.contains("\"old\""), "untouched; got {body}");
}

#[test]
fn existing_operator_block_overwritten_when_answers_opt_in() {
    let tmp = tempfile::tempdir().unwrap();
    fake_vertical(tmp.path(), "marketplace-seller-browser");
    let cfg = tmp.path().join("argo.toml");
    std::fs::write(
        &cfg,
        "[operator]\nname = \"old\"\nemail = \"old@example.com\"\n",
    )
    .unwrap();
    let mut answers = good_answers(tmp.path());
    answers.overwrite_existing = true;
    let mut io = MockPrompt::new(vec![]);
    let verifier = MockKeyVerifier { good: "sk-ant-good" };
    let outcome = run_steps(&mut io, &verifier, tmp.path(), &cfg, &answers, false)
        .unwrap()
        .expect("overwrite opt-in proceeds");
    assert_eq!(outcome.operator_name, "Ada Lovelace");
    let body = std::fs::read_to_string(&cfg).unwrap();
    assert!(body.contains("Ada Lovelace"), "body = {body}");
}

#[test]
fn key_verifier_passes_known_good_key() {
    let tmp = tempfile::tempdir().unwrap();
    fake_vertical(tmp.path(), "marketplace-seller-browser");
    let answers = good_answers(tmp.path());
    let verifier = MockKeyVerifier { good: "sk-ant-good" };
    let mut io = MockPrompt::new(vec![]);
    let outcome = run_steps(
        &mut io,
        &verifier,
        tmp.path(),
        &tmp.path().join("argo.toml"),
        &answers,
        false,
    )
    .unwrap()
    .expect("known-good key is accepted");
    assert_eq!(outcome.api_key, "sk-ant-good");
}

#[test]
fn key_verifier_rejects_bad_key() {
    let tmp = tempfile::tempdir().unwrap();
    fake_vertical(tmp.path(), "marketplace-seller-browser");
    let mut answers = good_answers(tmp.path());
    answers.provider_api_key = Some("sk-ant-WRONG".into());
    let verifier = MockKeyVerifier { good: "sk-ant-good" };
    let mut io = MockPrompt::new(vec![]);
    let err = run_steps(
        &mut io,
        &verifier,
        tmp.path(),
        &tmp.path().join("argo.toml"),
        &answers,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("provider key rejected"), "err = {err}");
    assert!(err.contains("WRONG"), "err = {err}");
}

#[test]
fn write_operator_block_preserves_other_sections() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join("argo.toml");
    std::fs::write(
        &cfg,
        "[core]\nprofile = \"prod\"\n\n[budget]\ncap_usd = 100\n",
    )
    .unwrap();
    write_operator_block(&cfg, "Ada", "ada@example.com").unwrap();
    let body = std::fs::read_to_string(&cfg).unwrap();
    assert!(body.contains("[operator]"));
    assert!(body.contains("[core]"));
    assert!(body.contains("profile = \"prod\""));
    assert!(body.contains("[budget]"));
}
