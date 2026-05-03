//! Codex live-judge calibration tests (#701d). Sibling of
//! `vertical_eval_judge.rs` so each test file stays under the
//! 500-LOC budget.
//!
//! Opt-in: skipped unless `ANTHROPIC_API_KEY` is set. When
//! set, runs the live LLM judge against the three Codex
//! adapter files from #700 (best / medium / worst) and
//! asserts the calibration ordering.
//!
//! - `codex_calibration_live_judge_orders_best_above_worst` —
//!   HARD assertion: worst must be strictly lowest. Soft
//!   warning if best ≤ medium (corpus labels are debatable;
//!   per-fixture scores are printed for inspection).
//! - `codex_calibration_strict_best_above_medium_above_worst` —
//!   Aspirational. Skipped without ANTHROPIC_API_KEY OR
//!   ARGO_CALIBRATION_STRICT=1. Becomes the regression lock
//!   once the rubric reliably ranks best > medium.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn run_live_judge_against_corpus(actuals_filename: &str) -> serde_json::Value {
    let root = workspace_root();
    let actuals = root.join("eval-actuals").join(actuals_filename);
    assert!(
        actuals.is_file(),
        "Codex corpus file missing — expected at {actuals:?}"
    );
    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", root.join("verticals"))
        .current_dir(&root) // manifests/providers/anthropic.toml resolves from cwd
        .args([
            "vertical",
            "eval",
            "gtm-engineer",
            "--score",
            "--json",
            "--actuals",
            actuals.to_str().unwrap(),
            "--judge",
            "anthropic/claude-sonnet-4-6",
        ])
        .output()
        .expect("spawn live judge");
    let body = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    serde_json::from_str(&body).unwrap_or_else(|e| {
        panic!(
            "live judge JSON parse failed for {actuals_filename}: {e}\n\
             stdout:\n{body}\n\
             stderr:\n{stderr}"
        )
    })
}

fn average_judge_score(suite_json: &serde_json::Value) -> f64 {
    let empty: Vec<serde_json::Value> = Vec::new();
    let fixtures = suite_json["suites"][0]["fixtures"]
        .as_array()
        .unwrap_or(&empty);
    let mut sum = 0.0;
    let mut count = 0;
    for fx in fixtures {
        if let Some(j) = fx.get("judge") {
            if let Some(s) = j.get("score").and_then(|x| x.as_f64()) {
                sum += s;
                count += 1;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

fn print_per_fixture_scores(label: &str, suite_json: &serde_json::Value) {
    eprintln!("--- {label} ---");
    let empty: Vec<serde_json::Value> = Vec::new();
    let fixtures = suite_json["suites"][0]["fixtures"]
        .as_array()
        .unwrap_or(&empty)
        .clone();
    for fx in fixtures {
        if let Some(j) = fx.get("judge") {
            eprintln!(
                "  {} → score={:?} passed={:?} reason={}",
                fx["id"].as_str().unwrap_or("?"),
                j["score"],
                j["passed"],
                j["reason"].as_str().unwrap_or("?")
            );
        }
    }
}

fn anthropic_key_present() -> bool {
    !std::env::var("ANTHROPIC_API_KEY")
        .map(|s| s.trim().is_empty())
        .unwrap_or(true)
}

#[test]
fn codex_calibration_live_judge_orders_best_above_worst() {
    if !anthropic_key_present() {
        eprintln!(
            "SKIP: ANTHROPIC_API_KEY not set — calibration test requires live judge"
        );
        return;
    }
    let best = run_live_judge_against_corpus("gtm_engineer_codex_best.jsonl");
    let medium = run_live_judge_against_corpus("gtm_engineer_codex_medium.jsonl");
    let worst = run_live_judge_against_corpus("gtm_engineer_codex_worst.jsonl");
    let best_avg = average_judge_score(&best);
    let medium_avg = average_judge_score(&medium);
    let worst_avg = average_judge_score(&worst);
    eprintln!(
        "calibration averages: best={best_avg:.4} medium={medium_avg:.4} worst={worst_avg:.4}"
    );

    // Hard contract: worst MUST be lowest. The deliberately
    // weak Codex variants ("Save 20 hours a week", "scale to
    // 5,000", "any company trying to use AI") are
    // unambiguously worse than the strong/medium variants. If
    // this flips, the rubric is broken — print everything to
    // help diagnosis.
    if !(best_avg > worst_avg && medium_avg > worst_avg) {
        print_per_fixture_scores("best", &best);
        print_per_fixture_scores("medium", &medium);
        print_per_fixture_scores("worst", &worst);
        panic!(
            "calibration FAIL: worst ({worst_avg:.4}) was not strictly lowest. \
             best={best_avg:.4} medium={medium_avg:.4}. Rubric needs revision."
        );
    }

    // Soft signal: best > medium. The Codex labels are
    // debatable (the "medium" variants often have stronger
    // assumption-labeling, which is canonical Argo
    // discipline). Print on inversion but do NOT panic — the
    // failure signal is the per-fixture report, which helps
    // the operator decide whether to re-tune the rubric or
    // re-label the corpus.
    if best_avg <= medium_avg {
        print_per_fixture_scores("best", &best);
        print_per_fixture_scores("medium", &medium);
        print_per_fixture_scores("worst", &worst);
        eprintln!(
            "WARN: best ({best_avg:.4}) ≤ medium ({medium_avg:.4}). \
             Not necessarily a rubric bug — the corpus labels are based on \
             superficial structure, not Argo-contract alignment. The medium \
             variants often lead with explicit assumption-labeling, which the \
             Argo `define_icp` skill spec rewards. Inspect the per-fixture \
             scores above to decide whether to re-tune the rubric or accept \
             the ordering. Set ARGO_CALIBRATION_STRICT=1 to enforce best > medium."
        );
    }
}

#[test]
fn codex_calibration_strict_best_above_medium_above_worst() {
    // Aspirational version: enforces the full ordering.
    // Skipped without ANTHROPIC_API_KEY OR without the
    // ARGO_CALIBRATION_STRICT opt-in. When the rubric tuning
    // lands the inversion fix, this test starts passing and
    // becomes the regression lock. Until then it's a
    // measurable target.
    if !anthropic_key_present() {
        eprintln!(
            "SKIP: ANTHROPIC_API_KEY not set — strict calibration test requires live judge"
        );
        return;
    }
    if std::env::var("ARGO_CALIBRATION_STRICT")
        .map(|s| s.trim().is_empty())
        .unwrap_or(true)
    {
        eprintln!(
            "SKIP: set ARGO_CALIBRATION_STRICT=1 alongside ANTHROPIC_API_KEY to enforce strict ordering. \
             The companion test `codex_calibration_live_judge_orders_best_above_worst` always runs \
             when the key is set."
        );
        return;
    }
    let best = run_live_judge_against_corpus("gtm_engineer_codex_best.jsonl");
    let medium = run_live_judge_against_corpus("gtm_engineer_codex_medium.jsonl");
    let worst = run_live_judge_against_corpus("gtm_engineer_codex_worst.jsonl");
    let best_avg = average_judge_score(&best);
    let medium_avg = average_judge_score(&medium);
    let worst_avg = average_judge_score(&worst);
    if !(best_avg > medium_avg && medium_avg > worst_avg) {
        print_per_fixture_scores("best", &best);
        print_per_fixture_scores("medium", &medium);
        print_per_fixture_scores("worst", &worst);
        panic!(
            "strict calibration FAIL: expected best > medium > worst; \
             got best={best_avg:.4} medium={medium_avg:.4} worst={worst_avg:.4}"
        );
    }
}
