//! Tests for [`crate::pack_lint`]. Cover each check by building a
//! tiny in-memory pack under `tempfile::tempdir()` and asserting the
//! finding surface. Real packs live under the workspace `packs/`
//! directory — we sample those via golden-path tests in the CLI
//! handler crate, not here, to keep this module self-contained.

use super::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write file");
}

fn minimal_pack(root: &Path) {
    write(
        &root.join("pack.toml"),
        "[pack]\nname='good-pack'\nversion='1.0.0'\n\n[[verticals]]\nid='good'\npath='.'\n",
    );
    write(&root.join("README.md"), "# good-pack\n");
}

fn find(report: &LintReport, id: CheckId, severity: PackLintSeverity) -> Option<&LintFinding> {
    report
        .findings
        .iter()
        .find(|f| f.id == id && f.severity == severity)
}

#[test]
fn clean_minimal_pack_has_no_findings() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    let report = lint_pack(tmp.path());
    assert!(report.findings.is_empty(), "got: {:?}", report.findings);
}

#[test]
fn missing_pack_toml_is_error() {
    let tmp = tempdir().unwrap();
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::PackToml, PackLintSeverity::Error).is_some());
}

#[test]
fn unparseable_pack_toml_is_error() {
    let tmp = tempdir().unwrap();
    write(&tmp.path().join("pack.toml"), "not = valid = toml = at = all");
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::PackToml, PackLintSeverity::Error).is_some());
}

#[test]
fn worker_toml_optional_when_missing() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::WorkerToml, PackLintSeverity::Error).is_none());
}

#[test]
fn unparseable_worker_toml_is_error() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    write(&tmp.path().join("worker.toml"), "][not toml");
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::WorkerToml, PackLintSeverity::Error).is_some());
}

#[test]
fn memory_schema_missing_type_is_error() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    // Collections-style with a field whose type is empty.
    write(
        &tmp.path().join("memory/schema.json"),
        r#"{"collections":[{"name":"c","fields":{"x":"","y":"string"}}]}"#,
    );
    let report = lint_pack(tmp.path());
    let f = find(&report, CheckId::MemorySchema, PackLintSeverity::Error)
        .expect("missing type should be error");
    assert!(f.message.contains("\"x\""));
}

#[test]
fn flat_memory_schema_missing_type_is_error() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    write(
        &tmp.path().join("memory_schema.json"),
        r#"{"fields":{"id":{"type":"string"},"created":{}}}"#,
    );
    let report = lint_pack(tmp.path());
    let f = find(&report, CheckId::MemorySchema, PackLintSeverity::Error)
        .expect("missing type should be error");
    assert!(f.message.contains("\"created\""));
}

#[test]
fn skill_ir_unparseable_is_error() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    write(&tmp.path().join("skills/bad.skill.json"), "{ not json");
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::SkillIrParse, PackLintSeverity::Error).is_some());
}

#[test]
fn skill_ir_empty_tool_binding_is_warning() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    let ir = serde_json::json!({
        "entry": "n1",
        "nodes": [{
            "id": "n1",
            "label": "x",
            "kind": {"type": "action", "instruction": "do x"},
            "annotations": {"toolBinding": {"mcpServer": "", "toolName": ""}},
        }],
        "edges": [],
        "metadata": {"name": "good_skill"}
    });
    write(
        &tmp.path().join("skills/good.skill.json"),
        &serde_json::to_string(&ir).unwrap(),
    );
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::SkillIrToolRef, PackLintSeverity::Warning).is_some());
}

#[test]
fn invalid_capabilities_is_error() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    let ir = serde_json::json!({
        "entry": "n1",
        "nodes": [{
            "id": "n1",
            "label": "x",
            "kind": {"type": "action", "instruction": "do x"},
            "annotations": {}
        }],
        "edges": [],
        "metadata": {"name": "bad_caps"},
        "capabilities": [{"kind": "no-such-capability"}]
    });
    write(
        &tmp.path().join("skills/bad_caps.skill.json"),
        &serde_json::to_string(&ir).unwrap(),
    );
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::Capabilities, PackLintSeverity::Error).is_some());
}

#[test]
fn eval_missing_expected_outcome_is_warning() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    write(
        &tmp.path().join("evals/golden.json"),
        r#"[{"id":"e1","prompt":"hi"}]"#,
    );
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::EvalShape, PackLintSeverity::Warning).is_some());
}

#[test]
fn eval_schema_version_zero_is_error() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    write(
        &tmp.path().join("evals/cases.json"),
        r#"{"schema_version":0,"cases":[]}"#,
    );
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::EvalParse, PackLintSeverity::Error).is_some());
}

#[test]
fn missing_readme_is_warning() {
    let tmp = tempdir().unwrap();
    write(
        &tmp.path().join("pack.toml"),
        "[pack]\nname='good-pack'\nversion='1.0.0'\n\n[[verticals]]\nid='good'\npath='.'\n",
    );
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::ReadmePresent, PackLintSeverity::Warning).is_some());
}

#[test]
fn oversized_file_is_warning() {
    let tmp = tempdir().unwrap();
    minimal_pack(tmp.path());
    let big = "x\n".repeat(501);
    write(&tmp.path().join("notes.md"), &big);
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::FileLocLimit, PackLintSeverity::Warning).is_some());
}

#[test]
fn non_kebab_pack_name_is_warning() {
    let tmp = tempdir().unwrap();
    write(
        &tmp.path().join("pack.toml"),
        "[pack]\nname='BadName'\nversion='1.0.0'\n\n[[verticals]]\nid='x'\npath='.'\n",
    );
    write(&tmp.path().join("README.md"), "# x\n");
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::KebabCase, PackLintSeverity::Warning).is_some());
}

#[test]
fn non_semver_version_is_error() {
    let tmp = tempdir().unwrap();
    write(
        &tmp.path().join("pack.toml"),
        "[pack]\nname='good-pack'\nversion='not-a-version'\n\n[[verticals]]\nid='x'\npath='.'\n",
    );
    write(&tmp.path().join("README.md"), "# x\n");
    let report = lint_pack(tmp.path());
    assert!(find(&report, CheckId::Semver, PackLintSeverity::Error).is_some());
}

#[test]
fn shipping_pack_lints_clean() {
    // Sample one of the in-tree fixtures from `packs/` — it should
    // produce zero errors. Warnings may still surface (oversized
    // README, etc.) — keep the assertion narrow to errors.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("packs/marketing-intern");
    if !p.is_dir() {
        // Not running from the workspace — skip silently.
        return;
    }
    let report = lint_pack(&p);
    assert_eq!(
        report.errors(),
        0,
        "expected clean shipping pack, got errors: {:#?}",
        report.findings
    );
}
