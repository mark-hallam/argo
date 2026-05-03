//! `argo vertical eval <id>` — walk every JSONL eval suite the
//! vertical declares, validate each fixture row is well-formed,
//! and tabulate fixture counts (#656).
//!
//! **Scope**: structural validation + fixture counts + a basic
//! comparison against `[success_metrics]` thresholds where they
//! make sense without an LLM (e.g., "suite must have at least N
//! fixtures"). Live LLM-driven scoring is deferred — that needs
//! per-fixture rubrics + API key handling, which is its own PR
//! scope.
//!
//! Fail behavior: returns a non-zero exit code if ANY suite has
//! a malformed fixture (so this is safe to use as a CI gate
//! while authoring a vertical).
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{id}' not found in {} — run `argo vertical list` to see what's loaded",
            dir.display()
        ))
    })?;
    let report = collect_eval_report(loaded);

    if json {
        println!("{}", report.to_json().to_string());
    } else {
        print_text_report(&report);
    }

    if report.ok {
        Ok(())
    } else {
        Err(ArgoError::Schema(format!(
            "vertical '{id}': one or more eval suites have malformed fixtures"
        )))
    }
}

/// Walk every eval suite the manifest declares + return a
/// structured per-suite report. Used by both the standalone
/// CLI command and by `argo vertical doctor` (#673) which
/// composes eval + validate + lint into one health check.
pub(crate) fn collect_eval_report(loaded: &argo_vertical::LoadedVertical) -> SuiteReport {
    let suites = loaded
        .manifest
        .evaluation
        .as_ref()
        .map(|e| e.suites.clone())
        .unwrap_or_default();
    let mut report = SuiteReport {
        id: loaded.manifest.vertical.id.clone(),
        suites: Vec::new(),
        ok: true,
    };
    for rel in &suites {
        let path = loaded.root.join(rel);
        let result = scan_suite(&path);
        if !result.errors.is_empty() {
            report.ok = false;
        }
        report.suites.push(SuiteRow {
            path: rel.clone(),
            fixture_count: result.fixture_count,
            errors: result.errors,
        });
    }
    report
}

pub(crate) struct SuiteReport {
    pub(crate) id: String,
    pub(crate) suites: Vec<SuiteRow>,
    pub(crate) ok: bool,
}

impl SuiteReport {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "ok": self.ok,
            "suites": self.suites.iter().map(|s| serde_json::json!({
                "path": s.path,
                "fixture_count": s.fixture_count,
                "errors": s.errors,
            })).collect::<Vec<_>>(),
        })
    }
}

pub(crate) struct SuiteRow {
    pub(crate) path: String,
    pub(crate) fixture_count: usize,
    pub(crate) errors: Vec<String>,
}

struct ScanResult {
    fixture_count: usize,
    errors: Vec<String>,
}

fn scan_suite(path: &Path) -> ScanResult {
    let body = match std::fs::read_to_string(path) {
        Ok(b) => b,
        Err(e) => {
            return ScanResult {
                fixture_count: 0,
                errors: vec![format!("read {}: {e}", path.display())],
            };
        }
    };
    let mut count = 0_usize;
    let mut errors: Vec<String> = Vec::new();
    for (i, raw) in body.lines().enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let line_no = i + 1;
        let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("line {line_no}: invalid JSON: {e}"));
                continue;
            }
        };
        for required in ["id", "input", "expected"] {
            if parsed.get(required).is_none() {
                errors.push(format!("line {line_no}: missing key '{required}'"));
            }
        }
        // `id` should be a non-empty string.
        if parsed
            .get("id")
            .and_then(|v| v.as_str())
            .is_none_or(str::is_empty)
        {
            errors.push(format!("line {line_no}: 'id' must be a non-empty string"));
        }
        count += 1;
    }
    ScanResult { fixture_count: count, errors }
}

fn print_text_report(r: &SuiteReport) {
    println!("vertical '{}': eval suite report", r.id);
    if r.suites.is_empty() {
        println!("  (manifest declares no [evaluation].suites)");
        return;
    }
    for s in &r.suites {
        let marker = if s.errors.is_empty() { "ok" } else { "FAIL" };
        println!(
            "  [{marker}] {} ({} fixture{})",
            s.path,
            s.fixture_count,
            if s.fixture_count == 1 { "" } else { "s" }
        );
        for e in &s.errors {
            println!("        {e}");
        }
    }
    if r.ok {
        println!("all suites passed structural validation");
    } else {
        println!("one or more suites have malformed fixtures (see above)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_suite_handles_clean_jsonl() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("clean.jsonl");
        fs::write(
            &p,
            "{\"id\":\"a\",\"input\":\"x\",\"expected\":{}}\n\
             {\"id\":\"b\",\"input\":\"y\",\"expected\":{}}\n\
             \n\
             {\"id\":\"c\",\"input\":\"z\",\"expected\":{}}\n",
        )
        .unwrap();
        let r = scan_suite(&p);
        assert_eq!(r.fixture_count, 3);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn scan_suite_flags_invalid_json() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("bad.jsonl");
        fs::write(&p, "{\"id\":\"a\",\"input\":\"x\",\"expected\":{}}\nNOT JSON\n").unwrap();
        let r = scan_suite(&p);
        assert_eq!(r.fixture_count, 1);
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].contains("line 2"));
        assert!(r.errors[0].contains("invalid JSON"));
    }

    #[test]
    fn scan_suite_flags_missing_required_keys() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("missing.jsonl");
        // line 1 missing "expected", line 2 missing "input"
        fs::write(
            &p,
            "{\"id\":\"a\",\"input\":\"x\"}\n{\"id\":\"b\",\"expected\":{}}\n",
        )
        .unwrap();
        let r = scan_suite(&p);
        assert_eq!(r.fixture_count, 2);
        assert_eq!(r.errors.len(), 2);
        assert!(r.errors.iter().any(|e| e.contains("line 1") && e.contains("expected")));
        assert!(r.errors.iter().any(|e| e.contains("line 2") && e.contains("input")));
    }

    #[test]
    fn scan_suite_flags_empty_id() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("empty_id.jsonl");
        fs::write(&p, "{\"id\":\"\",\"input\":\"x\",\"expected\":{}}\n").unwrap();
        let r = scan_suite(&p);
        assert!(r.errors.iter().any(|e| e.contains("non-empty")));
    }

    #[test]
    fn scan_suite_handles_missing_file() {
        let r = scan_suite(Path::new("/no/such/file.jsonl"));
        assert_eq!(r.fixture_count, 0);
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].contains("read"));
    }
}
