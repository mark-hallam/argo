//! Public-surface contract pins for `argo-acp`.
//!
//! Authored by `eval-argo-acp-crate-surface` (Agent Client Protocol
//! substrate audit, 2026-05-24). `argo-acp` is intentionally
//! standalone operator-tooling substrate (see the `zero-caller-allowed`
//! marker in `crates/argo-acp/Cargo.toml`): nothing in the workspace
//! depends on it, but external ACP harnesses (VS Code, Cursor CLI,
//! Zed, JetBrains, Copilot CLI, generic) consume the library directly.
//! Pin the exported handshake / auth / permission / wire surfaces so
//! that drift is loud the next time the operator wires `argo acp
//! serve` into the CLI.

#![allow(dead_code)]

use argo_acp::{
    map_permission, verify, AcpError, AcpNotification, AcpPermission, AcpRequest, AcpResponse,
    AcpSendMessageRequest, AcpServer, AcpService, AuthError, AuthScheme, AuthToken, ForkRequest,
    Harness, HarnessCapabilities, ListSessionsResponse, StreamChunk, JSONRPC_VERSION,
};
use argo_capability::CapabilityEngine;
use argo_types::{
    ArgoResult, Capability, CapabilityRole, CapabilitySet, Principal, SessionId, SpawnKind,
};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

// ---------------------------------------------------------------------------
// 1. `lib.rs` re-export inventory: compile-fence + count sentinel.
//
// Total inventory pinned: 19 names across 5 modules.
// ---------------------------------------------------------------------------

#[test]
fn pub_use_inventory_pins_count_at_19() {
    let inventory = [
        // auth (4)
        "verify",
        "AuthError",
        "AuthScheme",
        "AuthToken",
        // harness (2)
        "Harness",
        "HarnessCapabilities",
        // permission (2)
        "map_permission",
        "AcpPermission",
        // server (6)
        "AcpSendMessageRequest",
        "AcpServer",
        "AcpService",
        "ForkRequest",
        "ListSessionsResponse",
        "StreamChunk",
        // wire (5)
        "AcpError",
        "AcpNotification",
        "AcpRequest",
        "AcpResponse",
        "JSONRPC_VERSION",
    ];
    assert_eq!(
        inventory.len(),
        19,
        "argo-acp pub-use inventory drifted; update this contract with lib.rs"
    );

    // Function-pointer fences: anchor signatures so renames or arity
    // changes break the test rather than silently slipping.
    let _: fn(AuthScheme, &AuthToken) -> Result<Principal, AuthError> = verify;
    let _: fn(&AcpPermission) -> Capability = map_permission;
    let _: fn(serde_json::Value, serde_json::Value) -> AcpResponse = AcpResponse::ok;
    let _: fn(serde_json::Value, AcpError) -> AcpResponse = AcpResponse::err;
    let _: fn(&str) -> AcpError = AcpError::method_not_found;
    let _: fn(Harness) -> HarnessCapabilities = HarnessCapabilities::default_for;
    let _: fn() -> HarnessCapabilities = HarnessCapabilities::empty;
    let _: fn(Harness) -> &'static str = Harness::id;

    // Constant pin.
    assert_eq!(JSONRPC_VERSION, "2.0");
}

// ---------------------------------------------------------------------------
// 2. Harness capability defaults — VS Code, Cursor CLI, Generic, plus the
//    full set so a new variant cannot land silently.
// ---------------------------------------------------------------------------

#[test]
fn harness_capability_defaults_pin_all_four_methods_for_documented_harnesses() {
    for h in [
        Harness::CopilotCli,
        Harness::CursorCli,
        Harness::VsCode,
        Harness::Zed,
        Harness::JetBrains,
        Harness::Generic,
    ] {
        let caps = HarnessCapabilities::default_for(h);
        assert!(
            caps.send_message && caps.stream && caps.list_sessions && caps.fork_session,
            "harness {h:?} no longer advertises all four §28.Y methods"
        );
    }

    // Brief calls out VS Code, Cursor CLI, and Generic explicitly.
    assert_eq!(Harness::VsCode.id(), "vscode");
    assert_eq!(Harness::CursorCli.id(), "cursor_cli");
    assert_eq!(Harness::Generic.id(), "generic");

    let empty = HarnessCapabilities::empty();
    assert!(
        !(empty.send_message || empty.stream || empty.list_sessions || empty.fork_session),
        "HarnessCapabilities::empty must deny every method"
    );
}

#[test]
fn harness_ids_are_unique_and_stable() {
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
    assert_eq!(
        sorted.len(),
        6,
        "Harness::id() collisions break Principal::Acp.harness routing"
    );
    assert_eq!(
        sorted,
        ["copilot_cli", "cursor_cli", "generic", "jetbrains", "vscode", "zed"]
    );
}

// ---------------------------------------------------------------------------
// 3. Auth verification — success principal shape + every failure branch,
//    no live tokens or network.
// ---------------------------------------------------------------------------

#[test]
fn auth_success_returns_acp_principal_with_documented_harness_string() {
    let cases = [
        (
            AuthScheme::CopilotOAuth,
            AuthToken("gho_installation_token".into()),
            "copilot",
        ),
        (
            AuthScheme::CursorToken,
            AuthToken("cursor_project_token".into()),
            "cursor",
        ),
        (
            AuthScheme::GenericBearer,
            AuthToken("freeform-bearer".into()),
            "generic",
        ),
    ];
    for (scheme, token, expected_harness) in cases {
        let principal = verify(scheme, &token).expect("documented happy-path tokens verify");
        match principal {
            Principal::Acp { harness, identity } => {
                assert_eq!(harness, expected_harness);
                assert!(!identity.is_empty(), "identity prefix must not be empty");
            }
            other => panic!("expected Principal::Acp, got {other:?}"),
        }
    }
}

#[test]
fn auth_failure_shapes_are_pinned_without_live_tokens() {
    // Empty token → Empty.
    let err = verify(AuthScheme::GenericBearer, &AuthToken(String::new())).unwrap_err();
    assert!(matches!(err, AuthError::Empty), "empty token must yield AuthError::Empty");

    let err = verify(AuthScheme::CopilotOAuth, &AuthToken("   ".into())).unwrap_err();
    assert!(matches!(err, AuthError::Empty), "whitespace token must yield AuthError::Empty");

    // Malformed Copilot OAuth → Malformed { scheme, detail }.
    let err = verify(AuthScheme::CopilotOAuth, &AuthToken("not-a-github-token".into())).unwrap_err();
    match err {
        AuthError::Malformed { scheme, detail } => {
            assert_eq!(scheme, AuthScheme::CopilotOAuth);
            assert!(detail.contains("gho_") || detail.contains("ghu_"));
        }
        other => panic!("expected Malformed, got {other:?}"),
    }

    // Malformed Cursor token → Malformed.
    let err = verify(AuthScheme::CursorToken, &AuthToken("not-cursor".into())).unwrap_err();
    assert!(matches!(err, AuthError::Malformed { scheme: AuthScheme::CursorToken, .. }));

    // `AuthError::Rejected` exists on the wire surface even though
    // `verify()` doesn't emit it today (policy revocation lives upstream).
    // Pin its Display so downstream parsers can pattern-match on it.
    assert_eq!(
        AuthError::Rejected.to_string(),
        "token rejected by policy"
    );
}

// ---------------------------------------------------------------------------
// 4. Permission mapping — pins both path-bearing and Spawn-shaped contracts
//    and threads at least one through the real CapabilityEngine.
// ---------------------------------------------------------------------------

#[test]
fn permission_mapping_pins_every_acp_variant_to_argo_capability() {
    // Path-glob carriers.
    assert_eq!(
        map_permission(&AcpPermission::FileRead {
            path: "src/**".into()
        }),
        Capability::FsRead {
            path_glob: "src/**".into()
        }
    );
    assert_eq!(
        map_permission(&AcpPermission::FileWrite {
            path: "out/**".into()
        }),
        Capability::FsWrite {
            path_glob: "out/**".into()
        }
    );

    // Net egress: ACP only carries a host glob, port stays None.
    assert_eq!(
        map_permission(&AcpPermission::NetEgress {
            host: "*.example.com".into()
        }),
        Capability::NetEgress {
            host_glob: "*.example.com".into(),
            port: None,
        }
    );

    // Terminal + Subagent are spawn-shaped — pin the kind, not just the variant.
    match map_permission(&AcpPermission::Terminal) {
        Capability::Spawn { spawn_kind } => assert_eq!(spawn_kind, SpawnKind::Subprocess),
        other => panic!("Terminal must map to Spawn::Subprocess, got {other:?}"),
    }
    match map_permission(&AcpPermission::Subagent) {
        Capability::Spawn { spawn_kind } => assert_eq!(spawn_kind, SpawnKind::Subagent),
        other => panic!("Subagent must map to Spawn::Subagent, got {other:?}"),
    }
}

#[test]
fn permission_mapping_threads_through_argo_capability_engine() {
    // §28.Y point 5: ACP permission requests resolve through the
    // SAME capability engine that backs CLI / TUI / gateway. This
    // pins the integration contract.
    let engine = CapabilityEngine::new();
    engine
        .add_role(CapabilityRole {
            name: "acp_ide".into(),
            inherits_from: None,
            capabilities: CapabilitySet(vec![Capability::FsRead {
                path_glob: "src/**".into(),
            }]),
        })
        .expect("role registers");
    let principal = verify(AuthScheme::GenericBearer, &AuthToken("ide-1".into()))
        .expect("verify accepts non-empty generic bearer");
    engine
        .bind(principal.clone(), "acp_ide".into())
        .expect("role binds");

    // Granted FsRead under src/** → allow.
    let allowed = map_permission(&AcpPermission::FileRead {
        path: "src/main.rs".into(),
    });
    engine
        .check(&principal, &allowed)
        .expect("FileRead on src/** must be allowed via mapped Capability");

    // FsWrite not granted → deny.
    let denied = map_permission(&AcpPermission::FileWrite {
        path: "src/main.rs".into(),
    });
    assert!(
        engine.check(&principal, &denied).is_err(),
        "ungranted FsWrite must be denied through the same engine"
    );
}

// ---------------------------------------------------------------------------
// 5. JSON-RPC wire shape — request, response, error, notification all
//    round-trip with the expected on-wire field names.
// ---------------------------------------------------------------------------

#[test]
fn acp_request_response_serde_shape_for_session_send_message() {
    // Request side.
    let payload = AcpSendMessageRequest {
        session: SessionId::new(),
        text: "ping".into(),
    };
    let request = AcpRequest::new(
        serde_json::json!(42),
        "session/send_message",
        Some(serde_json::to_value(&payload).expect("payload serializes")),
    );

    let on_wire = serde_json::to_value(&request).expect("AcpRequest serializes");
    assert_eq!(on_wire["jsonrpc"], JSONRPC_VERSION);
    assert_eq!(on_wire["id"], 42);
    assert_eq!(on_wire["method"], "session/send_message");
    assert!(
        on_wire["params"]["text"].as_str() == Some("ping"),
        "params must be a nested object carrying the typed payload"
    );

    let parsed: AcpRequest = serde_json::from_value(on_wire).expect("AcpRequest round-trips");
    assert_eq!(parsed, request);
    let parsed_payload: AcpSendMessageRequest =
        serde_json::from_value(parsed.params.expect("params present"))
            .expect("payload round-trips through Value");
    assert_eq!(parsed_payload, payload);

    // Response side — success and error envelopes.
    let ok_response = AcpResponse::ok(
        serde_json::json!(42),
        serde_json::json!({"accepted": true}),
    );
    let ok_wire = serde_json::to_value(&ok_response).expect("AcpResponse::ok serializes");
    assert_eq!(ok_wire["jsonrpc"], JSONRPC_VERSION);
    assert_eq!(ok_wire["id"], 42);
    assert_eq!(ok_wire["result"]["accepted"], true);
    assert!(
        ok_wire.get("error").is_none(),
        "success response must omit `error` field on the wire"
    );

    let err_response = AcpResponse::err(
        serde_json::json!(42),
        AcpError::method_not_found("session/nope"),
    );
    let err_wire = serde_json::to_value(&err_response).expect("AcpResponse::err serializes");
    assert!(
        err_wire.get("result").is_none(),
        "error response must omit `result` field on the wire"
    );
    assert_eq!(err_wire["error"]["code"], -32601);
    assert!(
        err_wire["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("session/nope"),
        "method_not_found message must echo the missing method"
    );

    // Sanity: all four documented error constructors carry the JSON-RPC
    // codes (-32601 / -32602) and ACP-specific codes (1001 / 1003).
    assert_eq!(AcpError::invalid_params("x").code, -32602);
    assert_eq!(AcpError::unauthenticated("x").code, 1001);
    assert_eq!(AcpError::permission_denied("x").code, 1003);
}

#[test]
fn acp_notification_carries_jsonrpc_and_drops_id_field() {
    let stream = StreamChunk {
        session: SessionId::new(),
        text: "tok".into(),
        done: false,
    };
    let note = AcpNotification::new(
        "session/stream_chunk",
        Some(serde_json::to_value(&stream).expect("stream chunk serializes")),
    );
    let wire = serde_json::to_value(&note).expect("AcpNotification serializes");
    assert_eq!(wire["jsonrpc"], JSONRPC_VERSION);
    assert_eq!(wire["method"], "session/stream_chunk");
    assert!(
        wire.get("id").is_none(),
        "JSON-RPC notifications must not carry an `id` field"
    );

    let chunk_back: StreamChunk = serde_json::from_value(wire["params"].clone())
        .expect("StreamChunk round-trips through the notification envelope");
    assert_eq!(chunk_back, stream);

    // List + Fork payload shapes are pinned alongside the message
    // payload — the four advertised methods all serialize.
    let list = ListSessionsResponse {
        sessions: vec![SessionId::new(), SessionId::new()],
    };
    let list_wire = serde_json::to_value(&list).expect("ListSessionsResponse serializes");
    assert_eq!(
        list_wire["sessions"]
            .as_array()
            .map(Vec::len)
            .unwrap_or_default(),
        2
    );

    let fork = ForkRequest {
        source: SessionId::new(),
    };
    let fork_wire = serde_json::to_value(&fork).expect("ForkRequest serializes");
    assert!(
        fork_wire["source"].is_string(),
        "ForkRequest carries the source session as a string"
    );
}

// ---------------------------------------------------------------------------
// 6. AcpService + AcpServer object-safety pin: the dispatcher accepts a
//    user-supplied service and produces an AcpResponse. Catches arity /
//    associated-type drift on the trait.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ContractService {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl AcpService for ContractService {
    async fn send_message(&self, _req: AcpSendMessageRequest) -> ArgoResult<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
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

#[test]
fn acp_service_is_object_safe_and_server_is_constructible() {
    fn assert_object_safe(_svc: &dyn AcpService) {}
    let svc = ContractService::default();
    assert_object_safe(&svc);

    // Constructor surface: `AcpServer::new(Harness, AuthScheme, S)`.
    let server = AcpServer::new(Harness::VsCode, AuthScheme::GenericBearer, svc);
    assert_eq!(server.harness, Harness::VsCode);
    assert_eq!(server.auth_scheme, AuthScheme::GenericBearer);
    assert!(server.capabilities.send_message);
}
