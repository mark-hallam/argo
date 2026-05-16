//! Per-check implementations for `crate::pack_lint`. Split out of
//! `pack_lint.rs` so each file stays under the 500-LOC ceiling.
//!
//! Every helper takes `&mut Vec<LintFinding>` and appends via the
//! shared [`push`] sink. Checks read from disk, never write.

use argo_types::CapabilitySet;
use argo_vertical_packs::PackManifest;
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};

use super::pack_lint::{push, CheckId, LintFinding, Severity};
use crate::ir::SkillGraph;

pub(super) fn check_pack_toml(
    root: &Path,
    findings: &mut Vec<LintFinding>,
) -> Option<PackManifest> {
    let path = root.join("pack.toml");
    if !path.is_file() {
        push(
            findings,
            CheckId::PackToml,
            Severity::Error,
            "pack.toml missing",
        );
        return None;
    }
    match PackManifest::load(root) {
        Ok(m) => Some(m),
        Err(e) => {
            push(
                findings,
                CheckId::PackToml,
                Severity::Error,
                &format!("pack.toml: {e}"),
            );
            None
        }
    }
}

pub(super) fn check_worker_toml(root: &Path, findings: &mut Vec<LintFinding>) {
    let path = root.join("worker.toml");
    if !path.is_file() {
        return; // optional in current pack format
    }
    if let Ok(body) = fs::read_to_string(&path) {
        if let Err(e) = toml::from_str::<toml::Value>(&body) {
            push(
                findings,
                CheckId::WorkerToml,
                Severity::Error,
                &format!("worker.toml parse: {e}"),
            );
        }
    } else {
        push(
            findings,
            CheckId::WorkerToml,
            Severity::Error,
            "worker.toml unreadable",
        );
    }
}

pub(super) fn check_memory_schema(root: &Path, findings: &mut Vec<LintFinding>) {
    // Real packs ship `memory/schema.json` with `collections`
    // rather than the brief's `memory_schema.json`; check both
    // canonical paths so this works in either format.
    let candidates = ["memory/schema.json", "memory_schema.json"];
    let Some(rel) = candidates.iter().find(|c| root.join(c).is_file()) else {
        return; // memory schema is optional for v1 packs
    };
    let path = root.join(rel);
    let body = match fs::read_to_string(&path) {
        Ok(b) => b,
        Err(e) => {
            push(
                findings,
                CheckId::MemorySchema,
                Severity::Error,
                &format!("{}: {e}", path.display()),
            );
            return;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            push(
                findings,
                CheckId::MemorySchema,
                Severity::Error,
                &format!("{}: {e}", path.display()),
            );
            return;
        }
    };
    if let Some(cols) = value.get("collections").and_then(|c| c.as_array()) {
        for col in cols {
            let col_name = col
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("<unnamed>");
            let Some(fields) = col.get("fields").and_then(|f| f.as_object()) else {
                continue;
            };
            for (field, ty) in fields {
                let bad = !ty.is_string() || ty.as_str().map(str::is_empty).unwrap_or(true);
                if bad {
                    push(
                        findings,
                        CheckId::MemorySchema,
                        Severity::Error,
                        &format!(
                            "{}: collection \"{col_name}\" field \"{field}\" missing type",
                            path.display()
                        ),
                    );
                }
            }
        }
        return;
    }
    if let Some(fields) = value.get("fields").and_then(|f| f.as_object()) {
        for (field, def) in fields {
            let ty = def.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if ty.is_empty() {
                push(
                    findings,
                    CheckId::MemorySchema,
                    Severity::Error,
                    &format!("{}: field \"{field}\" missing type", path.display()),
                );
            }
        }
    }
}

pub(super) fn check_skill_irs(root: &Path, findings: &mut Vec<LintFinding>) -> Vec<String> {
    let dir = root.join("skills");
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut outcome_names: Vec<String> = Vec::new();
    for entry in walk_dir(&dir) {
        if !is_skill_json(&entry) {
            continue;
        }
        let body = match fs::read_to_string(&entry) {
            Ok(b) => b,
            Err(e) => {
                push(
                    findings,
                    CheckId::SkillIrParse,
                    Severity::Error,
                    &format!("{}: {e}", entry.display()),
                );
                continue;
            }
        };
        let graph: SkillGraph = match serde_json::from_str(&body) {
            Ok(g) => g,
            Err(e) => {
                push(
                    findings,
                    CheckId::SkillIrParse,
                    Severity::Error,
                    &format!("{}: parse: {e}", entry.display()),
                );
                continue;
            }
        };
        if let Some(name) = graph.metadata.name.as_deref() {
            outcome_names.push(name.to_string());
            check_kebab_case_skill(&entry, name, findings);
        }
        check_skill_ir_tool_refs(&entry, &graph, findings);
        check_skill_capabilities(&entry, &body, findings);
    }
    outcome_names
}

fn check_skill_ir_tool_refs(path: &Path, graph: &SkillGraph, findings: &mut Vec<LintFinding>) {
    for node in &graph.nodes {
        let Some(binding) = node.annotations.tool_binding.as_ref() else {
            continue;
        };
        if binding.mcp_server.trim().is_empty() || binding.tool_name.trim().is_empty() {
            push(
                findings,
                CheckId::SkillIrToolRef,
                Severity::Warning,
                &format!(
                    "{}: node \"{}\" tool_binding has empty mcp_server or tool_name",
                    path.display(),
                    node.id.as_str()
                ),
            );
        }
    }
}

fn check_skill_capabilities(path: &Path, body: &str, findings: &mut Vec<LintFinding>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return; // SkillIrParse already reported the error
    };
    let Some(caps_value) = value.get("capabilities") else {
        return; // capabilities are optional in current IR shape
    };
    if let Err(e) = serde_json::from_value::<CapabilitySet>(caps_value.clone()) {
        push(
            findings,
            CheckId::Capabilities,
            Severity::Error,
            &format!("{}: capabilities: {e}", path.display()),
        );
    }
}

pub(super) fn check_eval_fixtures(
    root: &Path,
    skill_names: &[String],
    findings: &mut Vec<LintFinding>,
) {
    let dir = root.join("evals");
    if !dir.is_dir() {
        return;
    }
    for entry in walk_dir(&dir) {
        if entry.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let body = match fs::read_to_string(&entry) {
            Ok(b) => b,
            Err(e) => {
                push(
                    findings,
                    CheckId::EvalParse,
                    Severity::Error,
                    &format!("{}: {e}", entry.display()),
                );
                continue;
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => {
                push(
                    findings,
                    CheckId::EvalParse,
                    Severity::Error,
                    &format!("{}: parse: {e}", entry.display()),
                );
                continue;
            }
        };
        // Eval fixtures may be `[...]` (current shipping shape:
        // `evals/golden.json`) or `{ schema_version: N, cases: [] }`.
        let cases: &[serde_json::Value] = match &value {
            serde_json::Value::Array(arr) => arr.as_slice(),
            serde_json::Value::Object(map) => {
                if let Some(v) = map.get("schema_version") {
                    if v.as_u64().unwrap_or(0) < 1 {
                        push(
                            findings,
                            CheckId::EvalParse,
                            Severity::Error,
                            &format!("{}: schema_version must be >= 1", entry.display()),
                        );
                    }
                }
                map.get("cases")
                    .and_then(|c| c.as_array())
                    .map_or(&[][..], |a| a.as_slice())
            }
            _ => {
                push(
                    findings,
                    CheckId::EvalParse,
                    Severity::Error,
                    &format!("{}: top-level must be array or object", entry.display()),
                );
                continue;
            }
        };
        for case in cases {
            check_one_eval_case(&entry, case, skill_names, findings);
        }
    }
}

fn check_one_eval_case(
    entry: &Path,
    case: &serde_json::Value,
    skill_names: &[String],
    findings: &mut Vec<LintFinding>,
) {
    let id = case
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("<no-id>");
    let outcome = case
        .get("expected_outcome")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if outcome.is_empty() {
        push(
            findings,
            CheckId::EvalShape,
            Severity::Warning,
            &format!("{}: case {id} missing expected_outcome", entry.display()),
        );
    }
    if let Some(expected) = case.get("expected_skill").and_then(|v| v.as_str()) {
        if !skill_names.is_empty() && !skill_names.iter().any(|s| s == expected) {
            push(
                findings,
                CheckId::EvalShape,
                Severity::Warning,
                &format!(
                    "{}: case {id} expected_skill \"{expected}\" not in pack skills",
                    entry.display(),
                ),
            );
        }
    }
}

pub(super) fn check_readme(root: &Path, findings: &mut Vec<LintFinding>) {
    if !root.join("README.md").is_file() {
        push(
            findings,
            CheckId::ReadmePresent,
            Severity::Warning,
            "README.md missing at pack root",
        );
    }
}

pub(super) fn check_file_loc(root: &Path, findings: &mut Vec<LintFinding>) {
    for entry in walk_dir(root) {
        if !is_source_file(&entry) {
            continue;
        }
        let Ok(body) = fs::read_to_string(&entry) else {
            continue;
        };
        let lines = body.lines().count();
        if lines > 500 {
            push(
                findings,
                CheckId::FileLocLimit,
                Severity::Warning,
                &format!("{}: {lines} LOC exceeds 500-line limit", entry.display()),
            );
        }
    }
}

pub(super) fn check_kebab_case_pack(name: &str, findings: &mut Vec<LintFinding>) {
    if !is_kebab_case(name) {
        push(
            findings,
            CheckId::KebabCase,
            Severity::Warning,
            &format!("pack.name \"{name}\" is not kebab-case"),
        );
    }
}

fn check_kebab_case_skill(path: &Path, name: &str, findings: &mut Vec<LintFinding>) {
    // Skill IRs frequently use snake_case (e.g. `draft_blog`),
    // so the check accepts both and only fires on space/uppercase.
    let bad = name.chars().any(|c| c.is_whitespace() || c.is_uppercase());
    if bad {
        push(
            findings,
            CheckId::KebabCase,
            Severity::Warning,
            &format!(
                "{}: skill name \"{name}\" should be lowercase + hyphens/underscores",
                path.display()
            ),
        );
    }
}

pub(super) fn check_semver_pack(version: &str, findings: &mut Vec<LintFinding>) {
    if Version::parse(version).is_err() {
        push(
            findings,
            CheckId::Semver,
            Severity::Error,
            &format!("pack.version \"{version}\" is not valid semver"),
        );
    }
}

fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

fn is_skill_json(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".skill.json"))
        .unwrap_or(false)
}

fn is_source_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.ends_with(".lock") || name.ends_with(".tmp") || name.ends_with(".swp") {
        return false;
    }
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "json" | "toml" | "md" | "yaml" | "yml")
    )
}

fn walk_dir(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}
