//! `pnpm audit --json` parser + detection (mission-pnpm-audit-parser).
//! Extends `argo security audit` to the JS side: detects `package.json`
//! or `pnpm-lock.yaml` at cwd or `cwd/web/`, shells out, normalizes
//! into the same [`super::Vuln`] envelope. Silent skip when pnpm is
//! absent or output is unparseable — exit code 2 stays reserved for
//! cargo-side failures per the #1458 contract.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use argo_types::{ArgoError, ArgoResult};
use serde::Deserialize;

use super::Vuln;

#[derive(Debug, Default, Deserialize)]
struct PnpmAuditReport {
    #[serde(default)]
    advisories: BTreeMap<String, PnpmAdvisory>,
}

#[derive(Debug, Default, Deserialize)]
struct PnpmAdvisory {
    #[serde(default)] severity: Option<String>,
    #[serde(default)] module_name: String,
    #[serde(default)] patched_versions: Option<String>,
}

fn detect_js_root(cwd: &Path) -> Option<PathBuf> {
    [cwd.to_path_buf(), cwd.join("web")].into_iter().find(|c| {
        c.join("pnpm-lock.yaml").exists() || c.join("package.json").exists()
    })
}

/// Run `pnpm audit --json` if a JS package is detected and `pnpm` is
/// on PATH; otherwise return no rows. Pnpm spawn or parse failures
/// warn to stderr and return no rows (exit-2 stays reserved for cargo).
pub fn collect_pnpm(cwd: &Path) -> Vec<Vuln> {
    let Some(js_root) = detect_js_root(cwd) else { return Vec::new(); };
    if Command::new("pnpm").arg("--version").output().is_err() {
        return Vec::new();
    }
    let out = match Command::new("pnpm").args(["audit", "--json"]).current_dir(&js_root).output() {
        Ok(o) => o,
        Err(e) => { eprintln!("argo: pnpm audit spawn failed: {e}"); return Vec::new(); }
    };
    parse_pnpm_audit(&String::from_utf8_lossy(&out.stdout)).unwrap_or_else(|e| {
        eprintln!("argo: {e}");
        Vec::new()
    })
}

/// Parse `pnpm audit --json`. Empty / blank input maps to no rows.
/// The advisory id is the JSON object key under `.advisories`.
pub fn parse_pnpm_audit(json: &str) -> ArgoResult<Vec<Vuln>> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let report: PnpmAuditReport = serde_json::from_str(trimmed)
        .map_err(|e| ArgoError::Schema(format!("pnpm audit JSON: {e}")))?;
    Ok(report.advisories.into_iter().map(|(id, adv)| Vuln {
        advisory_id: id,
        severity: adv.severity.unwrap_or_else(|| "unknown".into()).to_ascii_lowercase(),
        affected_crate: adv.module_name,
        fixed_in: adv.patched_versions,
        status: "unmitigated".into(),
        mitigation_note: None,
    }).collect())
}
