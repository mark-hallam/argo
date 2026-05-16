//! Unit tests for [`crate::Redacted`].

use crate::Redacted;

#[test]
fn debug_never_prints_inner() {
    let r = Redacted::new(String::from("hunter2-very-secret"));
    let out = format!("{r:?}");
    assert!(!out.contains("hunter2"), "Debug leaked the inner: {out}");
    assert_eq!(out, "Redacted(\"[redacted]\")");
}

#[test]
fn display_prints_redacted_marker() {
    let r = Redacted::new(String::from("hunter2-very-secret"));
    let out = format!("{r}");
    assert!(!out.contains("hunter2"));
    assert_eq!(out, "[redacted]");
}

#[test]
fn serialize_emits_redacted_literal() {
    let r = Redacted::new(String::from("abc123"));
    assert_eq!(serde_json::to_string(&r).unwrap(), "\"[redacted]\"");
}

#[test]
fn deserialize_round_trips_real_value() {
    let r: Redacted<String> = serde_json::from_str(r#""abc123""#).unwrap();
    assert_eq!(r.expose(), "abc123");
    assert_eq!(serde_json::to_string(&r).unwrap(), "\"[redacted]\"");
}

#[test]
fn expose_and_into_inner_return_real_value() {
    let r = Redacted::new(String::from("payload"));
    assert_eq!(r.expose(), "payload");
    assert_eq!(r.clone().into_inner(), String::from("payload"));
}

#[test]
fn eq_constant_time_compare() {
    let a = Redacted::new(String::from("same"));
    assert_eq!(a, Redacted::new(String::from("same")));
    assert_ne!(a, Redacted::new(String::from("diff")));
    assert_ne!(a, Redacted::new(String::from("a-much-longer-secret")));
}

#[test]
fn default_is_default_inner() {
    let r: Redacted<String> = Redacted::default();
    assert_eq!(r.expose(), "");
}
