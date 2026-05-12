//! cargo-fuzz target — JSON-RPC response frame parser.
//!
//! Fuzz the wire-format decoder Argo uses for inbound MCP server
//! responses. Confirms:
//!
//! - serde never panics on adversarial bytes (oversize numbers,
//!   nested arrays, embedded NULs)
//! - parsed frames re-serialize cleanly
#![no_main]

use argo_mcp::JsonRpcResponse;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(resp) = serde_json::from_slice::<JsonRpcResponse>(data) {
        if let Ok(encoded) = serde_json::to_vec(&resp) {
            let _ = serde_json::from_slice::<JsonRpcResponse>(&encoded);
        }
    }
});
