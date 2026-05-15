use super::*;

struct FakeVerifier;

impl KeyVerifier for FakeVerifier {
    fn verify(&self, key: &str) -> Result<(), String> {
        key.starts_with("sk-ant-")
            .then_some(())
            .ok_or_else(|| "bad key".to_string())
    }
}

#[test]
fn session_walks_all_five_steps_and_scaffolds_worker() {
    let tmp = tempfile::tempdir().expect("tmp");
    let cwd = tmp.path();
    std::fs::create_dir_all(cwd.join("verticals/marketplace-seller-browser")).expect("vertical");
    let config = default_config_path(cwd);
    let verifier = FakeVerifier;
    let tenant = "test-walk";

    assert_eq!(
        current(tenant, &config).expect("current").step,
        WizardStep::AwaitingProviderKey
    );
    assert_eq!(
        submit_answer(tenant, "sk-ant-test", &verifier, cwd, &config)
            .expect("key")
            .step,
        WizardStep::AwaitingOperatorName
    );
    assert_eq!(
        submit_answer(tenant, "Ada Lovelace", &verifier, cwd, &config)
            .expect("name")
            .step,
        WizardStep::AwaitingOperatorEmail
    );
    assert_eq!(
        submit_answer(tenant, "ada@example.com", &verifier, cwd, &config)
            .expect("email")
            .step,
        WizardStep::AwaitingVertical
    );
    assert_eq!(
        submit_answer(
            tenant,
            "marketplace-seller-browser",
            &verifier,
            cwd,
            &config
        )
        .expect("vertical")
        .step,
        WizardStep::AwaitingWorkerName
    );
    assert_eq!(
        submit_answer(tenant, "maya", &verifier, cwd, &config)
            .expect("worker")
            .step,
        WizardStep::AwaitingWorkerPersona
    );
    let done =
        submit_answer(tenant, "Marketplace teammate", &verifier, cwd, &config).expect("persona");
    assert_eq!(done.step, WizardStep::Complete);
    assert!(config.exists());
    assert!(cwd.join("workers/maya/worker.toml").exists());
}

#[test]
fn invalid_email_is_rejected_before_write() {
    let tmp = tempfile::tempdir().expect("tmp");
    let cwd = tmp.path();
    let config = default_config_path(cwd);
    let verifier = FakeVerifier;
    let tenant = "test-email";
    submit_answer(tenant, "sk-ant-test", &verifier, cwd, &config).expect("key");
    submit_answer(tenant, "Ada", &verifier, cwd, &config).expect("name");

    let err = submit_answer(tenant, "ada", &verifier, cwd, &config).expect_err("email");
    assert!(err.to_string().contains("user@domain.tld"));
    assert!(!config.exists());
}

#[test]
fn invalid_provider_key_stays_on_provider_step() {
    let tmp = tempfile::tempdir().expect("tmp");
    let cwd = tmp.path();
    let config = default_config_path(cwd);
    let tenant = "test-key";

    let err = submit_answer(tenant, "not-a-key", &FakeVerifier, cwd, &config).expect_err("key");
    assert!(err.to_string().contains("provider key rejected"));
    assert_eq!(
        current(tenant, &config).expect("current").step,
        WizardStep::AwaitingProviderKey
    );
}

#[test]
fn existing_operator_block_reports_complete() {
    let tmp = tempfile::tempdir().expect("tmp");
    let config = default_config_path(tmp.path());
    crate::handlers::wizard_onboard::write_operator_block(&config, "Ada", "ada@example.com")
        .expect("operator");

    let snap = current("test-complete", &config).expect("current");
    assert_eq!(snap.step, WizardStep::Complete);
    assert!(snap.complete);
}

#[test]
fn tenants_have_independent_sessions() {
    let tmp = tempfile::tempdir().expect("tmp");
    let cwd = tmp.path();
    let config = default_config_path(cwd);
    submit_answer("tenant-a", "sk-ant-test", &FakeVerifier, cwd, &config).expect("key");

    assert_eq!(
        current("tenant-a", &config).expect("tenant a").step,
        WizardStep::AwaitingOperatorName
    );
    assert_eq!(
        current("tenant-b", &config).expect("tenant b").step,
        WizardStep::AwaitingProviderKey
    );
}

#[test]
fn reset_returns_to_provider_step() {
    let tenant = "test-reset";
    let snap = reset(tenant).expect("reset");
    assert_eq!(snap.step, WizardStep::AwaitingProviderKey);
    assert!(!snap.complete);
}
