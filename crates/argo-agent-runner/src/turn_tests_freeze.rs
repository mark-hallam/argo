//! mission-pm-kill-switch-2-runner — pre-turn + pre-dispatch
//! worker-freeze gate tests. Split out of `turn_tests.rs` to keep
//! both files under the 500-LOC ceiling (HARD RULE 6), matching the
//! `turn_tests_max_rounds.rs` sibling pattern.
#![allow(deprecated)]

use super::*;
use argo_store::SessionStore;

#[tokio::test]
async fn frozen_worker_pre_turn_check_blocks_run_turn() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    let (tenant, ws) = store
        .ensure_workspace("ws-freeze-preturn", &cwd.path().to_string_lossy())
        .await
        .unwrap();
    let worker = argo_store::WorkerId::new();
    let sid = store
        .create_session_with_worker(argo_store::SessionCreate {
            tenant_id: tenant,
            workspace_id: ws,
            model: "mock-model",
            provider: "mock-provider",
            title: Some("freeze pre-turn"),
            vertical_id: None,
            worker_id: Some(worker),
        })
        .await
        .unwrap();

    argo_store::worker_freeze::freeze(
        store.pool(),
        &tenant.to_string(),
        &worker.to_string(),
        "principal:owner",
        "incident response",
    )
    .await
    .unwrap();

    let err = super::run_turn(&store, cwd.path(), sid, Some("hi"), false, MAX_TOOL_ROUNDS)
        .await
        .err()
        .expect("frozen worker must error before any provider work");
    let msg = err.to_string();
    assert!(
        msg.contains(&format!("Worker {worker} is frozen by principal:owner")),
        "got: {msg}"
    );
    assert!(msg.contains("incident response"), "got: {msg}");
    assert!(
        msg.contains("New turns and tool calls are blocked"),
        "got: {msg}"
    );
    assert!(
        msg.contains("In-flight tool calls continue to completion"),
        "got: {msg}"
    );

    let messages = store.list_messages(sid).await.unwrap();
    assert!(
        messages.is_empty(),
        "pre-turn gate must abort before append_message; got {} rows",
        messages.len()
    );
}

#[derive(Debug)]
struct OneToolCallTransport {
    caps: argo_types::CapabilitySet,
}

#[async_trait::async_trait]
impl argo_provider::Transport for OneToolCallTransport {
    fn id(&self) -> &str {
        "freeze-one-tool-call"
    }
    fn capabilities(&self) -> &argo_types::CapabilitySet {
        &self.caps
    }
    async fn complete(
        &self,
        _req: &argo_provider::ProviderRequest,
    ) -> argo_types::ArgoResult<argo_provider::ProviderResponse> {
        Ok(argo_provider::ProviderResponse {
            text: Some("dispatching one tool".into()),
            reasoning: None,
            tool_calls: vec![argo_types::ToolCallRequest {
                id: argo_types::ToolCallId::new(),
                provider_call_id: None,
                name: "file_read".into(),
                arguments: serde_json::json!({"path": "/dev/null"}),
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
        unimplemented!("streaming not exercised in pre-dispatch freeze test")
    }
}

#[tokio::test]
async fn pre_dispatch_check_refuses_tool_calls_for_frozen_worker() {
    use crate::helpers::SpendCtx;
    use crate::turn_vertical::SessionGrantState;
    use argo_audit::AuditSink;

    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let cwd = tempfile::tempdir().expect("cwd tempdir");
    let (tenant, ws) = store
        .ensure_workspace("ws-freeze-dispatch", &cwd.path().to_string_lossy())
        .await
        .unwrap();
    let worker = argo_store::WorkerId::new();
    let sid = store
        .create_session_with_worker(argo_store::SessionCreate {
            tenant_id: tenant,
            workspace_id: ws,
            model: "mock-model",
            provider: "mock-provider",
            title: Some("freeze pre-dispatch"),
            vertical_id: None,
            worker_id: Some(worker),
        })
        .await
        .unwrap();

    argo_store::worker_freeze::freeze(
        store.pool(),
        &tenant.to_string(),
        &worker.to_string(),
        "principal:owner",
        "halt the bots",
    )
    .await
    .unwrap();

    let transport = OneToolCallTransport {
        caps: argo_types::CapabilitySet(vec![]),
    };
    let audit = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
    let spend = SpendCtx {
        tenant_id: tenant,
        provider: "mock-provider",
        model: "mock-model",
        pricing: None,
    };
    let deny = std::collections::HashSet::new();
    let approval = std::collections::HashSet::new();
    let grants = SessionGrantState::default();
    let mut pending_ab: Vec<super::turn_skills_runtime::PendingAbOutcome> = Vec::new();

    let err = super::turn_helpers::run_turn_with_tools(
        &store,
        &transport,
        cwd.path(),
        sid,
        "mock-model",
        &audit,
        &spend,
        3,
        false,
        None,
        None,
        None,
        &deny,
        &approval,
        grants,
        Some(worker),
        "test-invocation",
        &mut pending_ab,
    )
    .await
    .err()
    .expect("pre-dispatch gate must refuse the tool call for a frozen worker");
    let msg = err.to_string();
    assert!(msg.contains("is frozen by principal:owner"), "got: {msg}");
    assert!(msg.contains("halt the bots"), "got: {msg}");

    audit.flush_pending().await.unwrap();
    let rows = store.list_audit_for_session(sid).await.unwrap();
    let kinds: Vec<&str> = rows.iter().map(|r| r.kind.as_str()).collect();
    assert!(
        kinds.contains(&"worker_freeze_blocked"),
        "expected WorkerFreezeBlocked audit row, got kinds: {kinds:?}"
    );
    assert!(
        !kinds.contains(&"tool_call"),
        "no ToolCall row should land — dispatch was refused; got: {kinds:?}"
    );
    let blocked_count = kinds
        .iter()
        .filter(|k| **k == "worker_freeze_blocked")
        .count();
    assert_eq!(
        blocked_count, 1,
        "exactly one block emission expected per tool call refused"
    );
}

#[tokio::test]
async fn check_or_cache_queries_once_then_returns_cached_result() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let worker = argo_store::WorkerId::new();
    argo_store::worker_freeze::freeze(
        store.pool(),
        &tenant.to_string(),
        &worker.to_string(),
        "principal:owner",
        "first reason",
    )
    .await
    .unwrap();

    let mut cache: Option<Option<argo_store::FreezeRecord>> = None;
    let first =
        super::turn_helpers_freeze::check_or_cache(&store, tenant, worker, &mut cache)
            .await
            .unwrap()
            .expect("first lookup must find the freeze row");
    assert_eq!(first.reason, "first reason");
    assert!(cache.is_some(), "cache must be populated after first call");

    // Mutate the DB after the cache is populated. The cached lookup
    // must keep returning the first-call value — proving the DB is
    // not re-queried on subsequent dispatch hops.
    argo_store::worker_freeze::unfreeze(
        store.pool(),
        &tenant.to_string(),
        &worker.to_string(),
        "principal:owner",
    )
    .await
    .unwrap();

    let second =
        super::turn_helpers_freeze::check_or_cache(&store, tenant, worker, &mut cache)
            .await
            .unwrap()
            .expect("cache must keep returning the original row");
    assert_eq!(second, first, "cached value must be identical");

    // Drop the cache and re-query — the fresh lookup now reflects
    // the unfreeze, locking the boundary at "per-turn cache" rather
    // than "process-wide cache".
    let mut fresh: Option<Option<argo_store::FreezeRecord>> = None;
    let third =
        super::turn_helpers_freeze::check_or_cache(&store, tenant, worker, &mut fresh)
            .await
            .unwrap();
    assert!(third.is_none(), "fresh lookup must see the unfreeze");
}
