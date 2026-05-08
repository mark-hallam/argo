//! Tests for `turn::build_system_prompt` + the multi-round
//! tool-dispatch loop's max-rounds behaviour (#724).

use super::*;
use argo_store::SessionStore;

#[tokio::test]
async fn build_system_prompt_returns_none_for_empty_workspace() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let prompt =
        build_system_prompt(&store, SessionId(uuid::Uuid::nil()), None, TenantId::new(), None)
            .await
            .unwrap();
    assert!(prompt.is_none());
}

#[tokio::test]
async fn build_system_prompt_concatenates_active_skills() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let skills = argo_skill::SkillStore::new(Arc::clone(&store));
    let raw = "---\nname: alpha\nversion: \"0.1\"\ndescription: D\n\
               lock: none\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\nbody-of-alpha";
    skills.install(raw, None).await.unwrap();
    let prompt =
        build_system_prompt(&store, SessionId(uuid::Uuid::nil()), None, TenantId::new(), None)
            .await
            .unwrap()
            .unwrap();
    assert!(prompt.contains("## alpha (v0.1)"));
    assert!(prompt.contains("body-of-alpha"));
    assert!(prompt.starts_with("You have access to"));
}

#[tokio::test]
async fn build_system_prompt_honours_per_session_attachments() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/sa").await.unwrap();
    let sid = store
        .create_session(tenant, ws, "m", "p", Some("t"), None)
        .await
        .unwrap();

    let skills = argo_skill::SkillStore::new(Arc::clone(&store));
    let raw_a = "---\nname: alpha\nversion: \"0.1\"\ndescription: D\n\
                 lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nbody-A";
    let raw_b = "---\nname: beta\nversion: \"0.1\"\ndescription: D\n\
                 lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nbody-B";
    let _alpha = skills.install(raw_a, None).await.unwrap();
    let beta = skills.install(raw_b, None).await.unwrap();

    // Default behaviour: both active skills are in scope.
    let prompt_default = build_system_prompt(&store, sid, None, tenant, None)
        .await
        .unwrap()
        .unwrap();
    assert!(prompt_default.contains("body-A"));
    assert!(prompt_default.contains("body-B"));

    // Attach only beta — alpha should now drop out.
    skills.attach_to_session(sid, beta.id).await.unwrap();
    let prompt_curated = build_system_prompt(&store, sid, None, tenant, None)
        .await
        .unwrap()
        .unwrap();
    assert!(prompt_curated.contains("body-B"));
    assert!(!prompt_curated.contains("body-A"));

    // Detach: falls back to all-active.
    let removed = skills.detach_from_session(sid, beta.id).await.unwrap();
    assert_eq!(removed, 1);
    let prompt_after = build_system_prompt(&store, sid, None, tenant, None)
        .await
        .unwrap()
        .unwrap();
    assert!(prompt_after.contains("body-A"));
    assert!(prompt_after.contains("body-B"));
}

#[tokio::test]
async fn build_system_prompt_for_bound_session_filters_to_vertical_skills() {
    // #679 — when a session is vertical-bound and has no
    // explicit attachments, the prompt fallback uses
    // SkillStore::list_skills_for_vertical, which excludes
    // other verticals' skills + workspace-global skills.
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let skills = argo_skill::SkillStore::new(Arc::clone(&store));

    // alpha-skill: bundled by vertical "alpha"
    let alpha_doc = "---\nname: alpha-skill\nversion: \"0.1\"\ndescription: D\n\
                     lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nalpha-body";
    skills
        .install_for_vertical(alpha_doc, None, Some("alpha"))
        .await
        .unwrap();
    // beta-skill: bundled by vertical "beta"
    let beta_doc = "---\nname: beta-skill\nversion: \"0.1\"\ndescription: D\n\
                    lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nbeta-body";
    skills
        .install_for_vertical(beta_doc, None, Some("beta"))
        .await
        .unwrap();
    // legacy-skill: workspace-global (no vertical context)
    let legacy_doc = "---\nname: legacy\nversion: \"0.1\"\ndescription: D\n\
                      lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nlegacy-body";
    skills.install(legacy_doc, None).await.unwrap();

    // Create a session bound to vertical "alpha" (no
    // explicit attachments, so the fallback path runs).
    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/v679").await.unwrap();
    let sid = store
        .create_session(tenant, ws, "m", "p", Some("t"), Some("alpha"))
        .await
        .unwrap();

    let prompt = build_system_prompt(&store, sid, Some("alpha"), tenant, None)
        .await
        .unwrap()
        .unwrap();
    assert!(prompt.contains("alpha-body"), "must contain own bundled");
    assert!(
        !prompt.contains("beta-body"),
        "must NOT contain other vertical's skills"
    );
    assert!(
        !prompt.contains("legacy-body"),
        "must NOT contain workspace-global skills (default-deny)"
    );
}

#[tokio::test]
async fn build_system_prompt_for_unbound_session_keeps_legacy_global_lookup() {
    // Backward compat: sessions WITHOUT vertical_id still see
    // the full workspace skill set (legacy behaviour). Locks
    // that #679 doesn't break the unbound path.
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let skills = argo_skill::SkillStore::new(Arc::clone(&store));
    let raw = "---\nname: any\nversion: \"0.1\"\ndescription: D\n\
               lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nany-body";
    skills
        .install_for_vertical(raw, None, Some("alpha"))
        .await
        .unwrap();

    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/v679u").await.unwrap();
    // session with vertical_id = None
    let sid = store
        .create_session(tenant, ws, "m", "p", Some("t"), None)
        .await
        .unwrap();
    let prompt = build_system_prompt(&store, sid, None, tenant, None)
        .await
        .unwrap()
        .unwrap();
    assert!(
        prompt.contains("any-body"),
        "unbound session must see all workspace skills (legacy)"
    );
}

#[tokio::test]
async fn build_system_prompt_injects_shared_team_memory_block() {
    use argo_identity::WorkerId;
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (tenant, _ws) = store.ensure_workspace("ws", "/tmp/mwm").await.unwrap();

    // Write 2 shared-team rows for this tenant.
    let memory = argo_memory::MemoryStore::new(Arc::clone(&store));
    let ns = argo_memory::shared_team(tenant);
    memory
        .write_shared_team(&ns, WorkerId::new(), "first handoff note".into())
        .await
        .unwrap();
    memory
        .write_shared_team(&ns, WorkerId::new(), "second handoff note".into())
        .await
        .unwrap();

    let prompt = build_system_prompt(&store, SessionId(uuid::Uuid::nil()), None, tenant, None)
        .await
        .unwrap()
        .unwrap();
    assert!(
        prompt.contains("## Memory (workspace-shared, last 2)"),
        "expected memory block header, got:\n{prompt}"
    );
    assert!(prompt.contains("first handoff note"), "got:\n{prompt}");
    assert!(prompt.contains("second handoff note"), "got:\n{prompt}");
}

#[tokio::test]
async fn build_system_prompt_prefers_worker_memory_when_session_has_worker() {
    use argo_identity::{MemoryNs, MemoryScope};
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/mwm-worker").await.unwrap();
    let worker = argo_store::WorkerId(uuid::Uuid::now_v7());
    let memory_worker = argo_identity::WorkerId(worker.0);
    let sid = store
        .create_session_with_worker(argo_store::SessionCreate {
            tenant_id: tenant,
            workspace_id: ws,
            model: "m",
            provider: "p",
            title: Some("t"),
            vertical_id: None,
            worker_id: Some(worker),
        })
        .await
        .unwrap();

    let memory = argo_memory::MemoryStore::new(Arc::clone(&store));
    let worker_ns = MemoryNs::new(tenant, memory_worker, MemoryScope::WorkerSelf);
    memory
        .append_in_namespace(
            &worker_ns,
            ws,
            argo_memory::MemoryFile::Memory,
            "worker-specific note".into(),
            Some(sid),
        )
        .await
        .unwrap();
    memory
        .write_shared_team(&argo_memory::shared_team(tenant), memory_worker, "shared note".into())
        .await
        .unwrap();

    let prompt = build_system_prompt(&store, sid, None, tenant, Some(worker))
        .await
        .unwrap()
        .unwrap();
    assert!(prompt.contains("## Memory (worker-scoped)"), "got:\n{prompt}");
    assert!(prompt.contains("worker-specific note"), "got:\n{prompt}");
    assert!(!prompt.contains("shared note"), "got:\n{prompt}");
}

#[tokio::test]
async fn build_system_prompt_omits_memory_block_when_no_rows() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (tenant, _ws) = store.ensure_workspace("ws", "/tmp/mwm-empty").await.unwrap();
    let prompt = build_system_prompt(&store, SessionId(uuid::Uuid::nil()), None, tenant, None)
        .await
        .unwrap();
    assert!(prompt.is_none());
}

#[tokio::test]
async fn prompt_memory_modes_select_shared_worker_and_both() {
    use argo_identity::{MemoryNs, MemoryScope};
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/mwm-modes").await.unwrap();
    let worker = argo_store::WorkerId(uuid::Uuid::now_v7());
    let memory_worker = argo_identity::WorkerId(worker.0);
    let memory = argo_memory::MemoryStore::new(Arc::clone(&store));
    let worker_ns = MemoryNs::new(tenant, memory_worker, MemoryScope::WorkerSelf);
    memory
        .append_in_namespace(
            &worker_ns,
            ws,
            argo_memory::MemoryFile::Memory,
            "mode worker note".into(),
            None,
        )
        .await
        .unwrap();
    memory
        .write_shared_team(
            &argo_memory::shared_team(tenant),
            memory_worker,
            "mode shared note".into(),
        )
        .await
        .unwrap();

    let shared = turn_memory::compose_prompt_memory_for_mode(
        &store,
        tenant,
        Some(worker),
        turn_memory::PromptMemoryMode::Shared,
    )
    .await;
    assert!(shared.contains("mode shared note"), "got:\n{shared}");
    assert!(!shared.contains("mode worker note"), "got:\n{shared}");

    let worker_without_id = turn_memory::compose_prompt_memory_for_mode(
        &store,
        tenant,
        None,
        turn_memory::PromptMemoryMode::Worker,
    )
    .await;
    assert!(worker_without_id.is_empty(), "got:\n{worker_without_id}");

    let both = turn_memory::compose_prompt_memory_for_mode(
        &store,
        tenant,
        Some(worker),
        turn_memory::PromptMemoryMode::Both,
    )
    .await;
    let worker_pos = both.find("mode worker note").expect("worker note");
    let shared_pos = both.find("mode shared note").expect("shared note");
    assert!(worker_pos < shared_pos, "got:\n{both}");
}

#[tokio::test]
async fn build_system_prompt_appends_memory_after_skills() {
    use argo_identity::WorkerId;
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (tenant, _ws) = store.ensure_workspace("ws", "/tmp/mwm-both").await.unwrap();

    let skills = argo_skill::SkillStore::new(Arc::clone(&store));
    let raw = "---\nname: alpha\nversion: \"0.1\"\ndescription: D\n\
               lock: none\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\nbody-of-alpha";
    skills.install(raw, None).await.unwrap();
    let memory = argo_memory::MemoryStore::new(Arc::clone(&store));
    memory
        .write_shared_team(
            &argo_memory::shared_team(tenant),
            WorkerId::new(),
            "shared note".into(),
        )
        .await
        .unwrap();

    let prompt = build_system_prompt(&store, SessionId(uuid::Uuid::nil()), None, tenant, None)
        .await
        .unwrap()
        .unwrap();
    let skills_pos = prompt.find("body-of-alpha").expect("skill body present");
    let memory_pos = prompt.find("## Memory").expect("memory header present");
    assert!(
        skills_pos < memory_pos,
        "expected skills before memory; got:\n{prompt}"
    );
    assert!(prompt.contains("shared note"));
}

/// Mock Transport that always returns a `file_read` tool call.
/// Used by the max-rounds graceful-exit test to drive the
/// dispatch loop forever — without ever satisfying the model
/// (the model "wants" another tool call every round).
#[derive(Debug)]
struct LoopForeverTransport {
    caps: argo_types::CapabilitySet,
    target_path: std::path::PathBuf,
}

impl LoopForeverTransport {
    fn new(target_path: std::path::PathBuf) -> Self {
        Self {
            caps: argo_types::CapabilitySet(vec![]),
            target_path,
        }
    }
}

#[async_trait::async_trait]
impl argo_provider::Transport for LoopForeverTransport {
    fn id(&self) -> &str {
        "loop-forever"
    }
    fn capabilities(&self) -> &argo_types::CapabilitySet {
        &self.caps
    }
    async fn complete(
        &self,
        _req: &argo_provider::ProviderRequest,
    ) -> argo_types::ArgoResult<argo_provider::ProviderResponse> {
        Ok(argo_provider::ProviderResponse {
            text: Some("looking up another file...".into()),
            reasoning: None,
            tool_calls: vec![argo_types::ToolCallRequest {
                id: argo_types::ToolCallId::new(),
                provider_call_id: None,
                name: "file_read".into(),
                arguments: serde_json::json!({"path": self.target_path.to_string_lossy()}),
            }],
            stop_reason: argo_provider::StopReason::ToolUse,
            tokens_in: None,
            tokens_out: None,
            provider_meta: serde_json::Value::Null,
        })
    }
    async fn complete_stream(
        &self,
        _req: &argo_provider::ProviderRequest,
    ) -> argo_types::ArgoResult<
        futures::stream::BoxStream<
            'static,
            argo_types::ArgoResult<argo_provider::ProviderResponseDelta>,
        >,
    > {
        unimplemented!("streaming not exercised in max-rounds test")
    }
}

/// #724 — when the inner tool-dispatch loop hits `max_rounds`,
/// it must NOT return Err. It returns Ok with
/// `stopped_on_tool_budget = true` and a fully consistent
/// session state (every dispatched tool has its matching
/// `tool_result` persisted) so the outer worker step loop can
/// continue cleanly. Pre-#724 this would panic on a "tool-
/// dispatch loop hit max_rounds=N; aborting" provider error,
/// which the worker misclassified as a transport failure and
/// stopped the job for `[STOP: NEED_INFO]`.
#[tokio::test]
async fn max_rounds_returns_ok_with_stopped_on_tool_budget_flag() {
    use crate::helpers::SpendCtx;
    use crate::turn_vertical::SessionGrantState;
    use argo_audit::AuditSink;

    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    // file_read needs a real on-disk file to dispatch successfully.
    let target = cwd.path().join("dummy.txt");
    std::fs::write(&target, "hello tool budget").unwrap();

    let (tenant, ws) = store
        .ensure_workspace("ws-724", &cwd.path().to_string_lossy())
        .await
        .unwrap();
    let sid = store
        .create_session(tenant, ws, "mock-model", "mock-provider", Some("t"), None)
        .await
        .unwrap();

    let transport = LoopForeverTransport::new(target);
    let audit = AuditSink::new(Arc::clone(&store));
    let spend = SpendCtx {
        tenant_id: tenant,
        provider: "mock-provider",
        model: "mock-model",
        pricing: None, // skip spend recording
    };
    let deny = std::collections::HashSet::new();
    let approval = std::collections::HashSet::new();
    let grants = SessionGrantState::default();

    let r = run_turn_with_tools(
        &store,
        &transport,
        cwd.path(),
        sid,
        "mock-model",
        &audit,
        &spend,
        3, // max_rounds — small so the test runs fast
        false,
        None,
        None,
        None,
        &deny,
        &approval,
        grants,
        None,
    )
    .await
    .expect("max_rounds must NOT produce an Err post-#724");

    assert!(
        r.stopped_on_tool_budget,
        "stopped_on_tool_budget must be true when the loop exits on the cap"
    );
    // Session state: 3 assistant messages (one per round) + 3 tool
    // messages (one per dispatched call). Pre-#724 the loop would
    // have appended 3 of each but then returned Err, leaving the
    // session intact but the caller seeing a transport-like error.
    let messages = store.list_messages(sid).await.unwrap();
    let role_counts = messages
        .iter()
        .fold((0u32, 0u32), |acc, m| match m.role.as_str() {
            "assistant" => (acc.0 + 1, acc.1),
            "tool" => (acc.0, acc.1 + 1),
            _ => acc,
        });
    assert_eq!(
        role_counts.0, 3,
        "expected 3 assistant messages, got {}",
        role_counts.0
    );
    assert_eq!(
        role_counts.1, 3,
        "expected 3 tool messages, got {}",
        role_counts.1
    );
}
