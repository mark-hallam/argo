//! Unit tests for `persona_edit.rs`. Brought in via
//! `#[cfg(test)] #[path = "persona_edit_tests.rs"] mod tests;`
//! so the parent file stays lean.

use super::*;
use argo_identity::{Persona, VoiceProfile};

fn sample_persona() -> Persona {
    Persona {
        name: "Maya".into(),
        voice_profile: VoiceProfile {
            preamble: "Be concise and direct.".into(),
        },
        avatar: Some("https://example.com/maya.png".into()),
        bio: Some("Marketplace SDR.".into()),
        builder_attribution: "operator@example.com".into(),
        provenance_chain: vec!["seed".into()],
    }
}

#[test]
fn dump_then_parse_round_trips_editable_subset() {
    let persona = sample_persona();
    let dumped = dump_toml(&persona);
    let parsed = parse_toml(&dumped).expect("dumped TOML must parse");

    assert_eq!(parsed.name, persona.name);
    assert_eq!(parsed.voice, persona.voice_profile.preamble);
    assert_eq!(parsed.avatar, persona.avatar);
    assert_eq!(parsed.bio, persona.bio);
}

#[test]
fn dumped_toml_starts_with_persona_block() {
    let persona = sample_persona();
    let dumped = dump_toml(&persona);
    assert!(
        dumped.contains("[persona]"),
        "expected `[persona]` block; got {dumped:?}"
    );
    assert!(dumped.contains("name = \"Maya\""), "name line missing: {dumped}");
}

#[test]
fn parse_rejects_oversized_voice() {
    let mut persona = sample_persona();
    persona.voice_profile.preamble = "x".repeat(MAX_VOICE_BYTES + 1);
    let toml = dump_toml(&persona);
    let err = parse_toml(&toml).expect_err("oversize voice must fail");
    let msg = format!("{err}");
    assert!(msg.contains("voice exceeds"), "wrong error: {msg}");
}

#[test]
fn parse_rejects_oversized_bio() {
    let mut persona = sample_persona();
    persona.bio = Some("y".repeat(MAX_BIO_BYTES + 1));
    let toml = dump_toml(&persona);
    let err = parse_toml(&toml).expect_err("oversize bio must fail");
    let msg = format!("{err}");
    assert!(msg.contains("bio exceeds"), "wrong error: {msg}");
}

#[test]
fn parse_rejects_invalid_name() {
    let toml = r#"[persona]
name = "1starts-with-digit"
voice = "ok"
"#;
    let err = parse_toml(toml).expect_err("invalid name must fail");
    let msg = format!("{err}");
    assert!(msg.contains("not valid"), "wrong error: {msg}");
}

#[test]
fn parse_rejects_unknown_field() {
    let toml = r#"[persona]
name = "Maya"
voice = "ok"
secret_admin_flag = true
"#;
    let err = parse_toml(toml).expect_err("unknown_field must fail");
    let msg = format!("{err}");
    assert!(msg.contains("invalid persona TOML"), "wrong error: {msg}");
}

#[test]
fn parse_rejects_malformed_toml() {
    let err = parse_toml("not = [valid] toml @ all").expect_err("garbage must fail");
    assert!(format!("{err}").contains("invalid persona TOML"));
}

#[test]
fn name_validator_accepts_typical_names() {
    for ok in [
        "Maya",
        "Riley Q",
        "ari-2",
        "M",
        &"a".repeat(64),
        "Long Name With Spaces",
    ] {
        assert!(is_valid_persona_name(ok), "should accept: {ok:?}");
    }
}

#[test]
fn name_validator_rejects_bad_names() {
    for bad in [
        "",
        " leading-space",
        "trailing-space ",
        "1leading-digit",
        "_underscore-first",
        "has@symbol",
        &"x".repeat(65),
    ] {
        assert!(!is_valid_persona_name(bad), "should reject: {bad:?}");
    }
}

#[test]
fn resolve_editor_prefers_argo_editor() {
    // SAFETY: env var manipulation in a unit test is process-wide;
    // tests that touch ARGO_EDITOR / VISUAL / EDITOR can race
    // when run with `--test-threads >1`. We isolate by snapshot+
    // restore + serial probe inside one test.
    let snapshot = (
        std::env::var("ARGO_EDITOR").ok(),
        std::env::var("VISUAL").ok(),
        std::env::var("EDITOR").ok(),
    );

    // Clear all three.
    for v in ["ARGO_EDITOR", "VISUAL", "EDITOR"] {
        // SAFETY: see comment above.
        unsafe { std::env::remove_var(v) };
    }
    assert_eq!(resolve_editor(), "vi");

    unsafe { std::env::set_var("EDITOR", "ed") };
    assert_eq!(resolve_editor(), "ed");

    unsafe { std::env::set_var("VISUAL", "code") };
    assert_eq!(resolve_editor(), "code");

    unsafe { std::env::set_var("ARGO_EDITOR", "nano") };
    assert_eq!(resolve_editor(), "nano");

    // Restore.
    restore_env("ARGO_EDITOR", snapshot.0.as_deref());
    restore_env("VISUAL", snapshot.1.as_deref());
    restore_env("EDITOR", snapshot.2.as_deref());
}

fn restore_env(key: &str, value: Option<&str>) {
    // SAFETY: see resolve_editor_prefers_argo_editor.
    unsafe {
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }
}

#[test]
fn read_in_place_missing_file_errors_with_path() {
    let path = std::path::Path::new("/tmp/argo-persona-edit-does-not-exist-xyzzy.toml");
    let err = read_in_place(path).expect_err("missing file must fail");
    let msg = format!("{err}");
    assert!(msg.contains("argo-persona-edit-does-not-exist-xyzzy.toml"), "{msg}");
}

#[test]
fn read_in_place_happy_path_returns_body() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("persona.toml");
    std::fs::write(
        &path,
        "[persona]\nname = \"Sage\"\nvoice = \"Be kind.\"\n",
    )
    .expect("write");
    let body = read_in_place(&path).expect("parse");
    assert_eq!(body.name, "Sage");
    assert_eq!(body.voice, "Be kind.");
    assert!(body.bio.is_none());
    assert!(body.avatar.is_none());
}
