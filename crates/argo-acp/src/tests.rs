use super::*;
use argo_capability::CapabilityEngine;
use argo_types::{ArgoResult, CapabilityRole, CapabilitySet, Principal, SessionId};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

// ── auth verification ─────────────────────────────────────────

#[test]
fn verify_copilot_oauth_accepts_gho_prefix() {
    let p = verify(AuthScheme::CopilotOAuth, &AuthToken("gho_abcdef".into())).unwrap();
    if let Principal::Acp { harness, .. } = p {
        assert_eq!(harness, "copilot");
    } else {
        panic!("expected Acp principal");
    }
}

#[test]
fn verify_copilot_oauth_rejects_wrong_prefix() {
    let err = verify(AuthScheme::CopilotOAuth, &AuthToken("xyz_token".into())).unwrap_err();
    assert!(matches!(err, AuthError::Malformed { .. }));
}

#[test]
fn verify_cursor_accepts_prefix() {
    let p = verify(
        AuthScheme::CursorToken,
        &AuthToken("cursor_proj_abc".into()),
    )
    .unwrap();
    if let Principal::Acp { harness, .. } = p {
        assert_eq!(harness, "cursor");
    } else {
        panic!("expected Acp principal");
    }
}

#[test]
fn verify_cursor_rejects_wrong_prefix() {
    let err = verify(AuthScheme::CursorToken, &AuthToken("cur_xyz".into())).unwrap_err();
    assert!(matches!(err, AuthError::Malformed { .. }));
}

#[test]
fn verify_generic_bearer_accepts_any_nonempty_token() {
    let p = verify(
        AuthScheme::GenericBearer,
        &AuthToken("freeform-token".into()),
    )
    .unwrap();
    if let Principal::Acp { harness, .. } = p {
        assert_eq!(harness, "generic");
    } else {
        panic!("expected Acp principal");
    }
}

#[test]
fn verify_rejects_empty_token() {
    let err = verify(AuthScheme::GenericBearer, &AuthToken("".into())).unwrap_err();
    assert!(matches!(err, AuthError::Empty));
}

// ── multi-harness advertisement ───────────────────────────────

#[test]
fn each_harness_has_a_stable_id() {
    let ids = [
        Harness::CopilotCli.id(),
        Harness::CursorCli.id(),
        Harness::VsCode.id(),
        Harness::Zed.id(),
        Harness::JetBrains.id(),
        Harness::Generic.id(),
    ];
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 6);
}

#[test]
fn default_capabilities_advertise_all_four_methods() {
    // §28.Y point 4: send message, receive stream, list sessions,
    // fork session — each harness ships with all four enabled
    // by default.
    for h in [
        Harness::CopilotCli,
        Harness::CursorCli,
        Harness::VsCode,
        Harness::Zed,
        Harness::JetBrains,
        Harness::Generic,
    ] {
        let c = HarnessCapabilities::default_for(h);
        assert!(c.send_message && c.stream && c.list_sessions && c.fork_session);
    }
}

// ── permission mapping → CapabilityEngine ─────────────────────

#[test]
fn permission_maps_through_capability_engine() {
    // §28.Y point 5: ACP permission requests resolve through the
    // SAME capability engine that backs CLI / TUI / gateway.
    let engine = CapabilityEngine::new();
    engine
        .add_role(CapabilityRole {
            name: "ide".into(),
            inherits_from: None,
            capabilities: CapabilitySet(vec![argo_types::Capability::FsRead {
                path_glob: "src/**".into(),
            }]),
        })
        .unwrap();
    let principal = verify(AuthScheme::GenericBearer, &AuthToken("ide-1".into())).unwrap();
    engine.bind(principal.clone(), "ide".into()).unwrap();

    // Allowed read.
    let allowed = map_permission(&AcpPermission::FileRead {
        path: "src/main.rs".into(),
    });
    assert!(engine.check(&principal, &allowed).is_ok());

    // Denied write (role only granted FsRead).
    let denied = map_permission(&AcpPermission::FileWrite {
        path: "src/main.rs".into(),
    });
    assert!(engine.check(&principal, &denied).is_err());
}

#[test]
fn terminal_and_subagent_map_to_spawn() {
    let t = map_permission(&AcpPermission::Terminal);
    let s = map_permission(&AcpPermission::Subagent);
    matches!(
        t,
        argo_types::Capability::Spawn {
            spawn_kind: argo_types::SpawnKind::Subprocess
        }
    );
    matches!(
        s,
        argo_types::Capability::Spawn {
            spawn_kind: argo_types::SpawnKind::Subagent
        }
    );
}

// ── server dispatch ──────────────────────────────────────────

#[derive(Default)]
struct StubService {
    sent: Arc<AtomicUsize>,
}

#[async_trait]
impl AcpService for StubService {
    async fn send_message(&self, _req: AcpSendMessageRequest) -> ArgoResult<()> {
        self.sent.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn list_sessions(&self) -> ArgoResult<ListSessionsResponse> {
        Ok(ListSessionsResponse {
            sessions: vec![SessionId::new()],
        })
    }
    async fn fork_session(&self, _req: ForkRequest) -> ArgoResult<SessionId> {
        Ok(SessionId::new())
    }
}

fn req(method: &str, params: serde_json::Value) -> AcpRequest {
    AcpRequest::new(serde_json::json!(1), method, Some(params))
}

fn handshake(params: serde_json::Value) -> AcpRequest {
    AcpRequest::new(serde_json::json!("init"), "initialize", Some(params))
}

#[tokio::test]
async fn unauthenticated_request_returns_1001() {
    let server = AcpServer::new(
        Harness::VsCode,
        AuthScheme::CopilotOAuth,
        StubService::default(),
    );
    let r = req("session/list", serde_json::json!({}));
    let resp = server.dispatch(r, &AuthToken("not-a-token".into())).await;
    let err = resp.error.unwrap();
    assert_eq!(err.code, 1001);
}

#[tokio::test]
async fn handshake_detects_harness_from_user_agent() {
    let server = AcpServer::new(
        Harness::Generic,
        AuthScheme::CopilotOAuth,
        StubService::default(),
    );
    let resp = server
        .dispatch(
            handshake(serde_json::json!({"user_agent": "Cursor/0.45 ACP"})),
            &AuthToken("any".into()),
        )
        .await;
    assert!(resp.error.is_none());
    let payload = resp.result.unwrap();
    assert_eq!(payload["harness"], Harness::CursorCli.id());
    assert_eq!(payload["capabilities"]["send_message"], true);
}

#[tokio::test]
async fn explicit_harness_declaration_overrides_user_agent() {
    let server = AcpServer::new(
        Harness::Generic,
        AuthScheme::GenericBearer,
        StubService::default(),
    );
    let req = AcpRequest::new(
        serde_json::json!("decl"),
        "harness/declare",
        Some(serde_json::json!({
            "harness": "zed",
            "user_agent": "Cursor/0.45"
        })),
    );
    let resp = server.dispatch(req, &AuthToken("not-a-token".into())).await;
    assert!(resp.error.is_none());
    assert_eq!(resp.result.unwrap()["harness"], Harness::Zed.id());
}

#[tokio::test]
async fn handshake_filters_capabilities_against_server_ceiling() {
    let mut server = AcpServer::new(
        Harness::Generic,
        AuthScheme::GenericBearer,
        StubService::default(),
    );
    server.capabilities.fork_session = false;
    server.capabilities.stream = false;
    let resp = server
        .dispatch(
            handshake(serde_json::json!({"harness": "jetbrains"})),
            &AuthToken("any".into()),
        )
        .await;
    let caps = &resp.result.unwrap()["capabilities"];
    assert_eq!(caps["send_message"], true);
    assert_eq!(caps["list_sessions"], true);
    assert_eq!(caps["stream"], false);
    assert_eq!(caps["fork_session"], false);
}

#[tokio::test]
async fn unknown_explicit_harness_is_invalid_params() {
    let server = AcpServer::new(
        Harness::Generic,
        AuthScheme::GenericBearer,
        StubService::default(),
    );
    let resp = server
        .dispatch(
            handshake(serde_json::json!({"harness": "mystery-ide"})),
            &AuthToken("any".into()),
        )
        .await;
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("unknown ACP harness"));
}

#[tokio::test]
async fn authenticated_send_message_dispatches_to_service() {
    let svc = StubService::default();
    let counter = Arc::clone(&svc.sent);
    let server = AcpServer::new(Harness::Generic, AuthScheme::GenericBearer, svc);
    let payload = AcpSendMessageRequest {
        session: SessionId::new(),
        text: "hi".into(),
    };
    let r = req(
        "session/send_message",
        serde_json::to_value(&payload).unwrap(),
    );
    let resp = server.dispatch(r, &AuthToken("any-token".into())).await;
    assert!(resp.error.is_none());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let server = AcpServer::new(
        Harness::Zed,
        AuthScheme::GenericBearer,
        StubService::default(),
    );
    let r = req("session/no_such_method", serde_json::json!({}));
    let resp = server.dispatch(r, &AuthToken("any".into())).await;
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
}

#[tokio::test]
async fn capability_disabled_returns_permission_denied() {
    let mut server = AcpServer::new(
        Harness::Generic,
        AuthScheme::GenericBearer,
        StubService::default(),
    );
    server.capabilities = HarnessCapabilities::empty();
    let payload = AcpSendMessageRequest {
        session: SessionId::new(),
        text: "hi".into(),
    };
    let r = req(
        "session/send_message",
        serde_json::to_value(&payload).unwrap(),
    );
    let resp = server.dispatch(r, &AuthToken("any".into())).await;
    let err = resp.error.unwrap();
    assert_eq!(err.code, 1003);
}

#[tokio::test]
async fn list_sessions_returns_payload() {
    let server = AcpServer::new(
        Harness::JetBrains,
        AuthScheme::GenericBearer,
        StubService::default(),
    );
    let r = req("session/list", serde_json::json!({}));
    let resp = server.dispatch(r, &AuthToken("any".into())).await;
    assert!(resp.error.is_none());
    let payload: ListSessionsResponse = serde_json::from_value(resp.result.unwrap()).unwrap();
    assert_eq!(payload.sessions.len(), 1);
}

#[tokio::test]
async fn mock_client_can_handshake_then_list_sessions() {
    let server = AcpServer::new(
        Harness::Generic,
        AuthScheme::GenericBearer,
        StubService::default(),
    );
    let init = server
        .dispatch(
            handshake(serde_json::json!({"userAgent": "Zed/0.17"})),
            &AuthToken("any".into()),
        )
        .await;
    assert_eq!(init.result.unwrap()["harness"], Harness::Zed.id());

    let list = server
        .dispatch(
            req("session/list", serde_json::json!({})),
            &AuthToken("any".into()),
        )
        .await;
    let payload: ListSessionsResponse = serde_json::from_value(list.result.unwrap()).unwrap();
    assert_eq!(payload.sessions.len(), 1);
}

// ── wire round-trip ──────────────────────────────────────────

#[test]
fn wire_envelopes_round_trip() {
    let req = AcpRequest::new(
        serde_json::json!(7),
        "session/list",
        Some(serde_json::json!({})),
    );
    let s = serde_json::to_string(&req).unwrap();
    let parsed: AcpRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed, req);
    assert_eq!(parsed.jsonrpc, JSONRPC_VERSION);
}
