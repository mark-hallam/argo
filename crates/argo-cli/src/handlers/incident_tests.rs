//! Tests for the `incident` handler — slug generation, YAML
//! fixture round-trip, overwrite refusal, and the interactive
//! prompt shape. Wired in from `handlers/mod.rs` under
//! `#[cfg(test)]`.

use super::incident::{
    load_fixture, output_path, prompt_input, render, run_new, slugify, today_utc,
    IncidentInput, MAX_SLUG_LEN, TEMPLATE_BODY,
};
use std::fs;
use std::io::Cursor;

fn sample_input() -> IncidentInput {
    IncidentInput {
        summary: "Gateway 502s after key rotation".to_string(),
        severity: "SEV2".to_string(),
        timeline: "- 14:02 UTC — operator rotated key.\n- 14:11 UTC — recovered.".to_string(),
        root_cause: "Cached secret in process memory.".to_string(),
        contributing_factors: "No reload runbook.".to_string(),
        detection: "Alert fired six minutes after onset.".to_string(),
        resolution: "Rolled key back, restarted gateway.".to_string(),
        action_items: "- [ ] Add `argo cred reload`.".to_string(),
        lessons_learned: "Cached secrets are silent footguns.".to_string(),
    }
}

fn sample_yaml() -> String {
    let input = sample_input();
    serde_yaml::to_string(&input).expect("serialize sample yaml")
}

#[test]
fn yaml_fixture_round_trip_writes_complete_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("fix.yaml");
    fs::write(&fixture, sample_yaml()).expect("write fixture");

    let input = load_fixture(&fixture).expect("load fixture");
    let path = run_new(input.clone(), dir.path(), false).expect("run_new");

    assert!(path.exists(), "output file should exist at {}", path.display());
    let body = fs::read_to_string(&path).expect("read output");
    // Every required section's content must appear in the rendered file.
    assert!(body.contains(&input.summary));
    assert!(body.contains(&input.severity));
    assert!(body.contains(&input.timeline));
    assert!(body.contains(&input.root_cause));
    assert!(body.contains(&input.contributing_factors));
    assert!(body.contains(&input.detection));
    assert!(body.contains(&input.resolution));
    assert!(body.contains(&input.action_items));
    assert!(body.contains(&input.lessons_learned));
    // Blameless note is non-negotiable per the brief.
    assert!(body.contains("Blameless"));
    // No unfilled placeholders left behind.
    assert!(!body.contains("{{"));
}

#[test]
fn missing_required_field_returns_clear_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut input = sample_input();
    input.root_cause = "   ".to_string(); // whitespace-only counts as empty

    let err = run_new(input, dir.path(), false).expect_err("expected schema error");
    let msg = format!("{err}");
    assert!(
        msg.contains("root_cause"),
        "error should name the missing field, got: {msg}"
    );
}

#[test]
fn slug_generation_normalizes_messy_summary() {
    let slug = slugify("  Gateway 502s after KEY rotation!! (oncall)  ");
    assert_eq!(slug, "gateway-502s-after-key-rotation-oncall");

    // Punctuation-only input collapses to the placeholder stem.
    assert_eq!(slugify("***"), "incident");

    // Cap at MAX_SLUG_LEN.
    let long = "a".repeat(120);
    let slug = slugify(&long);
    assert!(slug.chars().count() <= MAX_SLUG_LEN);

    // Trailing dashes from truncation are trimmed.
    let messy = format!("{}!!!{}", "x".repeat(58), "y".repeat(20));
    let slug = slugify(&messy);
    assert!(!slug.ends_with('-'), "trimmed slug = {slug}");
}

#[test]
fn refuses_to_overwrite_without_force() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = sample_input();
    let first = run_new(input.clone(), dir.path(), false).expect("first write");
    assert!(first.exists());

    let err = run_new(input, dir.path(), false).expect_err("second write should refuse");
    let msg = format!("{err}");
    assert!(msg.contains("--force"), "error should suggest --force, got: {msg}");
    assert!(msg.contains("refusing to overwrite"));
}

#[test]
fn force_flag_replaces_existing_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut input = sample_input();
    let path = run_new(input.clone(), dir.path(), false).expect("first write");
    let original = fs::read_to_string(&path).expect("read original");

    input.root_cause = "Updated root cause body.".to_string();
    let path2 = run_new(input.clone(), dir.path(), true).expect("force overwrite");
    assert_eq!(path, path2, "overwrite should hit the same path");

    let updated = fs::read_to_string(&path).expect("read updated");
    assert_ne!(original, updated);
    assert!(updated.contains("Updated root cause body."));
}

#[test]
fn non_interactive_path_bypasses_stdin() {
    // Simulate an empty stdin — `--from <yaml>` must not consult it.
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("fix.yaml");
    fs::write(&fixture, sample_yaml()).expect("write fixture");

    // Loading the fixture and rendering should succeed without
    // touching stdin at all (we never call `prompt_input` here).
    let input = load_fixture(&fixture).expect("load fixture");
    let path = run_new(input, dir.path(), false).expect("run_new");
    assert!(path.exists());

    // Filename shape: `YYYY-MM-DD-<slug>.md`, where date is today UTC.
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("stem is utf-8");
    assert!(stem.starts_with(&today_utc()), "stem = {stem}");
    let computed = output_path(dir.path(), &today_utc(), "Gateway 502s after key rotation");
    assert_eq!(path, computed);
}

#[test]
fn interactive_prompt_collects_each_section_and_timeline_round_trips() {
    // One line per single-line prompt, multi-line sections terminated
    // by `.`. Order must match the prompt list in `incident.rs`.
    let scripted = concat!(
        "Gateway 502s after key rotation\n",            // summary
        "SEV2\n",                                       // severity
        "- 14:02 UTC — rotated key.\n",                 // timeline (multiline)
        "- 14:11 UTC — recovered.\n",
        ".\n",
        "Cached secret in process memory.\n",           // root_cause
        ".\n",
        "No reload runbook.\n",                         // contributing_factors
        ".\n",
        "Alert fired six minutes after onset.\n",       // detection
        ".\n",
        "Rolled key back, restarted gateway.\n",        // resolution
        ".\n",
        "- [ ] Add `argo cred reload`.\n",              // action_items
        ".\n",
        "Cached secrets are silent footguns.\n",        // lessons_learned
        ".\n",
    );
    let mut output = Vec::new();
    let input = prompt_input(Cursor::new(scripted), &mut output).expect("prompt");

    assert_eq!(input.summary, "Gateway 502s after key rotation");
    assert_eq!(input.severity, "SEV2");
    // Timeline preserves the bullet list across the round-trip.
    assert_eq!(
        input.timeline,
        "- 14:02 UTC — rotated key.\n- 14:11 UTC — recovered."
    );
    assert!(input.lessons_learned.contains("silent footguns"));

    // Render → search ensures the multi-line timeline survives the
    // template substitution as well.
    let body = render(TEMPLATE_BODY, &input);
    assert!(body.contains("- 14:02 UTC — rotated key."));
    assert!(body.contains("- 14:11 UTC — recovered."));

    // Prompts were actually written to the output stream.
    let prompts = String::from_utf8(output).expect("utf-8 prompts");
    assert!(prompts.contains("Headline summary"));
    assert!(prompts.contains("Severity"));
}
