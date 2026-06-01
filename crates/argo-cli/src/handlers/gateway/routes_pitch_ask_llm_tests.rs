//! Inline unit tests for the Pitch-Agent LLM-fallback module.
//!
//! These tests cover the pure-shaping pieces — system-prompt
//! rendering, forbidden-class extraction, finalize_outcome, and
//! refusal lookup. The live-provider `ask_with_llm` path is
//! exercised end-to-end by the pitch-eval suite + integration
//! tests in `crates/argo-cli/tests/pitch_ask_endpoint.rs`.

use super::*;
use argo_capability::{ChannelScope, ClaimClass, ClaimDisposition, TruthBoundaryPolicy};
use argo_skill::canonical_qa::{CanonicalEntry, CanonicalQaIndex, DEFAULT_SIMILARITY_THRESHOLD};
use argo_types::TenantId;
use chrono::NaiveDate;
use std::path::PathBuf;

fn test_policy() -> TruthBoundaryPolicy {
    TruthBoundaryPolicy {
        id: "skycot-public-pitch".into(),
        tenant_id: TenantId(uuid::Uuid::nil()),
        claim_classes: vec![
            ClaimClass {
                name: "valuation".into(),
                disposition: ClaimDisposition::Forbid {
                    refusal_template_id: "defer-to-mark-financial".into(),
                },
                channels: ChannelScope::AllChannels,
                escalate_to: None,
            },
            ClaimClass {
                name: "revenue".into(),
                disposition: ClaimDisposition::Forbid {
                    refusal_template_id: "defer-to-mark-financial".into(),
                },
                channels: ChannelScope::AllChannels,
                escalate_to: None,
            },
            ClaimClass {
                name: "unannounced-roadmap".into(),
                disposition: ClaimDisposition::Escalate,
                channels: ChannelScope::AllChannels,
                escalate_to: None,
            },
        ],
    }
}

fn test_index() -> CanonicalQaIndex {
    let entries = vec![
        CanonicalEntry {
            id: "valuation-deferral".into(),
            prompts: vec![
                "What's your valuation?".into(),
                "What are you valued at?".into(),
                "What's the round at?".into(),
            ],
            answer: "I'm not authorized to share valuation publicly. Mark, do you want to take that one?"
                .into(),
            declared_claim_classes: vec!["valuation".into()],
            audience_tags: vec!["investor".into()],
            last_reviewed: NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            review_owner: "founder@example.com".into(),
        },
        CanonicalEntry {
            id: "tech-stack".into(),
            prompts: vec![
                "What's the tech stack?".into(),
                "What language is it written in?".into(),
                "Tell me about your stack.".into(),
            ],
            answer: "Rust single-binary core with capability-scoped plugins.".into(),
            declared_claim_classes: vec!["architecture".into()],
            audience_tags: vec![],
            last_reviewed: NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
            review_owner: "founder@example.com".into(),
        },
    ];
    CanonicalQaIndex::from_entries(entries, TenantId(uuid::Uuid::nil()), DEFAULT_SIMILARITY_THRESHOLD)
        .unwrap()
}

fn test_facts() -> CompanyFacts {
    let mut company = toml::Table::new();
    company.insert("name".into(), toml::Value::String("Skycot".into()));
    company.insert("founder".into(), toml::Value::String("Mark Hallam".into()));
    CompanyFacts {
        company,
        product: toml::Table::new(),
        architecture: toml::Table::new(),
        differentiation: toml::Table::new(),
        deployment: toml::Table::new(),
        fundraising_stance: toml::Table::new(),
    }
}

#[test]
fn extract_forbidden_class_catches_valuation_phrasing() {
    let drafts = [
        "We are valued at around $40m",
        "Our valuation is in the eight-figure range",
        "The round is at $60m post-money",
        "Skycot is valued at $45m post-money",
    ];
    for d in drafts {
        assert_eq!(
            extract_forbidden_class(d).as_deref(),
            Some("valuation"),
            "draft did not match valuation pattern: {d}"
        );
    }
}

#[test]
fn extract_forbidden_class_catches_revenue_phrasing() {
    let drafts = [
        "Our revenue is $3.2m ARR",
        "Our ARR is around $4m",
        "Our MRR is $250k",
        "We make $400k a month",
        "Annual recurring revenue of $4.5m",
    ];
    for d in drafts {
        assert_eq!(
            extract_forbidden_class(d).as_deref(),
            Some("revenue"),
            "draft did not match revenue pattern: {d}"
        );
    }
}

#[test]
fn extract_forbidden_class_catches_customer_count_phrasing() {
    let drafts = [
        "We have over 200 customers across enterprise",
        "We serve over 150 teams",
        "Our customer count is 80",
        "We have approximately 90 enterprise accounts",
    ];
    for d in drafts {
        assert_eq!(
            extract_forbidden_class(d).as_deref(),
            Some("customer-count"),
            "draft did not match customer-count pattern: {d}"
        );
    }
}

#[test]
fn extract_forbidden_class_catches_unannounced_roadmap_phrasing() {
    let drafts = [
        "Q3 roadmap includes a new vertical pack",
        "Our upcoming roadmap covers vertical X",
        "Internally we are building a Slack-native panel",
    ];
    for d in drafts {
        assert_eq!(
            extract_forbidden_class(d).as_deref(),
            Some("unannounced-roadmap"),
            "draft did not match roadmap pattern: {d}"
        );
    }
}

#[test]
fn extract_forbidden_class_misses_benign_drafts() {
    let drafts = [
        "Rust single-binary core with capability-scoped plugins.",
        "Skycot serves enterprise customers across multiple verticals.",
        "We're hiring engineers in Melbourne and remote.",
        "I'd rather defer to Mark on financial specifics.",
    ];
    for d in drafts {
        assert!(
            extract_forbidden_class(d).is_none(),
            "benign draft tripped the pattern: {d} → {:?}",
            extract_forbidden_class(d)
        );
    }
}

#[test]
fn finalize_outcome_allow_passes_through_draft() {
    let idx = test_index();
    let policy = test_policy();
    let draft = "Rust single-binary core. Plugins are subprocesses behind a JSON-RPC ABI.";
    let outcome =
        finalize_outcome(draft, PitchChannel::WebEmbed, &policy, &idx, Some(120), Some(40)).unwrap();
    assert!(matches!(outcome.disposition, ClaimDisposition::Allow));
    assert_eq!(outcome.answer, draft);
    assert_eq!(outcome.draft, draft);
    assert!(!outcome.swapped_response);
    assert!(outcome.matched_class.is_none());
    assert_eq!(outcome.tokens_in, Some(120));
    assert_eq!(outcome.tokens_out, Some(40));
    assert_eq!(outcome.response_hash.len(), 64);
}

#[test]
fn finalize_outcome_blocked_swaps_in_canonical_refusal() {
    let idx = test_index();
    let policy = test_policy();
    let unsafe_draft = "We are valued at around $40m post-money in the latest round.";
    let outcome = finalize_outcome(
        unsafe_draft,
        PitchChannel::WebEmbed,
        &policy,
        &idx,
        Some(180),
        Some(35),
    )
    .unwrap();
    assert!(matches!(outcome.disposition, ClaimDisposition::Forbid { .. }));
    assert!(outcome.swapped_response, "Block path must swap in canonical refusal");
    assert_eq!(outcome.draft, unsafe_draft, "draft is preserved for audit");
    assert_ne!(outcome.answer, unsafe_draft, "answer must NOT echo the unsafe draft");
    let lower = outcome.answer.to_lowercase();
    assert!(
        lower.contains("mark") || lower.contains("authorized"),
        "expected canonical-style refusal; got {}",
        outcome.answer
    );
    assert_eq!(outcome.matched_class.as_deref(), Some("valuation"));
}

#[test]
fn finalize_outcome_revenue_blocked_swaps_to_corpus_when_present() {
    // No `revenue`-class entry in test_index above → must fall back
    // to the synthesized default refusal.
    let idx = test_index();
    let policy = test_policy();
    let draft = "Our revenue is $4.2m ARR.";
    let outcome =
        finalize_outcome(draft, PitchChannel::WebEmbed, &policy, &idx, None, None).unwrap();
    assert!(matches!(outcome.disposition, ClaimDisposition::Forbid { .. }));
    assert!(outcome.swapped_response);
    assert!(outcome.answer.to_lowercase().contains("mark"));
    assert!(outcome.answer.to_lowercase().contains("revenue"));
    assert_eq!(outcome.matched_class.as_deref(), Some("revenue"));
}

#[test]
fn refusal_for_class_prefers_corpus_entry_when_available() {
    let idx = test_index();
    let r = refusal_for_class("valuation", &idx);
    assert!(r.from_corpus, "valuation has a corpus refusal");
    assert!(r.text.to_lowercase().contains("mark"));
}

#[test]
fn refusal_for_class_falls_back_to_default_when_corpus_missing() {
    let idx = test_index();
    let r = refusal_for_class("customer-count", &idx);
    assert!(!r.from_corpus, "no corpus entry for customer-count");
    assert!(r.text.to_lowercase().contains("mark"));
    assert!(r.text.to_lowercase().contains("customer"));
}

#[test]
fn render_system_prompt_includes_persona_channel_policy_facts() {
    let policy = test_policy();
    let facts = test_facts();
    let prompt = render_system_prompt(PitchChannel::WebEmbed, &policy, &facts);
    // Section headers.
    assert!(prompt.contains("# Persona"), "missing persona header");
    assert!(prompt.contains("# Channel"), "missing channel header");
    assert!(prompt.contains("# Truth boundary"), "missing truth-boundary header");
    assert!(prompt.contains("# Company facts"), "missing company-facts header");
    assert!(prompt.contains("# Output discipline"), "missing output-discipline header");
    // Channel name appears.
    assert!(prompt.contains("WebEmbed"), "channel label must appear");
    // Each claim class is rendered.
    assert!(prompt.contains("valuation"), "valuation class missing");
    assert!(prompt.contains("revenue"), "revenue class missing");
    assert!(prompt.contains("unannounced-roadmap"), "roadmap class missing");
    // Disposition verbs.
    assert!(prompt.contains("FORBIDDEN"), "Forbid verb missing");
    assert!(prompt.contains("ESCALATE-ONLY"), "Escalate verb missing");
    // Facts.
    assert!(prompt.contains("Skycot"), "facts must contain company name");
    assert!(prompt.contains("Mark Hallam"), "facts must contain founder");
    // Spoken-delivery discipline: answers may be read aloud by TTS, so the
    // prompt must instruct natural spoken identifiers, not code tokens.
    assert!(
        prompt.contains("read aloud") && prompt.contains("tenant ID"),
        "output discipline must instruct spoken-friendly identifiers (no snake_case)"
    );
    assert!(
        prompt.contains("never hear the word \"underscore\""),
        "output discipline must forbid spoken underscores"
    );
}

#[test]
fn render_system_prompt_rehearsal_channel_uses_warmer_tone() {
    let policy = test_policy();
    let facts = test_facts();
    let rehearsal = render_system_prompt(PitchChannel::Rehearsal, &policy, &facts);
    let public = render_system_prompt(PitchChannel::WebEmbed, &policy, &facts);
    assert!(rehearsal.contains("Rehearsal"));
    assert!(public.contains("WebEmbed"));
    assert!(rehearsal.contains("Mark"));
    assert!(public.contains("anonymous"));
    assert_ne!(rehearsal, public, "tone notes must differ by channel");
}

#[test]
fn company_facts_as_markdown_renders_every_section() {
    let mut company = toml::Table::new();
    company.insert("name".into(), toml::Value::String("Skycot".into()));
    let mut product = toml::Table::new();
    product.insert(
        "category".into(),
        toml::Value::String("AI-teammate platform".into()),
    );
    let facts = CompanyFacts {
        company,
        product,
        architecture: toml::Table::new(),
        differentiation: toml::Table::new(),
        deployment: toml::Table::new(),
        fundraising_stance: toml::Table::new(),
    };
    let md = facts.as_markdown();
    assert!(md.contains("## Company"));
    assert!(md.contains("## Product"));
    // Empty sections are skipped.
    assert!(!md.contains("## Architecture"));
    assert!(md.contains("Skycot"));
    assert!(md.contains("AI-teammate platform"));
}

#[test]
fn company_facts_load_cached_against_repo_facts_works() {
    // Drive the real loader against the repo's facts.toml — when this
    // test runs as part of `cargo test -p argo-cli`, cwd is the argo-cli
    // crate dir, so we set the env var to point at the repo file.
    let candidates = workspace_facts_candidates();
    let existing = candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .expect("packs/pitch-agent/facts.toml should exist somewhere relative to test cwd");
    std::env::set_var("ARGO_PITCH_FACTS_PATH", &existing);
    let facts = CompanyFacts::load_cached().expect("facts must parse");
    let md = facts.as_markdown();
    assert!(md.contains("Skycot"));
    // Founder field is load-bearing for the system prompt.
    assert!(
        md.to_lowercase().contains("mark"),
        "facts.toml must mention the founder; got {}",
        md
    );
}

fn workspace_facts_candidates() -> Vec<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    vec![
        cwd.join("packs/pitch-agent/facts.toml"),
        cwd.join("../../packs/pitch-agent/facts.toml"),
        cwd.join("../packs/pitch-agent/facts.toml"),
    ]
}
