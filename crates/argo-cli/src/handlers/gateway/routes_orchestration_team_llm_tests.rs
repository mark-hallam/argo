//! Unit tests for the model-backed delegation executor adapter.
//!
//! These cover the pure, provider-free surface: the flag + provider-id
//! precedence, the truncation helper, the prompt builders, and — most
//! importantly for "no theater" — the fallback path. When the flag is
//! ON but no provider manifest resolves, every step's `detail` falls
//! back to the EXACT deterministic acknowledgement
//! [`PersistingExecutor`] would have produced, and the persisted row
//! tree is byte-for-byte identical to a deterministic run. A real
//! provider round-trip is exercised end-to-end by the gateway smoke
//! test against a throwaway gateway (documented in the brief), not here
//! — dialing a real model from a unit test would be flaky.
use super::*;
use argo_orchestrator::{DelegationStep, OrchestratorStore, StepId, Supervisor, TeamPlan};
use argo_store::SessionStore;
use std::sync::{Mutex, OnceLock};

/// Env vars are process-global; serialize the precedence/flag tests so a
/// concurrent run does not observe a half-applied set/clear pair.
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    keys: Vec<(&'static str, Option<String>)>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn apply(set: &[(&'static str, &str)], unset: &[&'static str]) -> Self {
        let guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut keys = Vec::with_capacity(set.len() + unset.len());
        for (k, v) in set {
            keys.push((*k, std::env::var(k).ok()));
            std::env::set_var(k, v);
        }
        for k in unset {
            keys.push((*k, std::env::var(k).ok()));
            std::env::remove_var(k);
        }
        Self { keys, _guard: guard }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, old) in &self.keys {
            match old {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

#[test]
fn llm_executor_off_by_default() {
    let _env = EnvGuard::apply(&[], &[ORCH_LLM_EXECUTOR_ENV]);
    assert!(
        !llm_executor_enabled(),
        "executor must default OFF so the team-runs test suite stays deterministic"
    );
}

#[test]
fn llm_executor_truthy_values_enable() {
    for v in ["1", "true", "ON", "Yes"] {
        let _env = EnvGuard::apply(&[(ORCH_LLM_EXECUTOR_ENV, v)], &[]);
        assert!(llm_executor_enabled(), "`{v}` should enable the executor");
    }
    for v in ["0", "false", "off", "", "  "] {
        let _env = EnvGuard::apply(&[(ORCH_LLM_EXECUTOR_ENV, v)], &[]);
        assert!(!llm_executor_enabled(), "`{v}` should NOT enable");
    }
}

#[test]
fn provider_id_defaults_to_claude_code_cli() {
    let _env = EnvGuard::apply(&[], &[ORCH_LLM_PROVIDER_ENV, "ARGO_PITCH_LLM_PROVIDER"]);
    assert_eq!(resolve_provider_id(), "claude-code-cli");
}

#[test]
fn provider_id_prefers_orch_specific_env() {
    let _env = EnvGuard::apply(
        &[
            (ORCH_LLM_PROVIDER_ENV, "anthropic"),
            ("ARGO_PITCH_LLM_PROVIDER", "claude-code-cli"),
        ],
        &[],
    );
    assert_eq!(resolve_provider_id(), "anthropic");
}

#[test]
fn provider_id_falls_back_to_pitch_env() {
    let _env = EnvGuard::apply(
        &[("ARGO_PITCH_LLM_PROVIDER", "claude-code-cli")],
        &[ORCH_LLM_PROVIDER_ENV],
    );
    assert_eq!(resolve_provider_id(), "claude-code-cli");
}

#[test]
fn provider_id_ignores_blank_env() {
    let _env = EnvGuard::apply(
        &[
            (ORCH_LLM_PROVIDER_ENV, "   "),
            ("ARGO_PITCH_LLM_PROVIDER", ""),
        ],
        &[],
    );
    assert_eq!(resolve_provider_id(), "claude-code-cli");
}

#[test]
fn truncate_keeps_short_output_verbatim() {
    assert_eq!(truncate("short answer", 4_000), "short answer");
}

#[test]
fn truncate_caps_long_output_with_marker() {
    let long = "x".repeat(5_000);
    let out = truncate(&long, MAX_STEP_OUTPUT_CHARS);
    assert_eq!(out.chars().count(), MAX_STEP_OUTPUT_CHARS);
    assert!(out.ends_with('…'), "truncated output must carry the ellipsis marker");
}

#[test]
fn truncate_is_char_boundary_safe() {
    // Multi-byte chars must not be split mid-codepoint.
    let s = "é".repeat(10);
    let out = truncate(&s, 5);
    assert_eq!(out.chars().count(), 5);
    assert!(out.ends_with('…'));
}

#[test]
fn delegation_prompt_carries_task_and_asks_for_the_work_product() {
    let ctx = StepContext {
        tenant_id: TenantId::new(),
        lead: argo_identity::WorkerId::new(),
        to: argo_identity::WorkerId::new(),
        step_id: StepId::new("gather"),
        task: "summarise Q3 pricing for the three top competitors".to_string(),
        current_handoff_id: None,
        team_run_id: "t1".to_string(),
    };
    let prompt = delegation_prompt(&ctx);
    assert!(prompt.contains("summarise Q3 pricing"), "prompt must carry the task");
    assert!(
        prompt.contains("work product"),
        "prompt must ask for the actual result, not a meta-description"
    );
    assert!(prompt.contains(&ctx.to.to_string()), "prompt names the delegated worker");
}

#[test]
fn synthesis_prompt_includes_only_real_outputs() {
    let plan = TeamPlan::new(
        argo_identity::WorkerId::new(),
        "research sprint",
        vec![DelegationStep::new(
            StepId::new("a"),
            argo_identity::WorkerId::new(),
            "x",
        )],
    );
    let outputs = vec![
        StepOutput {
            step_id: StepId::new("a"),
            text: "real pricing table".to_string(),
            real: true,
        },
        StepOutput {
            step_id: StepId::new("b"),
            text: "delegated step 'b' to worker …".to_string(),
            real: false,
        },
    ];
    let prompt = synthesis_prompt(&plan, &outputs);
    assert!(prompt.contains("research sprint"), "synthesis prompt carries the plan title");
    assert!(prompt.contains("real pricing table"), "real step output is fed to the synthesis");
    assert!(
        !prompt.contains("delegated step 'b'"),
        "the deterministic-fallback output must NOT pollute the synthesis"
    );
}

// ---- End-to-end fallback proof (no provider, in-memory store) -------

/// Open an in-memory store and return both the raw [`SessionStore`]
/// (whose `pool()` is `pub`, so the test can count rows) and the
/// [`OrchestratorStore`] the executor persists through.
async fn store() -> (Arc<SessionStore>, Arc<OrchestratorStore>) {
    let inner = Arc::new(SessionStore::open(":memory:").await.unwrap());
    inner.migrate().await.unwrap();
    let orch = OrchestratorStore::new(Arc::clone(&inner));
    orch.ensure_schema().await.unwrap();
    (inner, Arc::new(orch))
}

async fn orch_run_count(inner: &SessionStore, tenant: TenantId) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM session WHERE tenant_id = ? AND provider = 'orchestrator'",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_one(inner.pool())
    .await
    .unwrap()
}

fn plan() -> TeamPlan {
    TeamPlan::new(
        argo_identity::WorkerId::new(),
        "research-sprint",
        vec![
            DelegationStep::new(StepId::new("a"), argo_identity::WorkerId::new(), "gather"),
            DelegationStep::new(StepId::new("b"), argo_identity::WorkerId::new(), "price"),
        ],
    )
}

/// Flag ON but the provider manifest cannot resolve (empty manifests
/// dir + bogus provider id) → every step falls back to the deterministic
/// detail, the run still completes, and the persisted row tree is the
/// SAME shape a deterministic run produces. This is the "no theater"
/// guard: a fallback reads identically to a real deterministic run, and
/// the LLM executor never hard-breaks the run on a provider miss.
#[tokio::test]
async fn runtime_executor_falls_back_to_deterministic_detail_when_no_provider() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let _env = EnvGuard::apply(
        &[
            (ORCH_LLM_EXECUTOR_ENV, "1"),
            ("ARGO_MANIFESTS_DIR", tmp.path().to_str().unwrap()),
            (ORCH_LLM_PROVIDER_ENV, "no-such-provider"),
        ],
        &["ARGO_PITCH_LLM_PROVIDER"],
    );

    let (inner, orch) = store().await;
    let tenant = TenantId::new();
    let p = plan();
    let persist = Arc::new(PersistingExecutor::new(Arc::clone(&orch)));
    let exec = Arc::new(RuntimeStepExecutor::new(persist, resolve_provider_id()));
    let result = Supervisor::new(exec)
        .run_team(&p, tenant, Some("lead-handoff-1".into()))
        .await
        .expect("run completes even with no provider");

    assert!(result.all_completed(), "all steps complete via fallback");
    assert_eq!(result.completed_count(), 2);
    // Each step's detail is the EXACT deterministic acknowledgement (no
    // theater — a fallback is indistinguishable from a deterministic run).
    for outcome in &result.outcomes {
        let ctx = StepContext {
            tenant_id: tenant,
            lead: p.lead,
            to: p
                .steps
                .iter()
                .find(|s| s.id == outcome.step_id)
                .unwrap()
                .to,
            step_id: outcome.step_id.clone(),
            task: String::new(),
            current_handoff_id: Some("lead-handoff-1".into()),
            team_run_id: String::new(),
        };
        // The deterministic detail does not depend on task/team_run_id.
        let want = PersistingExecutor::deterministic_detail(&ctx);
        assert_eq!(
            outcome.detail, want,
            "step {} must carry the deterministic fallback detail",
            outcome.step_id
        );
    }
    // 2 children + 1 synthesis = 3 orchestrator rows, all tenant-scoped.
    assert_eq!(orch_run_count(&inner, tenant).await, 3);

    // The synthesis row carries NO model headline (no real output to
    // synthesise) — it keeps the canonical title, so the SPA still parses
    // the run-id prefix + the `lead synthesis` body marker.
    let synth_sid =
        argo_orchestrator::SessionId::from_string(result.synthesis_run_id.clone());
    let title = orch
        .find_session(tenant, &synth_sid)
        .await
        .unwrap()
        .expect("synthesis row exists")
        .title;
    assert!(title.contains("lead synthesis —"), "synth title: {title}");
    assert!(
        !title.contains(" · "),
        "no real output → no appended headline: {title}"
    );
}
