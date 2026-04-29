//! Property tests for the manifest deserializer.
//!
//! Generates synthetic providers and asserts:
//! - well-formed manifests round-trip through `from_toml_str`,
//! - the validator catches the bad-shape inputs we expect it to catch,
//! - capability set always includes the declared `api_key_env`,
//! - the `format` discriminator only accepts the six known wire formats.
//!
//! These tests close §8 acceptance "Adding a synthetic OpenAI-compat
//! provider via TOML alone works (no Rust changes)" — every shape produced
//! by the strategy below is a brand-new provider definition, and we only
//! ever speak to it through the manifest schema.
use argo_provider::{ProviderManifest, ToolCallFormat};
use argo_types::Capability;
use proptest::prelude::*;

#[derive(Debug, Clone)]
struct SyntheticProvider {
    id: String,
    display_name: String,
    base_url: String,
    api_key_env: String,
    streaming: &'static str,
    format: &'static str,
    connect_timeout_ms: u64,
    read_timeout_ms: u64,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
    heartbeat: bool,
    heartbeat_grace_ms: u64,
    egress_hosts: Vec<String>,
    models: Vec<(String, u32)>,
}

fn arb_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,15}".prop_map(String::from)
}

fn arb_env() -> impl Strategy<Value = String> {
    "[A-Z][A-Z0-9_]{0,20}".prop_map(String::from)
}

fn arb_host() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9.-]{2,30}".prop_map(String::from)
}

type IdentityTuple = (String, String, String, String, &'static str, &'static str);
type TimingTuple = (u64, u64, u64, u64, bool, u64);
type ListsTuple = (Vec<String>, Vec<(String, u32)>);

fn arb_identity() -> impl Strategy<Value = IdentityTuple> {
    (
        arb_id(),
        "[A-Za-z][A-Za-z0-9 ]{0,30}".prop_map(String::from),
        arb_host(),
        arb_env(),
        prop::sample::select(&["sse", "none"][..]),
        prop::sample::select(
            &[
                "openai_tool_calls",
                "openai_response_tools",
                "anthropic_tool_use",
                "bedrock_tool_use",
                "xml_tool_call",
                "llm_extracted",
            ][..],
        ),
    )
}

fn arb_timing() -> impl Strategy<Value = TimingTuple> {
    (
        100u64..30_000,
        1_000u64..600_000,
        50u64..2_000,
        2_000u64..30_000,
        any::<bool>(),
        1_000u64..120_000,
    )
}

fn arb_lists() -> impl Strategy<Value = ListsTuple> {
    (
        prop::collection::vec(arb_host(), 0..4),
        prop::collection::vec((arb_id(), 1024u32..2_000_000), 0..5),
    )
}

fn arb_provider() -> impl Strategy<Value = SyntheticProvider> {
    (arb_identity(), arb_timing(), arb_lists()).prop_map(
        |(
            (id, display_name, base_host, api_key_env, streaming, format),
            (
                connect_timeout_ms,
                read_timeout_ms,
                initial_backoff_ms,
                max_backoff_extra,
                heartbeat,
                heartbeat_grace_ms,
            ),
            (egress_hosts, models),
        )| {
            let max_backoff_ms = initial_backoff_ms + max_backoff_extra;
            let base_url = format!("https://{base_host}");
            SyntheticProvider {
                id,
                display_name,
                base_url,
                api_key_env,
                streaming,
                format,
                connect_timeout_ms,
                read_timeout_ms,
                initial_backoff_ms,
                max_backoff_ms,
                heartbeat,
                heartbeat_grace_ms,
                egress_hosts,
                models,
            }
        },
    )
}

fn render_toml(p: &SyntheticProvider) -> String {
    let mut out = String::new();
    out.push_str("[provider]\n");
    out.push_str(&format!("id = \"{}\"\n", p.id));
    out.push_str("family = \"openai-compatible\"\n");
    out.push_str(&format!("display_name = \"{}\"\n", p.display_name));
    out.push_str(&format!("base_url = \"{}\"\n", p.base_url));
    out.push_str("\n[auth]\n");
    out.push_str(&format!("api_key_env = \"{}\"\n", p.api_key_env));
    out.push_str("\n[transport]\n");
    out.push_str(&format!("streaming = \"{}\"\n", p.streaming));
    out.push_str(&format!("connect_timeout_ms = {}\n", p.connect_timeout_ms));
    out.push_str(&format!("read_timeout_ms = {}\n", p.read_timeout_ms));
    out.push_str(&format!("initial_backoff_ms = {}\n", p.initial_backoff_ms));
    out.push_str(&format!("max_backoff_ms = {}\n", p.max_backoff_ms));
    if p.heartbeat && p.streaming == "sse" {
        out.push_str("heartbeat_event = \"ping\"\n");
        out.push_str(&format!("heartbeat_grace_ms = {}\n", p.heartbeat_grace_ms));
    }
    out.push_str("\n[tool_calls]\n");
    out.push_str(&format!("format = \"{}\"\n", p.format));
    if !p.egress_hosts.is_empty() {
        out.push_str("\n[capabilities]\n");
        let listed = p
            .egress_hosts
            .iter()
            .map(|h| format!("\"{h}\""))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("net_egress = [{listed}]\n"));
    }
    for (id, ctx) in &p.models {
        out.push_str("\n[[models]]\n");
        out.push_str(&format!("id = \"{id}\"\n"));
        out.push_str(&format!("context_window = {ctx}\n"));
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn synthetic_provider_round_trips(p in arb_provider()) {
        let toml = render_toml(&p);
        let parsed = ProviderManifest::from_toml_str(&toml)
            .map_err(|e| {
                TestCaseError::fail(format!("parse failed: {e:?}\n--- toml ---\n{toml}"))
            })?;
        prop_assert_eq!(&parsed.provider.id, &p.id);
        prop_assert_eq!(&parsed.provider.base_url, &p.base_url);
        prop_assert_eq!(&parsed.auth.api_key_env, &p.api_key_env);
        prop_assert_eq!(parsed.models.len(), p.models.len());
    }

    #[test]
    fn capabilities_always_include_api_key_env(p in arb_provider()) {
        let toml = render_toml(&p);
        let parsed = ProviderManifest::from_toml_str(&toml)
            .map_err(|e| TestCaseError::fail(format!("parse failed: {e:?}")))?;
        let set = parsed.capabilities.to_set(&parsed.auth.api_key_env);
        let want = Capability::EnvRead {
            var_glob: parsed.auth.api_key_env.clone(),
        };
        prop_assert!(set.covers(&want));
    }

    #[test]
    fn format_discriminator_is_strict(format in "[a-z_]{1,30}") {
        let known = matches!(
            format.as_str(),
            "openai_tool_calls"
                | "openai_response_tools"
                | "anthropic_tool_use"
                | "bedrock_tool_use"
                | "xml_tool_call"
                | "llm_extracted"
        );
        let parsed: Result<ToolCallFormat, _> =
            serde_json::from_value(serde_json::json!(format));
        prop_assert_eq!(parsed.is_ok(), known);
    }

    #[test]
    fn validator_rejects_inverted_backoff(
        initial in 1_000u64..5_000,
        delta in 1u64..1_000,
    ) {
        let max = initial - delta;
        let toml = format!(r#"
[provider]
id = "x"
family = "openai-compatible"
display_name = "X"
base_url = "https://api.x"
[auth]
api_key_env = "K"
[transport]
streaming = "none"
initial_backoff_ms = {initial}
max_backoff_ms = {max}
[tool_calls]
format = "openai_tool_calls"
"#);
        let res = ProviderManifest::from_toml_str(&toml);
        prop_assert!(matches!(res, Err(argo_types::ArgoError::Schema(_))));
    }
}
