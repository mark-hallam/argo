//! Tests for the pure authoring parse + the degrade-fail-safe contract.

use super::parse_authored;

#[test]
fn parse_authored_extracts_embedded_json() {
    let raw = "here you go: {\"persona_description\":\"a sharp executive assistant who guards your calendar\",\
        \"responsibilities\":[\"triage inbound requests\",\"hold the calendar\"],\
        \"onboarding_script\":\"Hello — I'll keep your day clear.\"} (done)";
    let a = parse_authored(raw).expect("should parse the embedded JSON object");
    assert_eq!(a.persona_description, "a sharp executive assistant who guards your calendar");
    assert_eq!(a.responsibilities.len(), 2);
    assert!(!a.onboarding_script.is_empty());
}

#[test]
fn parse_authored_fails_safe_on_garbage_or_empty() {
    // No JSON object at all → None (caller keeps the templated spec).
    assert!(parse_authored("the model said no").is_none());
    // Empty persona description → None (never accept a hollow authoring).
    let hollow = "{\"persona_description\":\"   \",\"responsibilities\":[],\"onboarding_script\":\"\"}";
    assert!(parse_authored(hollow).is_none());
}
