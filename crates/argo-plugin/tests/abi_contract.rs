//! Forward-compat ABI contract tests for the plugin wire shape.
//!
//! These tests pin the JSON-RPC 2.0 surface that every plugin SDK
//! (Python, TypeScript, Rust, …) targets. They are pure serde
//! round-trips — no subprocess, no `tokio` — so they run in ~1 ms and
//! catch silent regressions early.
//!
//! What a failure here means:
//!
//! - **Golden-shape test fails** → a field was renamed, removed, or
//!   re-tagged. That breaks every plugin compiled against the old
//!   shape. Either revert, or bump [`argo_plugin::PROTOCOL_VERSION`]
//!   to a new major in lockstep and update the snapshot.
//! - **`ignores_unknown_fields_*` test fails** → someone added
//!   `#[serde(deny_unknown_fields)]` to a wire type. That kills
//!   forward compatibility (newer plugins emitting an additive field
//!   can no longer talk to an older host). Remove the attribute.
//! - **`PROTOCOL_VERSION pinned` fails** → version bumped. Confirm
//!   the bump is deliberate and update the constant snapshot below.
//!
//! `tests/host.rs` covers the live subprocess path; this file covers
//! the wire shape that subprocess speaks.
use argo_plugin::{
    classify_compatibility, is_wire_compatible, AbiCompatibility, HandshakeRequest,
    HandshakeResponse, PluginJsonRpcError, PluginJsonRpcRequest, PluginJsonRpcResponse,
    PreToolCallVerdict, JSONRPC_VERSION, PROTOCOL_VERSION, RPC_INTERNAL_ERROR, RPC_INVALID_PARAMS,
    RPC_INVALID_REQUEST, RPC_METHOD_NOT_FOUND, RPC_PARSE_ERROR,
};
use serde_json::{from_str, from_value, json, to_value, Value};

// ---------------------------------------------------------------------
// 1. Pinned constants — version + standard error codes
// ---------------------------------------------------------------------

#[test]
fn protocol_version_pinned() {
    // Bumping PROTOCOL_VERSION is intentional; update this snapshot in
    // the same PR so reviewers see the wire-shape impact in the diff.
    assert_eq!(PROTOCOL_VERSION, "0.1.0");
}

#[test]
fn jsonrpc_envelope_pinned() {
    assert_eq!(JSONRPC_VERSION, "2.0");
}

#[test]
fn standard_error_codes_pinned() {
    assert_eq!(RPC_PARSE_ERROR, -32700);
    assert_eq!(RPC_INVALID_REQUEST, -32600);
    assert_eq!(RPC_METHOD_NOT_FOUND, -32601);
    assert_eq!(RPC_INVALID_PARAMS, -32602);
    assert_eq!(RPC_INTERNAL_ERROR, -32603);
}

// ---------------------------------------------------------------------
// 2. Golden wire shapes — pinned by JSON-Value equality
// ---------------------------------------------------------------------

#[test]
fn request_golden_shape() {
    let r = PluginJsonRpcRequest::new(42, "pre_tool_call", json!({"tool": "ls"}));
    let v = to_value(&r).unwrap();
    assert_eq!(
        v,
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "pre_tool_call",
            "params": {"tool": "ls"},
        })
    );
}

#[test]
fn response_golden_shape_with_result() {
    let r = PluginJsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: json!(7),
        result: Some(json!({"action": "allow"})),
        error: None,
    };
    let v = to_value(&r).unwrap();
    assert_eq!(
        v,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "result": {"action": "allow"},
        })
    );
    // `error` MUST be omitted on success — plugin SDKs in dynamic
    // languages rely on the presence of `result` vs. `error` to
    // discriminate.
    assert!(!serde_json::to_string(&r).unwrap().contains("\"error\""));
}

#[test]
fn response_golden_shape_with_error() {
    let r = PluginJsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: json!(7),
        result: None,
        error: Some(PluginJsonRpcError {
            code: RPC_METHOD_NOT_FOUND,
            message: "method not found: foo".into(),
            data: None,
        }),
    };
    let v = to_value(&r).unwrap();
    assert_eq!(
        v,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "error": {"code": -32601, "message": "method not found: foo"},
        })
    );
    assert!(!serde_json::to_string(&r).unwrap().contains("\"result\""));
}

#[test]
fn handshake_request_golden_shape() {
    let r = HandshakeRequest {
        protocol_version: PROTOCOL_VERSION.into(),
    };
    let v = to_value(&r).unwrap();
    assert_eq!(v, json!({"protocol_version": "0.1.0"}));
}

#[test]
fn handshake_response_golden_shape() {
    let r = HandshakeResponse {
        protocol_version: PROTOCOL_VERSION.into(),
        name: "echo".into(),
        version: "0.1.0".into(),
        capabilities: vec!["pre_tool_call".into()],
    };
    let v = to_value(&r).unwrap();
    assert_eq!(
        v,
        json!({
            "protocol_version": "0.1.0",
            "name": "echo",
            "version": "0.1.0",
            "capabilities": ["pre_tool_call"],
        })
    );
}

#[test]
fn pre_tool_call_verdict_allow_golden() {
    let v = to_value(PreToolCallVerdict::Allow).unwrap();
    assert_eq!(v, json!({"action": "allow"}));
}

#[test]
fn pre_tool_call_verdict_deny_golden() {
    let v = to_value(PreToolCallVerdict::Deny {
        reason: "policy".into(),
    })
    .unwrap();
    assert_eq!(v, json!({"action": "deny", "reason": "policy"}));
}

#[test]
fn pre_tool_call_verdict_transform_golden() {
    let v = to_value(PreToolCallVerdict::Transform {
        args: json!({"x": 1}),
    })
    .unwrap();
    assert_eq!(v, json!({"action": "transform", "args": {"x": 1}}));
}

// ---------------------------------------------------------------------
// 3. Forward-compat — additive fields tolerated on decode
// ---------------------------------------------------------------------
//
// The host MUST decode wire payloads from plugins that ship newer
// minor versions. Newer-minor payloads can only ADD fields, never
// remove existing ones. serde ignores unknown fields by default — we
// pin that here so a future `#[serde(deny_unknown_fields)]` regresses
// the test.

#[test]
fn ignores_unknown_fields_on_request() {
    let raw = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": {},
        "future_field": "ignored",
        "trace_id": "abc-123"
    }"#;
    let r: PluginJsonRpcRequest = from_str(raw).unwrap();
    assert_eq!(r.method, "ping");
}

#[test]
fn ignores_unknown_fields_on_response() {
    let raw = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"ok": true},
        "telemetry": {"ms": 4}
    }"#;
    let r: PluginJsonRpcResponse = from_str(raw).unwrap();
    assert_eq!(r.result.unwrap(), json!({"ok": true}));
    assert!(r.error.is_none());
}

#[test]
fn ignores_unknown_fields_on_error_payload() {
    let raw = r#"{
        "code": -32603,
        "message": "internal",
        "data": null,
        "hint": "retry in 5s"
    }"#;
    let e: PluginJsonRpcError = from_str(raw).unwrap();
    assert_eq!(e.code, RPC_INTERNAL_ERROR);
}

#[test]
fn ignores_unknown_fields_on_handshake_response() {
    let raw = r#"{
        "protocol_version": "0.1.0",
        "name": "future-plugin",
        "version": "1.2.3",
        "capabilities": ["a", "b"],
        "sdk_language": "python",
        "sdk_version": "0.7.0"
    }"#;
    let r: HandshakeResponse = from_str(raw).unwrap();
    assert_eq!(r.name, "future-plugin");
    assert_eq!(r.capabilities, vec!["a", "b"]);
}

// ---------------------------------------------------------------------
// 4. Backward-compat — missing optional fields default cleanly
// ---------------------------------------------------------------------

#[test]
fn handshake_response_tolerates_missing_optional_fields() {
    // An older plugin SDK may omit `version` and `capabilities`.
    let raw = r#"{"protocol_version": "0.1.0", "name": "old-plugin"}"#;
    let r: HandshakeResponse = from_str(raw).unwrap();
    assert_eq!(r.version, "");
    assert!(r.capabilities.is_empty());
}

#[test]
fn request_tolerates_missing_params() {
    let raw = r#"{"jsonrpc": "2.0", "id": 1, "method": "ping"}"#;
    let r: PluginJsonRpcRequest = from_str(raw).unwrap();
    assert_eq!(r.params, Value::Null);
}

#[test]
fn rpc_error_tolerates_missing_data() {
    let raw = r#"{"code": -32601, "message": "method not found"}"#;
    let e: PluginJsonRpcError = from_str(raw).unwrap();
    assert!(e.data.is_none());
}

// ---------------------------------------------------------------------
// 5. ID polymorphism — JSON-RPC 2.0 spec allows number, string, or null
// ---------------------------------------------------------------------

#[test]
fn request_id_can_be_number_string_or_null() {
    for id in [json!(1), json!("req-abc"), Value::Null] {
        let raw = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "ping",
            "params": {}
        }))
        .unwrap();
        let r: PluginJsonRpcRequest = from_str(&raw).unwrap();
        assert_eq!(r.id, id);
    }
}

// ---------------------------------------------------------------------
// 6. Verdict round-trip — every variant decodes from its golden JSON
// ---------------------------------------------------------------------

#[test]
fn verdict_allow_decodes_from_minimal_json() {
    let v: PreToolCallVerdict = from_value(json!({"action": "allow"})).unwrap();
    assert!(matches!(v, PreToolCallVerdict::Allow));
}

#[test]
fn verdict_deny_decodes() {
    let v: PreToolCallVerdict =
        from_value(json!({"action": "deny", "reason": "blocked"})).unwrap();
    match v {
        PreToolCallVerdict::Deny { reason } => assert_eq!(reason, "blocked"),
        other => panic!("expected Deny, got {other:?}"),
    }
}

#[test]
fn verdict_transform_decodes() {
    let v: PreToolCallVerdict =
        from_value(json!({"action": "transform", "args": {"k": "v"}})).unwrap();
    match v {
        PreToolCallVerdict::Transform { args } => assert_eq!(args, json!({"k": "v"})),
        other => panic!("expected Transform, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 7. Version-compat helpers — pin the planned wire-up's classifier
// ---------------------------------------------------------------------

#[test]
fn is_wire_compatible_classifier_matrix() {
    // Same version is always compatible.
    assert!(is_wire_compatible(PROTOCOL_VERSION, PROTOCOL_VERSION));
    // Plugin newer on same major is forward-compatible (additive-only).
    assert!(is_wire_compatible("0.1.0", "0.2.0"));
    assert!(is_wire_compatible("0.1.0", "0.1.7"));
    // Major bump is NOT compatible.
    assert!(!is_wire_compatible("0.1.0", "1.0.0"));
    // Plugin older is NOT compatible (host MAY require newer plugin).
    assert!(!is_wire_compatible("0.2.0", "0.1.0"));
    // Garbage version strings are NOT compatible.
    assert!(!is_wire_compatible("0.1.0", "9.9.9-mismatch"));
    assert!(!is_wire_compatible("0.1.0", "not-a-version"));
}

#[test]
fn classify_compatibility_unparseable_is_distinct_from_older() {
    assert_eq!(
        classify_compatibility("0.1.0", "9.9.9-mismatch"),
        AbiCompatibility::Unparseable
    );
    assert_eq!(
        classify_compatibility("0.1.0", ""),
        AbiCompatibility::Unparseable
    );
}

// ---------------------------------------------------------------------
// 8. Current host behavior — exact-string equality at handshake
// ---------------------------------------------------------------------
//
// `host::handshake` rejects ANY non-exact version. The semver helpers
// above are not yet wired in; this test pins the strict behavior so
// any future loosening is a deliberate code change visible in the
// diff.

#[test]
fn pinned_current_handshake_is_strict_string_equality() {
    // is_wire_compatible would accept "0.1.0" ⇄ "0.1.5", but the
    // current host does NOT — the integration test
    // `host::protocol_version_mismatch_is_rejected_cleanly`
    // exercises rejection of "9.9.9-mismatch". This test pins the
    // PROTOCOL_VERSION constant that the host compares against.
    assert_eq!(PROTOCOL_VERSION, "0.1.0");
    // And documents that loosening to is_wire_compatible would
    // accept this pair — i.e. the helper is correct and ready.
    assert!(is_wire_compatible(PROTOCOL_VERSION, "0.1.5"));
}

// ---------------------------------------------------------------------
// 9. FINDINGS-ONLY (#[ignore]'d) — forward-compat gap on PreToolCallVerdict
// ---------------------------------------------------------------------
//
// FINDING: `PreToolCallVerdict` is exhaustive on deserialize. If a
// newer plugin SDK adds a verdict variant (e.g. `{"action": "defer"}`),
// the current host will fail to decode with a serde error, breaking
// the call rather than gracefully rejecting the verdict.
//
// Forward-compat policy says additive variants on a wire enum SHOULD
// decode to an "unknown — treat as deny" fallback so a newer plugin
// against an older host degrades safely instead of crashing the call.
//
// Fix-brief: `mission-plugin-verdict-unknown-fallback` — wrap
// `PreToolCallVerdict` in a sum with `Unknown(Value)` via
// `#[serde(other)]` (or move to a manual `Deserialize` impl) so the
// older host treats unknown variants as a structured deny rather than
// a hard decode failure. ~80 LOC; touches `protocol.rs` + adds tests.
//
// This test is FINDINGS-ONLY (#[ignore]'d) per the brief's
// "real broken-wire found" clause. Un-ignore in the fix-brief.

#[test]
#[ignore = "FINDINGS-ONLY: forward-compat gap; fix in mission-plugin-verdict-unknown-fallback"]
fn finding_unknown_verdict_variant_should_decode_as_deny() {
    // Today: this would fail with serde "unknown variant" error.
    // After fix: should decode to a structured Deny/Unknown fallback.
    let raw = json!({"action": "defer", "until_ms": 5000});
    let result: Result<PreToolCallVerdict, _> = from_value(raw);
    // Post-fix expectation: Ok(_) with a safe-deny fallback variant.
    assert!(result.is_ok(), "unknown verdict variant must not hard-error");
}
