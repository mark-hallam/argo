//! Tests for the multi-round tool-dispatch loop's max-rounds
//! graceful-exit behaviour (#724). Split out of `turn_tests.rs`
//! to keep both files under the 500-LOC ceiling.

use super::*;
use argo_store::SessionStore;

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

    let r = super::turn_helpers::run_turn_with_tools(
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
