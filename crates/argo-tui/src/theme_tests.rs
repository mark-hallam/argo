//! Unit tests for the theme module.
use super::*;
use ratatui::style::Color;
use std::collections::BTreeMap;

#[test]
fn presets_construct_without_panic() {
    // Each built-in preset must build and carry a non-empty name.
    for theme in [Theme::dark(), Theme::light(), Theme::solarized(), Theme::dracula()] {
        assert!(!theme.name.is_empty(), "preset name must be non-empty");
    }
}

#[test]
fn lookup_unknown_falls_back_to_dark() {
    // Typos must not abort startup. The TUI gets the dark preset
    // and the warning log surfaces the bad name to the operator.
    let t = Theme::lookup("nonexistent-palette");
    assert_eq!(t.name, "dark");
    assert_eq!(t, Theme::dark());
}

#[test]
fn lookup_dracula_returns_dracula() {
    let t = Theme::lookup("dracula");
    assert_eq!(t.name, "dracula");
    assert_eq!(t, Theme::dracula());
}

#[test]
fn overrides_apply_to_named_slots() {
    // Override one slot per call; remaining slots stay on the base.
    let mut overrides = BTreeMap::new();
    overrides.insert("accent".to_string(), "#ff8800".to_string());
    overrides.insert("error".to_string(), "112233".to_string());
    let t = Theme::dark().with_overrides(&overrides);
    assert_eq!(t.accent, Color::Rgb(0xff, 0x88, 0x00));
    assert_eq!(t.error, Color::Rgb(0x11, 0x22, 0x33));
    // Unrelated slot stays on the base.
    assert_eq!(t.success, Theme::dark().success);
}

#[test]
fn hex_parse_accepts_valid_and_rejects_invalid() {
    // Accept with and without `#`.
    assert_eq!(parse_hex("#aabbcc"), Some(Color::Rgb(0xaa, 0xbb, 0xcc)));
    assert_eq!(parse_hex("aabbcc"), Some(Color::Rgb(0xaa, 0xbb, 0xcc)));
    // Reject anything not exactly 6 hex digits.
    assert_eq!(parse_hex(""), None);
    assert_eq!(parse_hex("#abc"), None);
    assert_eq!(parse_hex("#aabbccdd"), None);
    assert_eq!(parse_hex("#zzzzzz"), None);
    assert_eq!(parse_hex("not-a-color"), None);
}

#[test]
fn invalid_hex_overrides_fall_back_to_base() {
    // The brief: "Invalid hex in overrides logs warning + falls
    // back; no panic." Bad value leaves the base color in place.
    let mut overrides = BTreeMap::new();
    overrides.insert("accent".to_string(), "not-a-real-color".to_string());
    overrides.insert("unknown_slot".to_string(), "#ffffff".to_string());
    let t = Theme::dark().with_overrides(&overrides);
    assert_eq!(t.accent, Theme::dark().accent);
    // `from_config` with empty/None name also stays on dark.
    let same = from_config(None, &BTreeMap::new());
    assert_eq!(same, Theme::dark());
    let by_name = from_config(Some("dracula"), &BTreeMap::new());
    assert_eq!(by_name, Theme::dracula());
}
