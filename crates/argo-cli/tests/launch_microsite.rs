//! CI locks for `demo/launch-gtm-engineer/` — the launch-artifact
//! microsite. Three small file-level checks that catch the most
//! common kinds of breakage:
//!
//!   1. data/demo.json shape (page renders correctly only if every
//!      section's required keys are present)
//!   2. index.html references the three assets it depends on
//!      (style.css, app.js, data/demo.json)
//!   3. README.md mentions both deterministic and live modes (so a
//!      user pointed at the README knows the page has both paths)
//!
//! No browser automation, no JS engine — just file I/O + serde +
//! string contains. Always runnable, no env gates.

mod common;

use serde_json::Value;
use std::path::{Path, PathBuf};

fn microsite_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("demo/launch-gtm-engineer")
}

fn read_file(rel: &str) -> String {
    let path = microsite_dir().join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {path:?}: {e}"))
}

fn pluck<'a>(v: &'a Value, dotted: &str) -> &'a Value {
    let mut cur = v;
    for k in dotted.split('.') {
        cur = cur.get(k).unwrap_or_else(|| {
            panic!("missing key path {dotted:?} in demo.json (stuck at segment {k:?})")
        });
    }
    cur
}

fn require_non_empty_string(v: &Value, dotted: &str) {
    let node = pluck(v, dotted);
    let s = node.as_str().unwrap_or_else(|| {
        panic!("{dotted:?} must be a string; got: {node}")
    });
    assert!(!s.trim().is_empty(), "{dotted:?} must not be empty");
}

fn require_number(v: &Value, dotted: &str) {
    let node = pluck(v, dotted);
    assert!(
        node.is_number(),
        "{dotted:?} must be a number; got: {node}"
    );
}

#[test]
fn launch_microsite_demo_json_has_expected_shape() {
    let raw = read_file("data/demo.json");
    let v: Value = serde_json::from_str(&raw)
        .expect("data/demo.json must be valid JSON");

    // Hero
    for path in [
        "story.company",
        "story.tagline",
        "story.headline",
        "story.subheadline",
        "story.cta",
        "story.captured_at",
        "story.mode",
    ] {
        require_non_empty_string(&v, path);
    }

    // Before
    for path in [
        "before.weak_message.subject",
        "before.weak_message.body",
        "before.metrics.label",
        "before.agent_diagnosis",
    ] {
        require_non_empty_string(&v, path);
    }
    for path in [
        "before.metrics.sends",
        "before.metrics.open_rate",
        "before.metrics.reply_rate",
        "before.metrics.positive_reply_rate",
    ] {
        require_number(&v, path);
    }

    // Learning Moment
    for path in [
        "learning_moment.pattern.name",
        "learning_moment.pattern.summary",
        "learning_moment.candidate.skill_name",
        "learning_moment.candidate.problem",
        "learning_moment.candidate.reusable_pattern",
        "learning_moment.candidate.expected_reuse",
        "learning_moment.candidate.risk",
        "learning_moment.memory_entry_id",
    ] {
        require_non_empty_string(&v, path);
    }
    let evidence = pluck(&v, "learning_moment.pattern.evidence");
    let evidence_arr = evidence.as_array().expect("evidence must be array");
    assert!(
        evidence_arr.len() >= 3,
        "evidence must have ≥3 observations to qualify the pattern; got {}",
        evidence_arr.len()
    );

    // After
    for path in [
        "after.improved_message.subject",
        "after.improved_message.body",
        "after.why_improved",
        "after.active_skill_name",
    ] {
        require_non_empty_string(&v, path);
    }

    // Proof — receipts the launch video pivots on
    for path in [
        "proof.role.session_id",
        "proof.role.vertical",
        "proof.role.provider",
        "proof.candidate_promoted.memory_entry_id",
        "proof.candidate_promoted.to_skill_id",
        "proof.candidate_promoted.from_state",
        "proof.candidate_promoted.to_state",
        "proof.skill_approved.skill_id",
        "proof.skill_approved.from_state",
        "proof.skill_approved.to_state",
        "proof.active_skill.name",
        "proof.scored_eval.fixtures",
    ] {
        require_non_empty_string(&v, path);
    }
    for path in [
        "proof.scored_eval.passed",
        "proof.scored_eval.scored",
        "proof.scored_eval.pass_rate",
    ] {
        require_number(&v, path);
    }
    let mem_writes = pluck(&v, "proof.memory_writes")
        .as_array()
        .expect("proof.memory_writes must be array");
    assert!(
        !mem_writes.is_empty(),
        "proof.memory_writes must be non-empty"
    );
    for (i, w) in mem_writes.iter().enumerate() {
        assert!(
            w.get("kind").and_then(|x| x.as_str()).is_some(),
            "memory_writes[{i}] missing 'kind' string"
        );
        assert!(
            w.get("id").and_then(|x| x.as_str()).is_some(),
            "memory_writes[{i}] missing 'id' string"
        );
    }
}

#[test]
fn launch_microsite_index_html_references_required_assets() {
    let html = read_file("index.html");
    for needle in ["style.css", "app.js", "data/demo.json"] {
        assert!(
            html.contains(needle),
            "index.html must reference {needle:?}; not found in body"
        );
    }
    // Must declare a UTF-8 charset and a viewport meta — the page is
    // mobile-responsive and the JSON contains characters like —.
    assert!(
        html.to_lowercase().contains("charset=\"utf-8\""),
        "index.html must declare charset=utf-8"
    );
    assert!(
        html.to_lowercase().contains("name=\"viewport\""),
        "index.html must declare a viewport meta for mobile"
    );
}

#[test]
fn launch_microsite_readme_documents_both_modes() {
    let md = read_file("README.md").to_lowercase();
    assert!(
        md.contains("deterministic"),
        "README.md must mention deterministic mode"
    );
    let mentions_live = md.contains("--live")
        || md.contains("live_llm")
        || md.contains("live mode");
    assert!(
        mentions_live,
        "README.md must mention live mode (--live / LIVE_LLM=1 / 'live mode')"
    );
    assert!(
        md.contains("anthropic_api_key"),
        "README.md must mention ANTHROPIC_API_KEY for live-mode setup"
    );
    // Recording-the-video section is the launch artifact's reason for
    // existing — make sure it's not silently dropped on a refactor.
    assert!(
        md.contains("video"),
        "README.md must include guidance for recording the launch video"
    );
}
