//! Integration smoke: seed a local skill matching one marketplace
//! entry, then run `argo skill catalog --json` and assert the
//! installed-row carries `installed: true` while the rest carry
//! `installed: false`.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::process::{Command, Output};

#[test]
fn catalog_marks_installed_rows_and_filters() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();

    // Seed: install a skill whose name matches a known marketplace
    // entry (`marketplace-listing-draft`). The marketplace const
    // index in `argo-skill::marketplace` carries that name.
    let seed = run(
        home.path(),
        workspace.path(),
        &[
            "skill",
            "create",
            "marketplace-listing-draft",
            "--body",
            "local body for smoke test",
            "--new-version",
            "0.9",
        ],
    );
    assert_success(&seed);

    // Default catalog: all marketplace rows, with one [x].
    let all = run(
        home.path(),
        workspace.path(),
        &["skill", "catalog", "--json"],
    );
    assert_success(&all);
    let rows: Value = serde_json::from_slice(&all.stdout).unwrap();
    let rows = rows.as_array().unwrap();
    assert!(
        rows.len() >= 2,
        "expected ≥2 marketplace rows, got {}",
        rows.len()
    );
    let installed_count = rows.iter().filter(|r| r["installed"] == true).count();
    assert_eq!(installed_count, 1, "exactly one row should be installed");
    let listing = rows
        .iter()
        .find(|r| r["id"] == "marketplace-listing-draft")
        .expect("marketplace-listing-draft row missing");
    assert_eq!(listing["installed"], true);
    assert_eq!(listing["installed_version"], "0.9");

    // --installed-only should leave just the seeded row.
    let installed = run(
        home.path(),
        workspace.path(),
        &["skill", "catalog", "--installed-only", "--json"],
    );
    assert_success(&installed);
    let rows: Value = serde_json::from_slice(&installed.stdout).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], "marketplace-listing-draft");

    // --not-installed should exclude it.
    let not_installed = run(
        home.path(),
        workspace.path(),
        &["skill", "catalog", "--not-installed", "--json"],
    );
    assert_success(&not_installed);
    let rows: Value = serde_json::from_slice(&not_installed.stdout).unwrap();
    let rows = rows.as_array().unwrap();
    assert!(rows
        .iter()
        .all(|r| r["id"] != "marketplace-listing-draft"));
    assert!(rows.iter().all(|r| r["installed"] == false));

    // Unknown --vertical surfaces a clear error.
    let unknown = run(
        home.path(),
        workspace.path(),
        &["skill", "catalog", "--vertical", "no-such-tenant"],
    );
    assert!(
        !unknown.status.success(),
        "unknown vertical should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(
        stderr.contains("vertical no-such-tenant not found")
            || stderr.contains("not found"),
        "stderr did not mention the missing vertical: {stderr}"
    );

    // --installed-only + --not-installed should fail at clap parsing.
    let conflict = run(
        home.path(),
        workspace.path(),
        &[
            "skill",
            "catalog",
            "--installed-only",
            "--not-installed",
        ],
    );
    assert!(
        !conflict.status.success(),
        "combining --installed-only and --not-installed should be rejected"
    );
}

fn run(home: &std::path::Path, cwd: &std::path::Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
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
