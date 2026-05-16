use super::*;
use serde_json::json;

#[test]
fn redacts_secret_named_string_fields_only() {
    let mut value = json!({
        "siem": { "token": "splunk-secret" },
        "budget": { "default_tokens_in": 100 },
        "sso": { "token_endpoint": "https://idp.example/token" }
    });
    redact_json(&mut value, false);
    assert_eq!(value["siem"]["token"], REDACTED);
    assert_eq!(value["budget"]["default_tokens_in"], 100);
    assert_eq!(value["sso"]["token_endpoint"], "https://idp.example/token");
}

#[test]
fn dotted_lookup_handles_literal_dot_keys() {
    let value = json!({
        "gateway": {
            "rate_limit": {
                "route": {
                    "overrides": {
                        "forge.run": 10
                    }
                }
            }
        }
    });
    let found = lookup_json(&value, "gateway.rate_limit.route.overrides.forge.run").unwrap();
    assert_eq!(found, &json!(10));
}

#[test]
fn gateway_bind_key_is_derived_from_host_and_port() {
    let value = json!({ "gateway": { "host": "127.0.0.1", "port": 8090 } });
    let found = lookup_effective_key(&value, "gateway.bind").unwrap();
    assert_eq!(found, json!("127.0.0.1:8090"));
}

#[test]
fn source_annotation_adds_comments_to_values() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("argo.toml"), "[custom]\nvalue = 7777\n").unwrap();
    let sources = ConfigSourceMap::detect_from(dir.path());
    let rendered =
        annotate_toml_sources("[custom]\nvalue = 7777\nhost = \"127.0.0.1\"\n", &sources);
    assert!(rendered.contains("value = 7777 # source: project config"));
}

#[test]
fn key_value_text_renders_scalar_without_json_object_wrapper() {
    assert_eq!(render_key_value(&json!("127.0.0.1")).unwrap(), "127.0.0.1");
    assert_eq!(render_key_value(&json!(8090)).unwrap(), "8090");
}
