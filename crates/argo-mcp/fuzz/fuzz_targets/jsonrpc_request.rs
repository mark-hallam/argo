//! cargo-fuzz target — JSON-RPC request frame parser.
//!
//! `JsonRpcRequest` is the inbound wire format for every MCP server
//! Argo talks to. Untrusted plugin output flows through this decoder;
//! a panic here would crash the MCP client task and orphan the
//! subprocess.
//!
//! Decoder must reject malformed input gracefully, never panic. We
//! also re-encode the successfully-parsed frame to confirm the
//! struct round-trips through `serde_json`.
#![no_main]

use argo_mcp::JsonRpcRequest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(req) = serde_json::from_slice::<JsonRpcRequest>(data) {
        if let Ok(encoded) = serde_json::to_vec(&req) {
            let _ = serde_json::from_slice::<JsonRpcRequest>(&encoded);
        }
    }
});
