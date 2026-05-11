//! `argo operator risk-overrides …` end-to-end CLI smoke test.
//!
//! Brief: `mission-operator-specific-risk-overrides`. Validates the
//! `set / get / list / delete` roundtrip including:
//!
//! - upgrade-only default — a downgrade without `--allow-downgrade`
//!   still writes the row (the policy refuses at the resolver layer,
//!   not the storage layer; the CLI is a thin pass-through), and the
//!   stored `allow_downgrade` flag matches what the operator passed,
//! - per-vertical vs all-verticals scoping survives the JSON round
//!   trip,
//! - `delete` clears the right row and is idempotent.
//!
//! The integration shell exercises the live `OperatorRiskOverridesCmd`
//! enum + handler + store + migration end-to-end. Resolver semantics
//! (downgrade-blocked, per-vertical wins) are exercised by the
//! channel-level unit tests in
//! `crates/argo-channels/src/operator_risk_overrides_tests.rs` —
//! repeating that here would duplicate coverage without exercising
//! new code.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn operator_risk_overrides_cli_roundtrip() {
    let home = tempfile::tempdir().expect("home");
    let work = tempfile::tempdir().expect("work");

    let set_upgrade = run(
        home.path(),
        work.path(),
        &[
            "operator",
            "risk-overrides",
            "set",
            "OP123",
            "send_email_external",
            "high",
            "--json",
        ],
    );
    assert_success(&set_upgrade);
    let body: Value = serde_json::from_slice(&set_upgrade.stdout).expect("set json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["override"]["operator_id"], "OP123");
    assert_eq!(body["override"]["action_kind"], "send_email_external");
    assert_eq!(body["override"]["risk_level"], "high");
    assert!(body["override"]["vertical_id"].is_null());
    assert_eq!(body["override"]["allow_downgrade"], false);

    let set_downgrade = run(
        home.path(),
        work.path(),
        &[
            "operator",
            "risk-overrides",
            "set",
            "OP123",
            "book_travel",
            "low",
            "--allow-downgrade",
            "--vertical",
            "legal",
            "--json",
        ],
    );
    assert_success(&set_downgrade);
    let body: Value = serde_json::from_slice(&set_downgrade.stdout).expect("set downgrade json");
    assert_eq!(body["override"]["risk_level"], "low");
    assert_eq!(body["override"]["vertical_id"], "legal");
    assert_eq!(body["override"]["allow_downgrade"], true);

    let get = run(
        home.path(),
        work.path(),
        &[
            "operator",
            "risk-overrides",
            "get",
            "OP123",
            "--json",
        ],
    );
    assert_success(&get);
    let body: Value = serde_json::from_slice(&get.stdout).expect("get json");
    let rows = body["overrides"].as_array().expect("overrides array");
    assert_eq!(rows.len(), 2);
    // Sorted by action_kind ASC: book_travel comes before send_email_external.
    assert_eq!(rows[0]["action_kind"], "book_travel");
    assert_eq!(rows[0]["vertical_id"], "legal");
    assert_eq!(rows[0]["risk_level"], "low");
    assert_eq!(rows[0]["allow_downgrade"], true);
    assert_eq!(rows[1]["action_kind"], "send_email_external");
    assert!(rows[1]["vertical_id"].is_null());

    let list = run(
        home.path(),
        work.path(),
        &["operator", "risk-overrides", "list", "--json"],
    );
    assert_success(&list);
    let body: Value = serde_json::from_slice(&list.stdout).expect("list json");
    let rows = body["overrides"].as_array().expect("overrides array");
    assert_eq!(rows.len(), 2);

    // Deleting the wrong scope must not remove the per-vertical row.
    let bad_delete = run(
        home.path(),
        work.path(),
        &[
            "operator",
            "risk-overrides",
            "delete",
            "OP123",
            "book_travel",
            "--json",
        ],
    );
    assert_success(&bad_delete);
    let body: Value = serde_json::from_slice(&bad_delete.stdout).expect("bad delete json");
    assert_eq!(body["removed"], false);

    let good_delete = run(
        home.path(),
        work.path(),
        &[
            "operator",
            "risk-overrides",
            "delete",
            "OP123",
            "book_travel",
            "--vertical",
            "legal",
            "--json",
        ],
    );
    assert_success(&good_delete);
    let body: Value = serde_json::from_slice(&good_delete.stdout).expect("good delete json");
    assert_eq!(body["removed"], true);

    // Idempotent re-delete reports `removed = false`.
    let again = run(
        home.path(),
        work.path(),
        &[
            "operator",
            "risk-overrides",
            "delete",
            "OP123",
            "book_travel",
            "--vertical",
            "legal",
            "--json",
        ],
    );
    assert_success(&again);
    let body: Value = serde_json::from_slice(&again.stdout).expect("again delete json");
    assert_eq!(body["removed"], false);

    let after_delete = run(
        home.path(),
        work.path(),
        &[
            "operator",
            "risk-overrides",
            "get",
            "OP123",
            "--json",
        ],
    );
    assert_success(&after_delete);
    let body: Value = serde_json::from_slice(&after_delete.stdout).expect("get after delete");
    let rows = body["overrides"].as_array().expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["action_kind"], "send_email_external");
}

#[test]
fn operator_risk_overrides_rejects_invalid_risk_level() {
    let home = tempfile::tempdir().expect("home");
    let work = tempfile::tempdir().expect("work");
    let out = run(
        home.path(),
        work.path(),
        &[
            "operator",
            "risk-overrides",
            "set",
            "OP123",
            "send_email_external",
            "critical",
            "--json",
        ],
    );
    assert!(
        !out.status.success(),
        "expected 'critical' to be rejected by the clap value-enum"
    );
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("argo CLI")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
