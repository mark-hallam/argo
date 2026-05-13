//! Per-vertical regression harness — static structural check.
//!
//! For every vertical under `verticals/<id>/`, this crate's tests
//! load the manifest + every `eval/seed.jsonl` fixture and assert:
//!
//! 1. Every fixture's `expected.action_type` (or each entry in
//!    `expected.action_chain`) names an action declared in the
//!    manifest's `[actions]` vocabulary.
//! 2. When a fixture sets `expected.requires_approval`, the
//!    declared approval requirement on the manifest agrees: a
//!    `true` fixture must list the action_type under
//!    `[escalation.human_approval_required]`; a `false` fixture
//!    must not.
//!
//! What this crate is NOT:
//! - Not a live eval driver — no LLM calls, no E2B. That work
//!   already exists via `argo eval run`.
//! - Not a behavioural assertion — fixtures using the
//!   `must_include` / `must_not_include` regression schema are
//!   out of scope; the runtime side of those checks lives in
//!   `argo-eval`.
//!
//! Source of truth for the walk is the on-disk `verticals/` tree
//! resolved relative to the workspace root via
//! [`workspace_root`]. The tests live under `tests/regression_all.rs`.
use argo_vertical::{parse, VerticalManifest};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Pre-existing fixture/manifest drift that this test is being
/// landed *on top of*, not introduced by. Each entry is a
/// `(vertical_id, action_type)` pair where the fixture(s) in
/// `verticals/<vertical_id>/eval/seed.jsonl` reference an
/// `action_type` that is not declared in the manifest's
/// `[actions]` table.
///
/// Listed here so the test can ship as a forward-looking guard
/// without bundling a verticals-content cleanup the brief's
/// may-touch list forbids. The conductor queues
/// `mission-vertical-action-vocab-cleanup` to drain this list;
/// once a row is fixed in the vertical (either by adding the
/// declaration or retiring the fixture), the matching baseline
/// row here must be removed in the same PR — the test fails if
/// the baseline gets stale in either direction.
///
/// Authored against `origin/main` `392baa27` on 2026-05-13.
const KNOWN_DRIFT: &[(&str, &str)] = &[
    ("marketing-intern", "draft_newsletter"),
    ("marketing-intern", "recommend_post_time"),
    ("sales-sdr", "handle_objection"),
    ("sales-sdr", "update_lead_state"),
];

/// One parsed vertical the harness will check.
#[derive(Debug, Clone)]
pub struct VerticalCase {
    /// The manifest's declared `vertical.id`.
    pub id: String,
    /// Path of the `vertical.toml` we read.
    pub manifest_path: PathBuf,
    /// Parsed manifest.
    pub manifest: VerticalManifest,
    /// Fixtures discovered under `eval/seed.jsonl`.
    pub fixtures: Vec<Fixture>,
}

/// One JSONL line from a vertical's eval suite.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// JSONL file the fixture came from.
    pub source_path: PathBuf,
    /// 0-based line index within that file.
    pub line_index: usize,
    /// Fixture `id` field (best-effort — empty if absent).
    pub id: String,
    /// Raw parsed JSON.
    pub raw: Value,
}

/// One regression-test violation. Aggregated across every case
/// so the test panics once with the full picture, not once per
/// fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// `expected.action_type` (or an entry in `action_chain`)
    /// names an action_type not declared in the manifest's
    /// `[actions]` vocab.
    UndeclaredActionType {
        vertical: String,
        fixture: String,
        action_type: String,
    },
    /// `expected.requires_approval` disagrees with the manifest's
    /// `[escalation.human_approval_required]` for the same
    /// action_type.
    ApprovalMismatch {
        vertical: String,
        fixture: String,
        action_type: String,
        expected_approval: bool,
        declared_required: bool,
    },
}

impl Violation {
    /// Stable key for matching against `KNOWN_DRIFT`. Pairs the
    /// vertical id with the offending action_type so a single
    /// drift entry covers every fixture that uses it.
    #[must_use]
    pub fn drift_key(&self) -> (&str, &str) {
        match self {
            Self::UndeclaredActionType {
                vertical,
                action_type,
                ..
            }
            | Self::ApprovalMismatch {
                vertical,
                action_type,
                ..
            } => (vertical.as_str(), action_type.as_str()),
        }
    }

    /// Human-readable rendering used in the test failure body.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::UndeclaredActionType {
                vertical,
                fixture,
                action_type,
            } => format!(
                "[{vertical}/{fixture}] expected.action_type \"{action_type}\" is not declared in the manifest's [actions] vocabulary"
            ),
            Self::ApprovalMismatch {
                vertical,
                fixture,
                action_type,
                expected_approval,
                declared_required,
            } => format!(
                "[{vertical}/{fixture}] expected.requires_approval={expected_approval} disagrees with [escalation.human_approval_required] for action_type \"{action_type}\" (declared {declared_required})"
            ),
        }
    }
}

/// Walk `verticals_dir`, parse every `vertical.toml`, and return
/// one [`VerticalCase`] per role-bearing vertical. Library-tier
/// bundles (no `[role]` section) are skipped — they have no
/// agent and therefore no action-vocabulary to check.
///
/// Verticals whose manifest fails to parse are silently dropped:
/// `argo-vertical::manifest_validate_tests::fixture_manifests_pass_strict`
/// already gates manifest validity. This crate's job is the
/// fixture↔manifest cross-check.
#[must_use]
pub fn discover(verticals_dir: &Path) -> Vec<VerticalCase> {
    let mut cases = Vec::new();
    let Ok(entries) = fs::read_dir(verticals_dir) else {
        return cases;
    };
    let mut subdirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();
    for sub in subdirs {
        let manifest_path = sub.join("vertical.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let Ok(src) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = parse(&src) else {
            continue;
        };
        if manifest.role.is_none() {
            continue;
        }
        let id = manifest.vertical.id.clone();
        let fixtures = load_fixtures(&sub);
        cases.push(VerticalCase {
            id,
            manifest_path,
            manifest,
            fixtures,
        });
    }
    cases
}

fn load_fixtures(vertical_root: &Path) -> Vec<Fixture> {
    let mut out = Vec::new();
    let suite = vertical_root.join("eval").join("seed.jsonl");
    if !suite.is_file() {
        return out;
    }
    let Ok(src) = fs::read_to_string(&suite) else {
        return out;
    };
    for (idx, line) in src.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(raw) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let id = raw
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        out.push(Fixture {
            source_path: suite.clone(),
            line_index: idx,
            id,
            raw,
        });
    }
    out
}

/// Cross-check a single [`VerticalCase`]'s fixtures against its
/// manifest. Returns every violation found.
#[must_use]
pub fn check(case: &VerticalCase) -> Vec<Violation> {
    let mut out = Vec::new();
    let Some(actions) = case.manifest.actions.as_ref() else {
        return out;
    };
    let declared: BTreeSet<&str> = actions.actions.keys().map(String::as_str).collect();
    let approval_required: BTreeSet<&str> = case
        .manifest
        .escalation
        .as_ref()
        .map(|e| e.human_approval_required.iter().map(String::as_str).collect())
        .unwrap_or_default();

    for fx in &case.fixtures {
        let Some(expected) = fx.raw.get("expected") else {
            continue;
        };
        if is_smoke_only(expected) {
            continue;
        }

        let action_types = collect_action_types(expected);
        if action_types.is_empty() {
            continue;
        }
        for at in &action_types {
            if !declared.contains(at.as_str()) {
                out.push(Violation::UndeclaredActionType {
                    vertical: case.id.clone(),
                    fixture: fx.id.clone(),
                    action_type: at.clone(),
                });
            }
        }
        if let Some(approval) = expected.get("requires_approval").and_then(Value::as_bool) {
            for at in &action_types {
                if !declared.contains(at.as_str()) {
                    continue;
                }
                let declared_required = approval_required.contains(at.as_str());
                if approval != declared_required {
                    out.push(Violation::ApprovalMismatch {
                        vertical: case.id.clone(),
                        fixture: fx.id.clone(),
                        action_type: at.clone(),
                        expected_approval: approval,
                        declared_required,
                    });
                }
            }
        }
    }
    out
}

/// Smoke-test fixtures (live-executor or e2b-required) are out
/// of scope per the brief — they belong to the live eval path,
/// not the static regression harness.
fn is_smoke_only(expected: &Value) -> bool {
    if expected.get("expected_executor").and_then(Value::as_str) == Some("human") {
        return true;
    }
    if expected.get("requires_e2b").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    false
}

fn collect_action_types(expected: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(s) = expected.get("action_type").and_then(Value::as_str) {
        out.push(s.to_string());
    }
    if let Some(arr) = expected.get("action_chain").and_then(Value::as_array) {
        for v in arr {
            if let Some(s) = v.as_str() {
                out.push(s.to_string());
            }
        }
    }
    out
}

/// Partition violations into `(new_violations, retired_baseline_keys)`.
///
/// `new_violations` are entries the test should fail on. They are
/// every observed [`Violation`] whose `(vertical, action_type)`
/// key is NOT in [`KNOWN_DRIFT`].
///
/// `retired_baseline_keys` are entries currently in
/// [`KNOWN_DRIFT`] that no longer appear in the observed
/// violation set — usually because the cleanup brief landed and
/// fixed the underlying drift. They are surfaced so the test
/// fails until the baseline row is removed, preventing the
/// baseline from drifting out of sync with reality.
#[must_use]
pub fn partition(violations: &[Violation]) -> (Vec<Violation>, Vec<(String, String)>) {
    let baseline: BTreeSet<(&str, &str)> = KNOWN_DRIFT.iter().copied().collect();
    let mut new_violations = Vec::new();
    let mut observed_keys: BTreeSet<(String, String)> = BTreeSet::new();
    for v in violations {
        let (vid, at) = v.drift_key();
        observed_keys.insert((vid.to_string(), at.to_string()));
        if !baseline.contains(&(vid, at)) {
            new_violations.push(v.clone());
        }
    }
    let retired: Vec<(String, String)> = KNOWN_DRIFT
        .iter()
        .filter(|(v, a)| !observed_keys.contains(&((*v).to_string(), (*a).to_string())))
        .map(|(v, a)| ((*v).to_string(), (*a).to_string()))
        .collect();
    (new_violations, retired)
}

/// Workspace root resolved from this crate's `CARGO_MANIFEST_DIR`.
/// Mirrors the convention in
/// `argo-vertical::manifest_validate_tests::fixture_manifests_pass_strict`.
#[must_use]
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("workspace root above the crate dir")
}

#[cfg(test)]
mod tests;
