//! All four v1 bundled manifests must deserialize cleanly (§8.6 acceptance).
use argo_provider::{ProviderFamily, ProviderManifest, ToolCallFormat};

fn parse(name: &str) -> ProviderManifest {
    let path = format!("../../manifests/providers/{name}");
    let toml =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read manifest {path}: {e}"));
    ProviderManifest::from_toml_str(&toml)
        .unwrap_or_else(|e| panic!("parse manifest {name}: {e:?}"))
}

#[test]
fn openai_compatible() {
    let m = parse("openai-compatible.toml");
    assert_eq!(m.provider.family, ProviderFamily::OpenaiCompatible);
    assert_eq!(m.tool_calls.format, ToolCallFormat::OpenaiToolCalls);
    assert!(m.discovery.is_some());
    assert!(m.transport.heartbeat_event.is_some());
}

#[test]
fn openai_responses() {
    let m = parse("openai-responses.toml");
    assert_eq!(m.provider.family, ProviderFamily::OpenaiResponses);
    assert_eq!(m.tool_calls.format, ToolCallFormat::OpenaiResponseTools);
    assert_eq!(m.transport.chat_path, "/v1/responses");
}

#[test]
fn anthropic() {
    let m = parse("anthropic.toml");
    assert_eq!(m.provider.family, ProviderFamily::Anthropic);
    assert_eq!(m.tool_calls.format, ToolCallFormat::AnthropicToolUse);
    assert_eq!(m.auth.header_name, "x-api-key");
    // Anthropic-specific quirks must be present and consumable as-is.
    assert!(matches!(
        m.quirks.get("send-thinking-blocks"),
        Some(toml::Value::Boolean(true))
    ));
    assert!(m
        .quirks
        .get("prompt-caching-breakpoints")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false));
    let model_ids: Vec<_> = m.models.iter().map(|m| m.id.as_str()).collect();
    assert!(model_ids.contains(&"claude-haiku-4-5"));
    assert!(model_ids.contains(&"claude-opus-4-6"));
}

#[test]
fn bedrock() {
    let m = parse("bedrock.toml");
    assert_eq!(m.provider.family, ProviderFamily::Bedrock);
    assert_eq!(m.tool_calls.format, ToolCallFormat::BedrockToolUse);
    assert!(m
        .quirks
        .get("aws-region")
        .and_then(|v| v.as_str())
        .is_some());
}
