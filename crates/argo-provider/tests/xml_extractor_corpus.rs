//! Integration test: `xml_tool_call` against the Hermes fixture corpus.
//!
//! Each fixture is a plain-text `assistant.content` body that we wrap in
//! the OpenAI Chat Completions response shape (`choices[].message.content`)
//! and feed through the extractor. Eight fixtures spanning the cases the
//! Phase 2 brief calls out: single, multi, malformed, mixed-content,
//! empty args, and friends.
//!
//! Spec: `docs/plan/argo-build-plan.md` §8 acceptance.
use argo_provider::xml_tool_call;

fn body_with_content(text: &str) -> serde_json::Value {
    serde_json::json!({
        "choices": [{ "message": { "content": text } }]
    })
}

fn fixture(name: &str) -> String {
    let path = format!("tests/fixtures/hermes/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read fixture {path}: {e}"))
}

#[test]
fn single_call() {
    let body = body_with_content(&fixture("01_single_call.txt"));
    let calls = xml_tool_call(&body).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "ls");
    assert_eq!(calls[0].arguments["path"], "/");
}

#[test]
fn multiple_calls_in_order() {
    let body = body_with_content(&fixture("02_multiple_calls.txt"));
    let calls = xml_tool_call(&body).unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "ls");
    assert_eq!(calls[0].arguments["path"], "/etc");
    assert_eq!(calls[1].name, "file_read");
    assert_eq!(calls[1].arguments["path"], "/etc/hosts");
}

#[test]
fn empty_args_object() {
    let body = body_with_content(&fixture("03_empty_args.txt"));
    let calls = xml_tool_call(&body).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "noop");
    assert!(calls[0].arguments.is_object());
    assert_eq!(calls[0].arguments.as_object().unwrap().len(), 0);
}

#[test]
fn inline_with_surrounding_prose() {
    let body = body_with_content(&fixture("04_inline_in_prose.txt"));
    let calls = xml_tool_call(&body).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[0].arguments["path"], "Cargo.toml");
}

#[test]
fn nested_object_args_round_trip() {
    let body = body_with_content(&fixture("05_nested_object_args.txt"));
    let calls = xml_tool_call(&body).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].arguments["query"], "argo");
    assert_eq!(calls[0].arguments["filters"]["language"], "rust");
    assert_eq!(calls[0].arguments["filters"]["limit"], 10);
}

#[test]
fn unicode_args_preserved() {
    let body = body_with_content(&fixture("06_unicode_args.txt"));
    let calls = xml_tool_call(&body).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].arguments["text"], "héllo wörld 日本語");
}

#[test]
fn no_tool_calls_yields_empty() {
    let body = body_with_content(&fixture("07_no_tool_calls.txt"));
    let calls = xml_tool_call(&body).unwrap();
    assert!(calls.is_empty());
}

#[test]
fn unterminated_tag_yields_no_calls() {
    // The opening tag is never closed; we should not emit a partial call.
    let body = body_with_content(&fixture("08_unterminated_tag.txt"));
    let calls = xml_tool_call(&body).unwrap();
    assert!(calls.is_empty());
}

#[test]
fn malformed_json_inside_block_is_provider_error() {
    let body = body_with_content(&fixture("09_malformed_json.txt"));
    let err = xml_tool_call(&body).unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::Provider(_)));
}
