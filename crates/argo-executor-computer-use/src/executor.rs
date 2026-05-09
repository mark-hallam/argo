//! Public `ComputerUseExecutor` orchestrator. Wires
//! profile + budget + pool + audit + the Anthropic CU loop into
//! one `run()` entry point. The `argo-cli` Stream A stub
//! (`worker_actions_executor_stubs::ComputerUseExecutor`) is
//! replaced by a thin shim that calls into this crate.
//!
//! Outcome polarity:
//!
//! - `Outcome::Completed` — model emitted `stop_success` and the
//!   loop captured a result. The dispatch shim auto-completes
//!   the action with the embedded `result_json`.
//! - `Outcome::HumanHandoff` — budget exhausted, model emitted
//!   `stop_failure`, or a recoverable HTTP failure surfaced. The
//!   ledger row stays at `executing` and the operator finishes
//!   manually.

use crate::anthropic::{AnthropicClient, CuStepRequest, CuStepResponse, StopReason};
use crate::audit::{AuditLog, AuditRow};
use crate::budget::{BudgetTracker, BudgetVerdict};
use crate::config::CuConfig;
use crate::e2b::{E2bClient, Sandbox};
use crate::executor_types::default_bundle_dir;
use crate::pool::{PoolKey, SandboxPool};
use crate::profile::{Profile, ProfileRegistry};
use crate::restart::{self, emit_restart_event, RestartCoordinator, RestartDecision};
use crate::CuError;
use argo_skill::SkillStore;
use std::sync::Arc;
use std::time::Duration;

pub use crate::executor_types::{Outcome, RunRequest, RunResult};

/// Top-level executor. Cloneable — internals are `Arc`-shared so
/// the same instance can drive concurrent dispatches across
/// workers.
#[derive(Debug, Clone)]
pub struct ComputerUseExecutor {
    e2b: E2bClient,
    anthropic_template: AnthropicClientTemplate,
    pool: SandboxPool,
    profiles: ProfileRegistry,
    /// Optional skill-learning hook (Stream
    /// `claude-cu-skill-learn-wire`).
    ///
    /// When set, a successful run proposes a `SkillCandidate`
    /// through `SkillStore::propose_for_review` so a human can
    /// approve the learned skill before it goes live. Errors are
    /// logged + swallowed: a learning-loop failure never fails
    /// the action.
    skill_store: Option<Arc<SkillStore>>,
    /// Per-action threshold above which a successful trajectory
    /// is worth proposing as a learned skill. Caller can override
    /// via [`ComputerUseExecutor::with_skill_threshold`].
    skill_threshold: f64,
}

/// Template for building per-profile Anthropic clients (each
/// profile may pin a different model). Cheaper than holding one
/// client per profile up-front.
#[derive(Debug, Clone)]
struct AnthropicClientTemplate {
    base_url: String,
    api_key: String,
    per_request_timeout_seconds: u32,
}

impl ComputerUseExecutor {
    /// Build from a fully-resolved `CuConfig` + the loaded
    /// profile registry. Used by tests + the `from_env` shim.
    #[must_use]
    pub fn new(config: CuConfig, profiles: ProfileRegistry) -> Self {
        let e2b = E2bClient::new(
            config.e2b_base_url.clone(),
            config.e2b_api_key.clone(),
            config.per_request_timeout_seconds,
        )
        .unwrap_or_else(|_| {
            // Build with a default reqwest client — the only
            // way `new` fails is reqwest tls config; recovery
            // is to fall back to a no-op client that errors at
            // first call. Safe because `new` is only fallible
            // in pathological tls envs.
            E2bClient::new(
                "http://localhost:1".into(),
                "invalid".into(),
                1,
            )
            .unwrap_or_else(|_| panic!("reqwest cannot build any client"))
        });
        let pool = SandboxPool::new(e2b.clone());
        Self {
            e2b,
            anthropic_template: AnthropicClientTemplate {
                base_url: config.anthropic_base_url,
                api_key: config.anthropic_api_key,
                per_request_timeout_seconds: config.per_request_timeout_seconds,
            },
            pool,
            profiles,
            skill_store: None,
            skill_threshold: crate::learn::DEFAULT_THRESHOLD,
        }
    }

    /// Attach a skill-learning hook. When set, a successful CU
    /// run proposes a `SkillCandidate` for human review via
    /// `SkillStore::propose_for_review`. Failures are logged +
    /// swallowed so the action's outcome is unaffected.
    #[must_use]
    pub fn with_skill_store(mut self, store: Arc<SkillStore>) -> Self {
        self.skill_store = Some(store);
        self
    }

    /// Override the per-action skill-proposal threshold. Default
    /// is [`crate::learn::DEFAULT_THRESHOLD`] (0.8). Used by
    /// tenants who want a stricter/looser bar.
    #[must_use]
    pub fn with_skill_threshold(mut self, threshold: f64) -> Self {
        self.skill_threshold = threshold;
        self
    }

    /// Convenience: build from env + load the profiles
    /// registry. Falls back to the built-in
    /// `marketplace_browser` profile when the registry path is
    /// absent (e.g. running outside a workspace).
    pub fn from_env() -> Result<Self, CuError> {
        let config = CuConfig::from_env()?;
        let mut registry = ProfileRegistry::load_from_path(&config.profiles_path)?;
        if registry.is_empty() {
            registry = ProfileRegistry::with_builtin_marketplace_browser();
        }
        Ok(Self::new(config, registry))
    }

    /// Public reference to the loaded profile registry. Used by
    /// the dispatch shim to surface profile-resolution errors
    /// before opening a sandbox.
    #[must_use]
    pub fn profiles(&self) -> &ProfileRegistry {
        &self.profiles
    }

    /// Run the full CU loop for one action. Caller fills in the
    /// request; this method orchestrates pool acquire, budget
    /// tracking, screenshot+audit per step, and outcome
    /// classification.
    pub async fn run(&self, req: RunRequest) -> Result<Outcome, CuError> {
        let profile = self.profiles.get(&req.profile_name)?.clone();
        let bundle_dir = req
            .bundle_dir
            .clone()
            .unwrap_or_else(|| default_bundle_dir(&req));
        let audit = AuditLog::create(bundle_dir.clone())?;

        let pool_key = PoolKey {
            tenant_id: req.tenant_id.clone(),
            persona_id: req.persona_id.clone(),
            profile: req.profile_name.clone(),
        };
        let sandbox = self
            .pool
            .acquire(pool_key.clone(), &profile.sandbox_template)
            .await?;

        let mut tracker = BudgetTracker::new(&profile);
        if let Some(usd) = req.budget_usd {
            // Profile-pinned tokens-per-dollar: a future TODO is
            // to read this from the model price registry; for
            // now we use a fixed conservative ratio so the cap
            // is meaningful even before that lands.
            tracker = tracker.with_usd_cap(usd, 100_000);
        }

        let outcome = self.run_loop(&req, &profile, &sandbox, &audit, &mut tracker).await;
        self.pool.release(&pool_key).await;
        let final_outcome = outcome.map(|o| match o {
            LoopExit::Success(result) => Outcome::Completed {
                result,
                bundle_dir,
                budget: tracker.snapshot(),
            },
            LoopExit::Handoff(reason) => Outcome::HumanHandoff {
                reason,
                bundle_dir,
                budget: tracker.snapshot(),
            },
        });
        // Stream `claude-cu-skill-learn-wire`: if the run
        // succeeded + a skill store is attached + a session id
        // is plumbed through, propose a `SkillCandidate` for
        // human review. Errors are logged + swallowed inside the
        // helper so they never fail the action.
        if let (Ok(ref outcome_ref), Some(store), Some(session)) =
            (&final_outcome, &self.skill_store, req.session_id)
        {
            let _ = crate::learn::maybe_propose(
                Some(store.as_ref()),
                &req,
                outcome_ref,
                session,
                self.skill_threshold,
            )
            .await;
        }
        final_outcome
    }

    async fn run_loop(
        &self,
        req: &RunRequest,
        profile: &Profile,
        sandbox: &Sandbox,
        audit: &AuditLog,
        tracker: &mut BudgetTracker,
    ) -> Result<LoopExit, CuError> {
        let anthropic = self.anthropic_for(profile)?;
        let mut prior_actions: Vec<String> = Vec::new();
        let mut last_text: Option<String> = None;
        let mut sandbox = sandbox.clone();
        let mut system_prompt = req.system_prompt.clone();
        let mut coordinator = RestartCoordinator::new(profile.restart.clone());

        // Step 0: capture the initial screenshot before sending
        // the first message.
        let mut screenshot = self.e2b.screenshot(&sandbox).await?;

        loop {
            let resp = anthropic
                .step(CuStepRequest {
                    system_prompt: &system_prompt,
                    screenshot_png: &screenshot,
                    prior_actions: &prior_actions,
                    screen_width: profile.screen_width,
                    screen_height: profile.screen_height,
                })
                .await;
            let resp = match resp {
                Ok(r) => r,
                Err(e) if e.is_recoverable_to_handoff() => {
                    match self
                        .handle_restart(
                            &e,
                            "anthropic_step",
                            profile,
                            &mut coordinator,
                            &mut sandbox,
                            &mut screenshot,
                            &mut system_prompt,
                            &req.system_prompt,
                        )
                        .await?
                    {
                        Some(handoff) => return Ok(LoopExit::Handoff(handoff)),
                        None => continue,
                    }
                }
                Err(e) => return Err(e),
            };

            match resp {
                CuStepResponse::Stop {
                    reason,
                    text,
                    tokens_used,
                } => {
                    let verdict = tracker.charge_step(tokens_used);
                    if matches!(verdict, BudgetVerdict::Exhausted { .. }) {
                        return Ok(LoopExit::Handoff(stringify_verdict(verdict)));
                    }
                    last_text = text.or(last_text);
                    return Ok(match reason {
                        StopReason::Success => LoopExit::Success(RunResult {
                            final_text: last_text,
                            actions_taken: prior_actions,
                            tokens_used: tracker.tokens_used(),
                        }),
                        StopReason::Failure => LoopExit::Handoff(format!(
                            "model emitted stop_failure: {}",
                            last_text.unwrap_or_default()
                        )),
                    });
                }
                CuStepResponse::ToolCall {
                    action,
                    tokens_used,
                } => {
                    let verdict = tracker.charge_step(tokens_used);
                    if let BudgetVerdict::Exhausted { .. } = verdict {
                        return Ok(LoopExit::Handoff(stringify_verdict(verdict)));
                    }
                    let summary = action.summary();
                    let result = match self.e2b.execute_action(&sandbox, &action).await {
                        Ok(r) => r,
                        Err(e) if e.is_recoverable_to_handoff() => {
                            match self
                                .handle_restart(
                                    &e,
                                    "sandbox_action",
                                    profile,
                                    &mut coordinator,
                                    &mut sandbox,
                                    &mut screenshot,
                                    &mut system_prompt,
                                    &req.system_prompt,
                                )
                                .await?
                            {
                                Some(handoff) => {
                                    return Ok(LoopExit::Handoff(format!(
                                        "{handoff} (step {})",
                                        tracker.step()
                                    )))
                                }
                                None => continue,
                            }
                        }
                        Err(e) => return Err(e),
                    };
                    if let Some(t) = result.text.clone() {
                        last_text = Some(t);
                    }
                    // Capture a fresh screenshot for next step.
                    screenshot = match result.screenshot_b64.as_deref() {
                        Some(_b64) => self.e2b.screenshot(&sandbox).await?,
                        None => self.e2b.screenshot(&sandbox).await?,
                    };
                    let relpath = audit.save_screenshot(tracker.step(), &screenshot)?;
                    audit.append(&AuditRow {
                        step: tracker.step(),
                        at: chrono::Utc::now().to_rfc3339(),
                        action,
                        result,
                        screenshot_relpath: relpath,
                        tokens_used,
                        budget: tracker.snapshot(),
                    })?;
                    prior_actions.push(format!("step {}: {summary}", tracker.step()));
                    // Take a snapshot at the configured cadence so a
                    // SnapshotResetRetry has fresh state to seed from.
                    if coordinator.should_snapshot(tracker.step()) {
                        coordinator.record_snapshot(restart::take_snapshot(
                            &screenshot,
                            tracker.step(),
                            &req.system_prompt,
                            &prior_actions,
                        ));
                    }
                }
            }
        }
    }

    /// Consult the restart coordinator on a transient failure. Returns
    /// `Ok(None)` when the loop should retry (state has already been
    /// reset / context restored), `Ok(Some(reason))` when the caller
    /// should fall through to handoff with the given reason. Errors
    /// only on a fresh sandbox open / restore failure.
    #[allow(clippy::too_many_arguments)]
    async fn handle_restart(
        &self,
        err: &CuError,
        stage: &'static str,
        profile: &Profile,
        coordinator: &mut RestartCoordinator,
        sandbox: &mut Sandbox,
        screenshot: &mut Vec<u8>,
        system_prompt: &mut String,
        base_system_prompt: &str,
    ) -> Result<Option<String>, CuError> {
        match coordinator.should_restart(err) {
            RestartDecision::Continue => Ok(Some(format!("{stage} failed: {err}"))),
            RestartDecision::Bail => {
                emit_restart_event(stage, coordinator.attempts(), Some(false));
                Ok(Some(format!(
                    "{stage} failed (retries exhausted): {err}"
                )))
            }
            RestartDecision::Retry { with_context } => {
                emit_restart_event(stage, coordinator.attempts(), None);
                if !with_context {
                    if let Some(snap) = coordinator.last_snapshot().cloned() {
                        // Re-seed prompt + screenshot from snapshot.
                        // Map the static-string Invariant into a
                        // CuError::Protocol so the loop can surface it
                        // through the existing taxonomy.
                        restart::restore_from(&snap, system_prompt, screenshot)
                            .map_err(|e| CuError::Protocol(e.to_string()))?;
                    } else {
                        // No snapshot yet — reset the prompt to the
                        // base; the first fresh step starts clean.
                        *system_prompt = base_system_prompt.to_string();
                    }
                    // Best-effort close of the wedged sandbox; open a
                    // fresh one. The pool slot keeps the original
                    // entry — its TTL reaper will evict it. The fresh
                    // sandbox lives only for this dispatch.
                    let _ = self.e2b.close_sandbox(sandbox).await;
                    *sandbox = self.e2b.open_sandbox(&profile.sandbox_template).await?;
                    *screenshot = self.e2b.screenshot(sandbox).await?;
                }
                let backoff = u64::from(coordinator.backoff_ms());
                if backoff > 0 {
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
                Ok(None)
            }
        }
    }

    fn anthropic_for(&self, profile: &Profile) -> Result<AnthropicClient, CuError> {
        AnthropicClient::new(
            self.anthropic_template.base_url.clone(),
            self.anthropic_template.api_key.clone(),
            profile.model.clone(),
            self.anthropic_template.per_request_timeout_seconds,
        )
    }
}

#[derive(Debug)]
enum LoopExit {
    Success(RunResult),
    Handoff(String),
}

fn stringify_verdict(v: BudgetVerdict) -> String {
    match v {
        BudgetVerdict::Exhausted { reason } => reason,
        BudgetVerdict::Ok => "(unexpected ok verdict)".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_executor() -> ComputerUseExecutor {
        let cfg = CuConfig::for_tests("http://e2b.local".into(), "http://anth.local".into());
        let registry = ProfileRegistry::with_builtin_marketplace_browser();
        ComputerUseExecutor::new(cfg, registry)
    }

    #[tokio::test]
    async fn run_errors_with_unknown_profile() {
        let exec = test_executor();
        let req = RunRequest::demo("act-1", "no_such_profile");
        let err = exec
            .run(req)
            .await
            .err()
            .unwrap_or_else(|| panic!("unknown profile must error"));
        assert!(err.to_string().contains("no_such_profile"));
        assert!(err.to_string().contains("marketplace_browser"));
    }
}
