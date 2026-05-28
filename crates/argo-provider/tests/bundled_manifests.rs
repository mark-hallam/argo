//! All v1 bundled manifests must deserialize cleanly (§8.6 acceptance —
//! covers the four HTTP-family manifests plus the subprocess-kind
//! `claude-code-cli.toml` per brief `provider-claude-code-cli-subprocess`).
use argo_provider::{ProviderFamily, ProviderManifest, ToolCallFormat, TransportKind};

fn parse(name: &str) -> ProviderManifest {
    let path = format!("../../manifests/providers/{name}");
    let toml =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read manifest {path}: {e}"));
    ProviderManifest::from_toml_str(&toml)
        .unwrap_or_else(|e| panic!("parse manifest {name}: {e:?}"))
}

#[test]
fn openai_compatible_manifest_loads_with_expected_family() {
    let m = parse("openai-compatible.toml");
    assert_eq!(m.provider.family, ProviderFamily::OpenaiCompatible);
    assert_eq!(m.tool_calls.format, ToolCallFormat::OpenaiToolCalls);
    assert!(m.discovery.is_some());
    assert!(m.transport.heartbeat_event.is_some());
}

#[test]
fn openai_responses_manifest_loads_with_expected_chat_path() {
    let m = parse("openai-responses.toml");
    assert_eq!(m.provider.family, ProviderFamily::OpenaiResponses);
    assert_eq!(m.tool_calls.format, ToolCallFormat::OpenaiResponseTools);
    assert_eq!(m.transport.chat_path, "/v1/responses");
}

#[test]
fn anthropic_manifest_loads_with_quirks_and_models() {
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
fn bedrock_manifest_loads_with_aws_region_quirk() {
    let m = parse("bedrock.toml");
    assert_eq!(m.provider.family, ProviderFamily::Bedrock);
    assert_eq!(m.tool_calls.format, ToolCallFormat::BedrockToolUse);
    assert!(m
        .quirks
        .get("aws-region")
        .and_then(|v| v.as_str())
        .is_some());
}

#[test]
fn claude_code_cli_manifest_loads_with_subprocess_block() {
    let m = parse("claude-code-cli.toml");
    assert_eq!(m.provider.id, "claude-code-cli");
    // Family stays Anthropic — same response shape — but transport kind
    // is what selects the subprocess driver.
    assert_eq!(m.provider.family, ProviderFamily::Anthropic);
    assert_eq!(m.provider.transport_kind, TransportKind::Subprocess);
    let sp = m.subprocess.expect("subprocess block present");
    assert_eq!(sp.binary_path_env, "ARGO_CLAUDE_CODE_BINARY");
    assert_eq!(sp.binary_path_fallback, "claude");
    assert_eq!(sp.default_args, vec!["--print", "--output-format", "json"]);
    assert!(!sp.streaming, "v1 driver is one-shot only");
    assert!(sp.timeout_ms >= 1_000, "timeout should be a sane positive");
}

#[test]
fn http_manifests_default_to_transport_kind_http() {
    for name in ["anthropic.toml", "openai-compatible.toml", "openai-responses.toml", "bedrock.toml"] {
        let m = parse(name);
        assert_eq!(
            m.provider.transport_kind,
            TransportKind::Http,
            "{name} should default to HTTP transport kind",
        );
        assert!(
            m.subprocess.is_none(),
            "{name} must not carry a [subprocess] block",
        );
    }
}
