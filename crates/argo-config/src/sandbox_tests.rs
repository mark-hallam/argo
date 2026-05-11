//! Unit tests for `[sandbox.image]` parsing.
use crate::ArgoConfig;

#[test]
fn missing_table_is_empty() {
    let cfg: ArgoConfig = toml::from_str("").expect("empty parses");
    assert!(cfg.sandbox.image.is_empty(), "missing table → empty map");
    assert_eq!(cfg.sandbox.image_for_executor("browser"), None);
}

#[test]
fn parse_multi_image() {
    let src = r#"
[sandbox.image]
marketplace_browser = "skycot/marketplace-browser:latest"
code_runner = "skycot/code-runner:0.3"
"#;
    let cfg: ArgoConfig = toml::from_str(src).expect("parses");
    assert_eq!(cfg.sandbox.image.len(), 2);
    assert_eq!(
        cfg.sandbox.image_for_executor("marketplace_browser"),
        Some("skycot/marketplace-browser:latest"),
    );
    assert_eq!(
        cfg.sandbox.image_for_executor("code_runner"),
        Some("skycot/code-runner:0.3"),
    );
    assert_eq!(cfg.sandbox.image_for_executor("nonexistent"), None);
}

#[test]
fn reject_non_string_value() {
    let src = r#"
[sandbox.image]
marketplace_browser = 42
"#;
    let err = toml::from_str::<ArgoConfig>(src).expect_err("non-string must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("string") || msg.contains("integer"),
        "expected type-mismatch message, got: {msg}",
    );
}
