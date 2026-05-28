//! Wire-shape spec for `POST /api/v1/pitch/ask`
//! (pitch-agent-rehearsal-ui-and-gateway).
//!
//! `argo-cli` is a bin crate with no exported library, so this file
//! cannot reach the gateway dispatcher directly (same constraint that
//! drives `genesis_route_e2e.rs`). Instead it pins the load-bearing
//! contract the route stands up — exercising the same
//! `argo-skill::CanonicalQaIndex` + `argo-capability::evaluate_claim`
//! pipeline the handler chains together — against the actual repo's
//! canonical-Q&A corpus + truth-boundary policies.
//!
//! Full inline coverage of the wire-format dispatcher (request
//! parsing, error envelopes, disposition mapping) lives in
//! `crates/argo-cli/src/handlers/gateway/routes_pitch_ask_tests.rs`.
//!
//! The four cases here mirror the brief's acceptance contract:
//!
//! 1. Canonical hit (architecture question) → Allow disposition.
//! 2. Canonical hit on a forbidden class (valuation) under the public
//!    pitch policy → Forbid disposition (renders as `"Block"` on the
//!    wire).
//! 3. Canonical miss → no match returned; the handler maps this to
//!    the Escalate-shaped fallback (operator sees the founder-handoff
//!    line).
//! 4. The same forbidden class under the private rehearsal policy on
//!    the Rehearsal channel → Allow (rehearsal lifts the public
//!    refusals so the operator can stress-test the answer).

use argo_capability::{
    evaluate_claim, load_policy, ChannelKind, ClaimCandidate, ClaimDisposition, TruthBoundaryPolicy,
};
use argo_skill::canonical_qa::CanonicalQaIndex;
use argo_types::TenantId;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn workspace_root() -> PathBuf {
    // `cargo test` runs with CWD = crate dir; walk up to the
    // workspace root (the closest ancestor whose `Cargo.toml` carries
    // a `[workspace]` table).
    let mut cursor: PathBuf = std::env::current_dir().expect("cwd");
    loop {
        let manifest = cursor.join("Cargo.toml");
        if manifest.is_file() {
            let raw = std::fs::read_to_string(&manifest).unwrap_or_default();
            if raw.contains("[workspace]") {
                return cursor;
            }
        }
        if !cursor.pop() {
            panic!("could not find workspace root from current dir");
        }
    }
}

fn corpus_dir(root: &Path) -> PathBuf {
    root.join("skill-library/canonical-qa/corpora/skycot")
}

fn public_policy(root: &Path) -> TruthBoundaryPolicy {
    load_policy(&root.join("policies/truth-boundary/skycot-public-pitch.toml"))
        .expect("load public pitch policy")
}

fn private_policy(root: &Path) -> TruthBoundaryPolicy {
    load_policy(&root.join("policies/truth-boundary/skycot-private-rehearsal.toml"))
        .expect("load private rehearsal policy")
}

fn load_index(root: &Path) -> CanonicalQaIndex {
    CanonicalQaIndex::load(&corpus_dir(root), TenantId(Uuid::nil())).expect("load corpus")
}

#[test]
fn canonical_hit_on_architecture_question_returns_allow_disposition() {
    let root = workspace_root();
    let idx = load_index(&root);
    let policy = public_policy(&root);
    let hit = idx
        .lookup("What's the tech stack?", "investor")
        .expect("expected a canonical hit for the tech-stack question");
    assert_eq!(hit.entry_id, "tech-stack");
    assert!(
        !hit.answer.is_empty(),
        "canonical answer must not be empty: entry_id={}",
        hit.entry_id
    );
    let evaluation = evaluate_claim(
        &policy,
        ChannelKind::WebEmbed,
        &ClaimCandidate {
            text: hit.answer.clone(),
            declared_classes: hit.declared_claim_classes.clone(),
        },
    );
    assert!(
        matches!(evaluation.disposition, ClaimDisposition::Allow),
        "architecture should pass truth-boundary on public channels; got {:?}",
        evaluation.disposition
    );
}

#[test]
fn canonical_hit_on_valuation_class_returns_forbid_under_public_policy() {
    let root = workspace_root();
    let idx = load_index(&root);
    let policy = public_policy(&root);
    let hit = idx
        .lookup("What's the valuation?", "investor")
        .expect("expected a canonical hit for the valuation question");
    let evaluation = evaluate_claim(
        &policy,
        ChannelKind::WebEmbed,
        &ClaimCandidate {
            text: hit.answer.clone(),
            declared_classes: hit.declared_claim_classes.clone(),
        },
    );
    assert!(
        matches!(evaluation.disposition, ClaimDisposition::Forbid { .. }),
        "valuation must be Forbid on a public channel; got {:?}",
        evaluation.disposition
    );
    assert!(
        evaluation.matched_class.is_some(),
        "Forbid disposition must carry a matched_class for the audit row"
    );
}

#[test]
fn unrecognized_question_returns_no_canonical_hit() {
    let root = workspace_root();
    let idx = load_index(&root);
    // A deliberately off-topic prompt that no corpus entry should
    // recover above the 0.45 similarity threshold.
    let hit = idx.lookup(
        "Please recite a haiku about the migratory patterns of arctic terns.",
        "investor",
    );
    assert!(
        hit.is_none(),
        "off-topic prompt unexpectedly matched: {:?}",
        hit.map(|h| h.entry_id)
    );
}

#[test]
fn valuation_class_is_allow_on_private_rehearsal_channel() {
    let root = workspace_root();
    let policy = private_policy(&root);
    let evaluation = evaluate_claim(
        &policy,
        ChannelKind::Rehearsal,
        &ClaimCandidate {
            text: "ARR is $4.2M".into(),
            declared_classes: vec!["valuation".into()],
        },
    );
    assert!(
        matches!(evaluation.disposition, ClaimDisposition::Allow),
        "Rehearsal channel must lift the valuation refusal so the operator can rehearse the answer; got {:?}",
        evaluation.disposition
    );
}
