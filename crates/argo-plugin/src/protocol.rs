//! JSON-RPC 2.0 framing types used over plugin stdin/stdout.
//!
//! Argo's plugin transport is **line-delimited JSON**: each request,
//! response, or notification is one JSON object terminated by `\n`.
//! That lets a plugin written in any language (Python, TypeScript,
//! Rust, …) implement the protocol with `print(json.dumps(...))` and
//! `for line in sys.stdin:`.
//!
//! Spec: `docs/plan/argo-build-plan.md` §11.
//!
//! Forward-compat rules, error-code constants, and version-compat
//! classification live in [`crate::abi`]. Wire-shape contract tests
//! that pin the JSON exactly live in `tests/abi_contract.rs`.
use serde::{Deserialize, Serialize};

/// Pinned protocol version. Bumped when wire-shape changes; the
/// `protocol_version_pinned` contract test in `tests/abi_contract.rs`
/// fails on any change so the bump is deliberate and reviewed.
pub const PROTOCOL_VERSION: &str = "0.1.0";

/// One Argo → plugin (or plugin → Argo) request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginJsonRpcRequest {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Correlation id.
    pub id: serde_json::Value,
    /// Method name (e.g. `"pre_tool_call"`).
    pub method: String,
    /// Params object — schema is method-specific.
    #[serde(default)]
    pub params: serde_json::Value,
}

impl PluginJsonRpcRequest {
    /// Build a request with the next sequential id.
    #[must_use]
    pub fn new(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: serde_json::Value::Number(id.into()),
            method: method.into(),
            params,
        }
    }
}

/// One response to a request. `result` and `error` are mutually
/// exclusive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginJsonRpcResponse {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Echo of the request's id.
    pub id: serde_json::Value,
    /// Success body — present iff `error` is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error body — present iff `result` is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<PluginJsonRpcError>,
}

/// One error within a [`PluginJsonRpcResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginJsonRpcError {
    /// JSON-RPC error code.
    pub code: i64,
    /// Human-readable message.
    pub message: String,
    /// Optional structured payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Body of a `handshake` request (Argo → plugin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    /// Protocol version Argo speaks.
    pub protocol_version: String,
}

/// Body of a `handshake` response (plugin → Argo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    /// Protocol version the plugin agrees to.
    pub protocol_version: String,
    /// Plugin name (matches manifest / config).
    pub name: String,
    /// Plugin version.
    #[serde(default)]
    pub version: String,
    /// Capabilities the plugin requests up-front (free-form for v0.2;
    /// the host may query the user / config to grant or deny).
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Verdict returned by `pre_tool_call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PreToolCallVerdict {
    /// Let the tool dispatch proceed unchanged.
    Allow,
    /// Veto the call — the loop must skip it.
    Deny {
        /// Reason surfaced to the audit log.
        reason: String,
    },
    /// Replace the call's arguments.
    Transform {
        /// New arguments (must match the tool's JSON schema).
        args: serde_json::Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_through_json() {
        let r = PluginJsonRpcRequest::new(1, "pre_tool_call", serde_json::json!({"tool": "ls"}));
        let s = serde_json::to_string(&r).unwrap();
        let back: PluginJsonRpcRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(back.method, "pre_tool_call");
        assert_eq!(back.id, serde_json::json!(1));
    }

    #[test]
    fn response_with_result_omits_error_field() {
        let r = PluginJsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(7),
            result: Some(serde_json::json!({"action": "allow"})),
            error: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("\"error\""));
    }

    #[test]
    fn pre_tool_call_verdict_round_trips() {
        let verdicts = [
            PreToolCallVerdict::Allow,
            PreToolCallVerdict::Deny {
                reason: "policy".into(),
            },
            PreToolCallVerdict::Transform {
                args: serde_json::json!({"x": 1}),
            },
        ];
        for v in verdicts {
            let s = serde_json::to_string(&v).unwrap();
            let back: PreToolCallVerdict = serde_json::from_str(&s).unwrap();
            assert_eq!(format!("{back:?}"), format!("{v:?}"));
        }
    }
}
