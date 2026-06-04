//! Unit tests for `name_from_description` — the worker-name extractor
//! shared by `POST /api/v1/onboarding/first-hire` and its Genesis path.
//!
//! Regression anchor: UX-AUDIT-T22-20 — the prior first-capitalised-token
//! heuristic named every demo hire `A`, lifted from the wizard's own
//! example prompt "A marketing intern named Maya …". The explicit-pattern
//! list now wins so the operator's real name surfaces.

use super::name_from_description;

#[test]
fn explicit_named_pattern_wins_over_leading_article() {
    // The exact UX-AUDIT-T22-20 P0: the example copy from the wizard.
    assert_eq!(
        name_from_description("A marketing intern named Maya who writes social copy"),
        "Maya"
    );
}

#[test]
fn explicit_called_pattern() {
    assert_eq!(
        name_from_description("an assistant called Bob for triaging email"),
        "Bob"
    );
}

#[test]
fn explicit_name_is_pattern() {
    assert_eq!(
        name_from_description("Their name is Sarah and she handles invoices"),
        "Sarah"
    );
}

#[test]
fn explicit_i_am_pattern() {
    assert_eq!(name_from_description("Hi, I am Alex and I help with support"), "Alex");
}

#[test]
fn explicit_my_noun_is_pattern() {
    assert_eq!(
        name_from_description("my coordinator is Riley, please set them up"),
        "Riley"
    );
}

#[test]
fn article_a_is_never_returned_as_a_name() {
    // No explicit clause + no other capitalised token → must NOT be "A".
    assert_eq!(name_from_description("A friendly assistant for our team"), "Pioneer");
}

#[test]
fn lowercase_article_a_is_rejected_too() {
    assert_eq!(name_from_description("a helpful teammate for scheduling"), "Pioneer");
}

#[test]
fn explicit_pattern_skips_article_then_lands_on_real_name() {
    // "I am a … named Maya" — the `I am` clause captures the article `a`
    // first; the extractor must skip it and land on `Maya`.
    assert_eq!(
        name_from_description("I am a marketing intern named Maya"),
        "Maya"
    );
}

#[test]
fn capitalised_token_fallback_skips_leading_article() {
    // No explicit clause; first real capitalised token wins, not "The".
    assert_eq!(
        name_from_description("The teammate Quinn manages our calendar"),
        "Quinn"
    );
}

#[test]
fn lowercase_explicit_name_is_capitalised() {
    assert_eq!(name_from_description("we need someone named maya"), "Maya");
}

#[test]
fn empty_description_falls_back_to_pioneer() {
    assert_eq!(name_from_description("   "), "Pioneer");
}

#[test]
fn hyphenated_name_is_preserved() {
    assert_eq!(
        name_from_description("an assistant named Mary-Jane for outreach"),
        "Mary-Jane"
    );
}
