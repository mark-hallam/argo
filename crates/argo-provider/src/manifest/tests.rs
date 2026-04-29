use super::*;

fn minimal_toml() -> &'static str {
    r#"
[provider]
id = "test"
family = "openai-compatible"
display_name = "Test"
base_url = "https://api.test"

[auth]
api_key_env = "TEST_KEY"

[transport]
streaming = "sse"

[tool_calls]
format = "openai_tool_calls"
    "#
}

#[test]
fn parses_minimal_with_defaults() {
    let m = ProviderManifest::from_toml_str(minimal_toml()).unwrap();
    assert_eq!(m.transport.chat_path, "/v1/chat/completions");
    assert_eq!(m.transport.connect_timeout_ms, 5_000);
    assert_eq!(m.auth.header_name, "Authorization");
    assert_eq!(m.auth.header_format, "Bearer {key}");
}

#[test]
fn rejects_trailing_slash_base_url() {
    let bad = minimal_toml().replace("https://api.test", "https://api.test/");
    let err = ProviderManifest::from_toml_str(&bad).unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn rejects_non_http_base_url() {
    let bad = minimal_toml().replace("https://api.test", "ftp://api.test");
    let err = ProviderManifest::from_toml_str(&bad).unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn rejects_missing_key_placeholder() {
    let toml = r#"
[provider]
id = "x"
family = "openai-compatible"
display_name = "X"
base_url = "https://api.x"

[auth]
api_key_env = "K"
header_format = "NoKeyHere"

[transport]
streaming = "sse"

[tool_calls]
format = "openai_tool_calls"
    "#;
    let err = ProviderManifest::from_toml_str(toml).unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn capabilities_set_includes_api_key_env() {
    let m = ProviderManifest::from_toml_str(minimal_toml()).unwrap();
    let set = m.capabilities.to_set(&m.auth.api_key_env);
    assert!(set.covers(&Capability::EnvRead {
        var_glob: "TEST_KEY".into()
    }));
}

#[test]
fn resolve_latest_uses_first_model() {
    let toml = r#"
[provider]
id = "x"
family = "openai-compatible"
display_name = "X"
base_url = "https://api.x"
[auth]
api_key_env = "K"
[transport]
streaming = "sse"
[tool_calls]
format = "openai_tool_calls"
[[models]]
id = "gpt-4o-mini"
[[models]]
id = "gpt-4o"
    "#;
    let m = ProviderManifest::from_toml_str(toml).unwrap();
    assert_eq!(m.resolve_model_id("latest"), "gpt-4o-mini");
    assert_eq!(m.resolve_model_id("gpt-3.5"), "gpt-3.5");
}

#[test]
fn heartbeat_without_grace_rejected() {
    let toml = r#"
[provider]
id = "x"
family = "openai-compatible"
display_name = "X"
base_url = "https://api.x"
[auth]
api_key_env = "K"
[transport]
streaming = "sse"
heartbeat_event = "ping"
[tool_calls]
format = "openai_tool_calls"
    "#;
    let err = ProviderManifest::from_toml_str(toml).unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn quirks_round_trip_arbitrary_keys() {
    let toml = r#"
[provider]
id = "x"
family = "openai-compatible"
display_name = "X"
base_url = "https://api.x"
[auth]
api_key_env = "K"
[transport]
streaming = "sse"
[tool_calls]
format = "openai_tool_calls"
[quirks]
send-thinking-blocks = true
prompt-caching-breakpoints = ["system", "tools"]
max_output_tokens_default = 4096
    "#;
    let m = ProviderManifest::from_toml_str(toml).unwrap();
    assert_eq!(
        m.quirks.get("send-thinking-blocks"),
        Some(&toml::Value::Boolean(true))
    );
    let bps = m
        .quirks
        .get("prompt-caching-breakpoints")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(bps.len(), 2);
    assert_eq!(
        m.quirks.get("max_output_tokens_default"),
        Some(&toml::Value::Integer(4096))
    );
}

#[test]
fn quirks_default_to_empty_map() {
    let m = ProviderManifest::from_toml_str(minimal_toml()).unwrap();
    assert!(m.quirks.is_empty());
}

#[test]
fn hooks_default_when_table_absent() {
    let m = ProviderManifest::from_toml_str(minimal_toml()).unwrap();
    assert!(m.hooks.is_none());
}

#[test]
fn hooks_partial_override_keeps_default_for_other_field() {
    let toml = r#"
[provider]
id = "x"
family = "openai-compatible"
display_name = "X"
base_url = "https://api.x"
[auth]
api_key_env = "K"
[transport]
streaming = "sse"
[tool_calls]
format = "openai_tool_calls"
[hooks]
build_request = "anthropic-thinking-shim"
    "#;
    let m = ProviderManifest::from_toml_str(toml).unwrap();
    let h = m.hooks.unwrap();
    assert_eq!(h.build_request, "anthropic-thinking-shim");
    assert_eq!(h.parse_response, "default");
}

#[test]
fn hooks_both_explicit() {
    let toml = r#"
[provider]
id = "x"
family = "openai-compatible"
display_name = "X"
base_url = "https://api.x"
[auth]
api_key_env = "K"
[transport]
streaming = "sse"
[tool_calls]
format = "openai_tool_calls"
[hooks]
build_request = "h1"
parse_response = "h2"
    "#;
    let m = ProviderManifest::from_toml_str(toml).unwrap();
    let h = m.hooks.unwrap();
    assert_eq!(h.build_request, "h1");
    assert_eq!(h.parse_response, "h2");
}

#[test]
fn heartbeat_with_grace_accepted() {
    let toml = r#"
[provider]
id = "x"
family = "openai-compatible"
display_name = "X"
base_url = "https://api.x"
[auth]
api_key_env = "K"
[transport]
streaming = "sse"
heartbeat_event = "ping"
heartbeat_grace_ms = 30000
keepalive_chunks_reset = true
[tool_calls]
format = "openai_tool_calls"
    "#;
    let m = ProviderManifest::from_toml_str(toml).unwrap();
    assert_eq!(m.transport.heartbeat_event.as_deref(), Some("ping"));
    assert_eq!(m.transport.heartbeat_grace_ms, Some(30_000));
    assert!(m.transport.keepalive_chunks_reset);
}
