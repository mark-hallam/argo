//! Optional real-binary smoke test for the Claude Code CLI subprocess
//! provider. Not run in CI — only invoked by hand when the operator
//! wants to validate the round-trip against a real `claude` binary.
//!
//! Run with:
//!   cargo run -p argo-provider --example real_binary_smoke
//!
//! Output: a one-paragraph response + token counts + selected model.
use argo_provider::{ClaudeCodeCliTransport, ProviderManifest, ProviderRequest, Sampling, Transport};
use argo_types::{ContentBlock, Message, MessageId, MessageRole, SessionId};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_toml = std::fs::read_to_string("manifests/providers/claude-code-cli.toml")?;
    let manifest = ProviderManifest::from_toml_str(&manifest_toml)?;
    let transport = ClaudeCodeCliTransport::from_manifest(manifest)?;

    let req = ProviderRequest {
        model: "claude-sonnet-4-6".into(),
        messages: vec![Message {
            id: MessageId::new(),
            session_id: SessionId::new(),
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: "Say exactly the word: pong".into(),
            }],
            tool_calls: vec![],
            tool_call_id: None,
            reasoning: None,
            created_at: Utc::now(),
            provider_meta: serde_json::Value::Null,
        }],
        tools: vec![],
        system: Some("You are a smoke-test responder. Output only what the user asks.".into()),
        sampling: Sampling::default(),
    };

    let resp = transport.complete(&req).await?;
    println!("text: {:?}", resp.text);
    println!("tokens_in: {:?}", resp.tokens_in);
    println!("tokens_out: {:?}", resp.tokens_out);
    println!("model: {}", resp.provider_meta["model"]);
    println!("stop_reason: {:?}", resp.stop_reason);
    Ok(())
}
