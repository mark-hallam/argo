//! HTTP adapter over the existing `argo wizard onboard` five-step flow.

use super::wizard_onboard::{
    existing_operator_block, is_plausible_email, is_valid_worker_name, provider_err,
    scaffold_first_worker, Answers, KeyVerifier, PromptIo, DEFAULT_VERTICAL,
};
use argo_types::ArgoResult;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[rustfmt::skip]
pub enum WizardStep { AwaitingProviderKey, AwaitingOperatorName, AwaitingOperatorEmail, AwaitingVertical, AwaitingWorkerName, AwaitingWorkerPersona, Complete }

#[derive(Debug, Clone, Serialize)]
#[rustfmt::skip]
pub struct WizardSnapshot { pub step: WizardStep, pub prompt: &'static str, pub complete: bool, pub message: String }

#[derive(Debug, Clone)]
#[rustfmt::skip]
pub struct WizardSession { step: WizardStep, answers: Answers, last_message: String }

impl Default for WizardSession {
    fn default() -> Self {
        Self {
            step: WizardStep::AwaitingProviderKey,
            answers: Answers::default(),
            last_message: "step 2/5  provider — ANTHROPIC_API_KEY".to_string(),
        }
    }
}

impl WizardSession {
    pub fn snapshot(&self, config_path: &Path) -> ArgoResult<WizardSnapshot> {
        if existing_operator_block(config_path)? {
            return Ok(make_snapshot(
                WizardStep::Complete,
                true,
                "operator configuration already exists".to_string(),
            ));
        }
        Ok(make_snapshot(
            self.step,
            self.step == WizardStep::Complete,
            self.last_message.clone(),
        ))
    }

    pub fn reset(&mut self) -> WizardSnapshot {
        *self = Self::default();
        make_snapshot(self.step, false, "wizard reset".to_string())
    }

    pub fn submit<V: KeyVerifier>(
        &mut self,
        answer: &str,
        verifier: &V,
        cwd: &Path,
        config_path: &Path,
    ) -> ArgoResult<WizardSnapshot> {
        let answer = answer.trim();
        if answer.is_empty() {
            return Err(provider_err("wizard http: answer is required"));
        }
        match self.step {
            WizardStep::AwaitingProviderKey => {
                verifier.verify(answer).map_err(|e| {
                    provider_err(format!("wizard onboard: provider key rejected: {e}"))
                })?;
                self.answers.provider_api_key = Some(answer.to_string());
                self.answers.skip_key_test = true;
                self.step = WizardStep::AwaitingOperatorName;
            }
            WizardStep::AwaitingOperatorName => {
                self.answers.operator_name = Some(answer.to_string());
                self.step = WizardStep::AwaitingOperatorEmail;
            }
            WizardStep::AwaitingOperatorEmail => {
                if !is_plausible_email(answer) {
                    return Err(provider_err("wizard http: need user@domain.tld"));
                }
                self.answers.operator_email = Some(answer.to_string());
                self.step = WizardStep::AwaitingVertical;
            }
            WizardStep::AwaitingVertical => {
                self.answers.vertical = Some(if answer == "default" {
                    DEFAULT_VERTICAL.to_string()
                } else {
                    answer.to_string()
                });
                self.step = WizardStep::AwaitingWorkerName;
            }
            WizardStep::AwaitingWorkerName => {
                if !is_valid_worker_name(answer) {
                    return Err(provider_err(
                        "wizard http: worker name must match /^[a-z][a-z0-9_-]{2,30}$/",
                    ));
                }
                self.answers.worker_name = Some(answer.to_string());
                self.step = WizardStep::AwaitingWorkerPersona;
            }
            WizardStep::AwaitingWorkerPersona => {
                self.answers.worker_persona = Some(answer.to_string());
                let mut io = BufferedPrompt;
                let outcome = super::wizard_onboard::run_steps(
                    &mut io,
                    &AlreadyVerified,
                    cwd,
                    config_path,
                    &self.answers,
                    false,
                )?
                .ok_or_else(|| provider_err("wizard http: existing operator block preserved"))?;
                argo_config::cred::set(cwd, "ANTHROPIC_API_KEY", &outcome.api_key)?;
                scaffold_first_worker(
                    cwd,
                    &outcome.worker_name,
                    &outcome.persona,
                    &outcome.vertical,
                )?;
                self.step = WizardStep::Complete;
            }
            WizardStep::Complete => {}
        }
        self.last_message = format!("next: {}", prompt_for(self.step));
        self.snapshot(config_path)
    }
}

fn make_snapshot(step: WizardStep, complete: bool, message: String) -> WizardSnapshot {
    WizardSnapshot {
        step,
        prompt: prompt_for(step),
        complete,
        message,
    }
}

struct BufferedPrompt;

#[rustfmt::skip]
impl PromptIo for BufferedPrompt {
    fn read_line(&mut self, _prompt: &str) -> ArgoResult<Option<String>> { Ok(None) }
    fn write_line(&mut self, _line: &str) -> ArgoResult<()> { Ok(()) }
}

struct AlreadyVerified;

#[rustfmt::skip]
impl KeyVerifier for AlreadyVerified {
    fn verify(&self, _key: &str) -> Result<(), String> { Ok(()) }
}

#[rustfmt::skip]
fn prompt_for(step: WizardStep) -> &'static str { match step { WizardStep::AwaitingProviderKey => "Anthropic API key (sk-ant-...)", WizardStep::AwaitingOperatorName => "operator name", WizardStep::AwaitingOperatorEmail => "operator email", WizardStep::AwaitingVertical => "vertical (default marketplace-seller-browser)", WizardStep::AwaitingWorkerName => "worker name (lowercase, 3-31 chars)", WizardStep::AwaitingWorkerPersona => "persona summary (one line)", WizardStep::Complete => "Setup complete" } }

fn sessions() -> &'static Mutex<HashMap<String, WizardSession>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, WizardSession>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn with_session<T>(
    tenant: &str,
    f: impl FnOnce(&mut WizardSession) -> ArgoResult<T>,
) -> ArgoResult<T> {
    let mut guard = sessions()
        .lock()
        .map_err(|_| provider_err("wizard http: session lock poisoned"))?;
    f(guard.entry(tenant.to_string()).or_default())
}

pub fn current(tenant: &str, config_path: &Path) -> ArgoResult<WizardSnapshot> {
    with_session(tenant, |s| s.snapshot(config_path))
}

pub fn reset(tenant: &str) -> ArgoResult<WizardSnapshot> {
    with_session(tenant, |s| Ok(s.reset()))
}

pub fn submit_answer<V: KeyVerifier>(
    tenant: &str,
    answer: &str,
    verifier: &V,
    cwd: &Path,
    config_path: &Path,
) -> ArgoResult<WizardSnapshot> {
    with_session(tenant, |s| s.submit(answer, verifier, cwd, config_path))
}

pub fn default_config_path(cwd: &Path) -> PathBuf {
    cwd.join("argo.toml")
}

#[cfg(test)]
#[path = "wizard_onboard_http_tests.rs"]
mod tests;
