//! ABI stability pins for the public `argo-plugin` JSON-RPC surface.
//!
//! These are audit-oriented contract tests. They pin the host-sendable
//! envelopes and the minimal decode shapes that out-of-tree plugin SDKs
//! depend on, so ABI drift fails before it reaches a release.

use argo_plugin::{
    classify_compatibility, is_wire_compatible, AbiCompatibility, HandshakeRequest,
    HandshakeResponse, PluginJsonRpcError, PluginJsonRpcRequest, PluginJsonRpcResponse,
    PreToolCallVerdict, JSONRPC_VERSION, PROTOCOL_VERSION, RPC_INTERNAL_ERROR, RPC_INVALID_PARAMS,
    RPC_INVALID_REQUEST, RPC_METHOD_NOT_FOUND, RPC_PARSE_ERROR,
};
use serde_json::{from_value, json, to_value, Value};

fn request(id: u64, method: &str, params: Value) -> Value {
    to_value(PluginJsonRpcRequest::new(id, method, params)).unwrap()
}

#[test]
fn protocol_and_jsonrpc_constants_are_pinned() {
    assert_eq!(PROTOCOL_VERSION, "0.1.0");
    assert_eq!(JSONRPC_VERSION, "2.0");
    assert_eq!(RPC_PARSE_ERROR, -32700);
    assert_eq!(RPC_INVALID_REQUEST, -32600);
    assert_eq!(RPC_METHOD_NOT_FOUND, -32601);
    assert_eq!(RPC_INVALID_PARAMS, -32602);
    assert_eq!(RPC_INTERNAL_ERROR, -32603);
}

#[test]
fn host_handshake_request_envelope_is_pinned() {
    let params = to_value(HandshakeRequest {
        protocol_version: PROTOCOL_VERSION.into(),
    })
    .unwrap();

    assert_eq!(
        request(1, "handshake", params),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "handshake",
            "params": {"protocol_version": "0.1.0"},
        })
    );
}

#[test]
fn host_method_request_catalogue_envelopes_are_pinned() {
    assert_eq!(
        request(
            2,
            "pre_tool_call",
            json!({"tool": "terminal", "args": {"cmd": "date"}}),
        ),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "pre_tool_call",
            "params": {"tool": "terminal", "args": {"cmd": "date"}},
        })
    );

    assert_eq!(
        request(3, "ping", json!({"hello": "world"})),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "ping",
            "params": {"hello": "world"},
        })
    );

    assert_eq!(
        request(0, "shutdown", json!({})),
        json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "shutdown",
            "params": {},
        })
    );
}

#[test]
fn handshake_response_minimal_and_full_shapes_are_pinned() {
    let minimal: HandshakeResponse =
        from_value(json!({"protocol_version": "0.1.0", "name": "echo"})).unwrap();
    assert_eq!(minimal.protocol_version, PROTOCOL_VERSION);
    assert_eq!(minimal.name, "echo");
    assert_eq!(minimal.version, "");
    assert!(minimal.capabilities.is_empty());

    let full = HandshakeResponse {
        protocol_version: PROTOCOL_VERSION.into(),
        name: "echo".into(),
        version: "0.1.0".into(),
        capabilities: vec!["pre_tool_call".into(), "shutdown".into()],
    };
    assert_eq!(
        to_value(full).unwrap(),
        json!({
            "protocol_version": "0.1.0",
            "name": "echo",
            "version": "0.1.0",
            "capabilities": ["pre_tool_call", "shutdown"],
        })
    );
}

#[test]
fn response_success_and_error_shapes_are_pinned() {
    let success = PluginJsonRpcResponse {
        jsonrpc: JSONRPC_VERSION.into(),
        id: json!(9),
        result: Some(json!({"action": "allow"})),
        error: None,
    };
    assert_eq!(
        to_value(&success).unwrap(),
        json!({"jsonrpc": "2.0", "id": 9, "result": {"action": "allow"}})
    );

    let error = PluginJsonRpcResponse {
        jsonrpc: JSONRPC_VERSION.into(),
        id: json!("req-9"),
        result: None,
        error: Some(PluginJsonRpcError {
            code: RPC_METHOD_NOT_FOUND,
            message: "method not found: future_method".into(),
            data: None,
        }),
    };
    assert_eq!(
        to_value(&error).unwrap(),
        json!({
            "jsonrpc": "2.0",
            "id": "req-9",
            "error": {"code": -32601, "message": "method not found: future_method"},
        })
    );
}

#[test]
fn pre_tool_call_verdict_wire_variants_are_pinned() {
    assert_eq!(
        to_value(PreToolCallVerdict::Allow).unwrap(),
        json!({"action": "allow"})
    );
    assert_eq!(
        to_value(PreToolCallVerdict::Deny {
            reason: "policy".into(),
        })
        .unwrap(),
        json!({"action": "deny", "reason": "policy"})
    );
    assert_eq!(
        to_value(PreToolCallVerdict::Transform {
            args: json!({"cmd": "ls", "path": "/tmp"}),
        })
        .unwrap(),
        json!({"action": "transform", "args": {"cmd": "ls", "path": "/tmp"}})
    );
}

#[test]
fn additive_fields_remain_accepted_on_public_wire_types() {
    let req: PluginJsonRpcRequest = from_value(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": {},
        "trace_id": "future-trace",
    }))
    .unwrap();
    assert_eq!(req.method, "ping");

    let resp: PluginJsonRpcResponse = from_value(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"ok": true},
        "elapsed_ms": 7,
    }))
    .unwrap();
    assert_eq!(resp.result.unwrap(), json!({"ok": true}));

    let handshake: HandshakeResponse = from_value(json!({
        "protocol_version": "0.1.0",
        "name": "future-plugin",
        "version": "0.2.0",
        "capabilities": ["pre_tool_call"],
        "sdk_language": "typescript",
    }))
    .unwrap();
    assert_eq!(handshake.name, "future-plugin");
}

#[test]
fn non_optional_fields_are_currently_required() {
    assert!(from_value::<HandshakeRequest>(json!({})).is_err());
    assert!(from_value::<HandshakeResponse>(json!({"name": "echo"})).is_err());
    assert!(from_value::<HandshakeResponse>(json!({"protocol_version": "0.1.0"})).is_err());

    assert!(
        from_value::<PluginJsonRpcRequest>(json!({"id": 1, "method": "ping", "params": {}}))
            .is_err()
    );
    assert!(from_value::<PluginJsonRpcRequest>(
        json!({"jsonrpc": "2.0", "method": "ping", "params": {}})
    )
    .is_err());
    assert!(
        from_value::<PluginJsonRpcRequest>(json!({"jsonrpc": "2.0", "id": 1, "params": {}}))
            .is_err()
    );

    assert!(from_value::<PluginJsonRpcResponse>(json!({"id": 1, "result": {}})).is_err());
    assert!(from_value::<PluginJsonRpcResponse>(json!({"jsonrpc": "2.0", "result": {}})).is_err());

    assert!(from_value::<PluginJsonRpcError>(json!({"message": "missing code"})).is_err());
    assert!(from_value::<PluginJsonRpcError>(json!({"code": -32603})).is_err());
}

#[test]
fn optional_fields_default_without_expanding_the_required_surface() {
    let req: PluginJsonRpcRequest =
        from_value(json!({"jsonrpc": "2.0", "id": "abc", "method": "ping"})).unwrap();
    assert_eq!(req.params, Value::Null);

    let resp: PluginJsonRpcResponse =
        from_value(json!({"jsonrpc": "2.0", "id": 4, "result": {"pong": true}})).unwrap();
    assert!(resp.error.is_none());

    let err: PluginJsonRpcError =
        from_value(json!({"code": -32603, "message": "internal"})).unwrap();
    assert!(err.data.is_none());
}

#[test]
fn version_classifier_contract_is_pinned_but_host_remains_strict() {
    assert_eq!(
        classify_compatibility(PROTOCOL_VERSION, PROTOCOL_VERSION),
        AbiCompatibility::Identical
    );
    assert_eq!(
        classify_compatibility(PROTOCOL_VERSION, "0.1.7"),
        AbiCompatibility::PluginNewerMinor
    );
    assert!(is_wire_compatible(PROTOCOL_VERSION, "0.1.7"));

    // `host::handshake` still compares exact strings against this constant.
    // A future swap to `is_wire_compatible` should update this pin and report.
    assert_eq!(PROTOCOL_VERSION, "0.1.0");
}
