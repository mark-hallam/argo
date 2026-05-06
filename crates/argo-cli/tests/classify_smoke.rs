//! End-to-end smoke for `argo classify set/list/rules` (Stream
//! `claude-data-classification`). Covers the tag round-trip, the
//! `--expired` retention filter, and the rules-print path that
//! reads `<workspace>/.argo/classification.toml`.

use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn classify_set_list_filters_and_rules_round_trip() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    // Tag three records with different classes / kinds / authors.
    let set_pii = run(
        workspace.path(),
        home.path(),
        &[
            "classify",
            "set",
            "memory_entry:m-1",
            "pii",
            "--by",
            "op-1",
            "--json",
        ],
    );
    assert_ok(&set_pii, "classify set pii");
    let set_pii_json: serde_json::Value = serde_json::from_slice(&set_pii.stdout).unwrap();
    assert_eq!(set_pii_json["class"], "pii");
    assert_eq!(set_pii_json["record_kind"], "memory_entry");
    assert_eq!(set_pii_json["record_id"], "m-1");
    assert_eq!(set_pii_json["tagged_by"], "op-1");

    assert_ok(
        &run(
            workspace.path(),
            home.path(),
            &[
                "classify",
                "set",
                "audit_record:a-1",
                "secret",
                "--by",
                "op-2",
                "--json",
            ],
        ),
        "classify set secret",
    );
    assert_ok(
        &run(
            workspace.path(),
            home.path(),
            &[
                "classify",
                "set",
                "memory_entry:m-2",
                "phi",
                "--json",
            ],
        ),
        "classify set phi",
    );

    // List all → 3 rows.
    let list_all = run(
        workspace.path(),
        home.path(),
        &["classify", "list", "--json"],
    );
    assert_ok(&list_all, "classify list all");
    let arr: serde_json::Value = serde_json::from_slice(&list_all.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 3, "expected 3: {arr}");

    // Filter by class=pii → 1 row.
    let list_pii = run(
        workspace.path(),
        home.path(),
        &["classify", "list", "--class", "pii", "--json"],
    );
    assert_ok(&list_pii, "classify list pii");
    let arr: serde_json::Value = serde_json::from_slice(&list_pii.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 1);
    assert_eq!(arr[0]["record_id"], "m-1");

    // Filter by kind=memory_entry → 2 rows.
    let list_mem = run(
        workspace.path(),
        home.path(),
        &["classify", "list", "--kind", "memory_entry", "--json"],
    );
    assert_ok(&list_mem, "classify list kind=memory_entry");
    let arr: serde_json::Value = serde_json::from_slice(&list_mem.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 2);

    // Filter by worker=op-1 → 1 row.
    let list_w1 = run(
        workspace.path(),
        home.path(),
        &["classify", "list", "--worker", "op-1", "--json"],
    );
    assert_ok(&list_w1, "classify list worker=op-1");
    let arr: serde_json::Value = serde_json::from_slice(&list_w1.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 1);
    assert_eq!(arr[0]["class"], "pii");

    // Combined filters: class=pii AND kind=memory_entry → 1 row.
    let list_combo = run(
        workspace.path(),
        home.path(),
        &[
            "classify",
            "list",
            "--class",
            "pii",
            "--kind",
            "memory_entry",
            "--json",
        ],
    );
    assert_ok(&list_combo, "classify list combo");
    let arr: serde_json::Value = serde_json::from_slice(&list_combo.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 1);

    // --expired with default ship rules: nothing is expired (all
    // tags are ~now). Empty array.
    let list_expired = run(
        workspace.path(),
        home.path(),
        &["classify", "list", "--expired", "--json"],
    );
    assert_ok(&list_expired, "classify list --expired");
    let arr: serde_json::Value = serde_json::from_slice(&list_expired.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 0);

    // Rules: shipped defaults.
    let rules = run(
        workspace.path(),
        home.path(),
        &["classify", "rules", "--json"],
    );
    assert_ok(&rules, "classify rules");
    let obj: serde_json::Value = serde_json::from_slice(&rules.stdout).unwrap();
    assert_eq!(obj["pii"]["retention_days"], 90);
    assert_eq!(obj["pii"]["redact_in_export"], true);
    assert_eq!(obj["secret"]["retention_days"], 30);
    assert_eq!(obj["public"]["retention_days"], 0);
    assert_eq!(obj["public"]["redact_in_export"], false);

    // Workspace override: drop a classification.toml that shrinks
    // PII retention to 1 day. Then re-run rules and confirm.
    let argo_dir = workspace.path().join(".argo");
    std::fs::create_dir_all(&argo_dir).unwrap();
    std::fs::write(
        argo_dir.join("classification.toml"),
        "[pii]\nretention_days = 1\n",
    )
    .unwrap();
    let rules2 = run(
        workspace.path(),
        home.path(),
        &["classify", "rules", "--json"],
    );
    assert_ok(&rules2, "classify rules after override");
    let obj2: serde_json::Value = serde_json::from_slice(&rules2.stdout).unwrap();
    assert_eq!(
        obj2["pii"]["retention_days"], 1,
        "expected override to take effect: {obj2}"
    );
    // redact_in_export wasn't overridden → falls back to ship default.
    assert_eq!(obj2["pii"]["redact_in_export"], true);
}

#[test]
fn classify_set_rejects_unknown_class() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &["classify", "set", "memory_entry:x", "medical", "--json"],
    );
    assert!(!bad.status.success(), "expected non-zero on unknown class");
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("medical") || stderr.contains("class"),
        "expected helpful error, got: {stderr}",
    );
}

#[test]
fn classify_set_rejects_malformed_target() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &["classify", "set", "no_colon_here", "pii"],
    );
    assert!(!bad.status.success());
}
