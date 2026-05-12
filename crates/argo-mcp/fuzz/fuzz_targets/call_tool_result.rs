//! cargo-fuzz target — `tools/call` response content decoder.
//!
//! The `CallToolResult.content` shape is `McpContent`, whose serde
//! deserializer uses an `#[serde(other)]` catch-all for unrecognised
//! `type` tags. Fuzz the decoder against random bytes to confirm:
//!
//! - no panic on unknown content variants
//! - the catch-all path keeps the parser flowing
#![no_main]

use argo_mcp::CallToolResult;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<CallToolResult>(data);
});
