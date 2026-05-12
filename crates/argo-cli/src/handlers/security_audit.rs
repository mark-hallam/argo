//! `argo security audit` — wrap `cargo audit` + `cargo deny check
//! advisories` (Mission `mission-vuln-management-workflow`). Shells
//! out to the existing tooling, merges their JSON outputs into a
//! single envelope, applies a `--severity-floor` filter, prints
//! the envelope to stdout, and exits 0 / 1 / 2 per
//! `docs/skycot/security/vuln-management.md` §4.2.

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Command;

use crate::cli::cli_security_cmds::SecurityCmd;

const SEVERITIES: [&str; 4] = ["low", "medium", "high", "critical"];

/// One row in the merged envelope's `vulnerabilities[]` field.
/// Mirrors the schema in §4.1 of the workflow doc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vuln {
    pub advisory_id: String,
    pub severity: String,
    pub affected_crate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_in: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mitigation_note: Option<String>,
}

/// The full merged JSON envelope written to stdout.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Envelope {
    pub vulnerabilities: Vec<Vuln>,
    pub total_count_by_severity: BTreeMap<String, u32>,
}

pub async fn run(cmd: SecurityCmd) -> ArgoResult<()> {
    match cmd {
        SecurityCmd::Audit { severity_floor, compact } => {
            run_audit(&severity_floor, compact)
        }
        SecurityCmd::RotateKeys { .. } => unreachable!(
            "security rotate-keys is dispatched separately from main.rs"
        ),
    }
}

fn run_audit(floor: &str, compact: bool) -> ArgoResult<()> {
    let floor_rank = severity_rank(floor).ok_or_else(|| {
        ArgoError::Schema(format!(
            "unknown severity-floor '{floor}'; expected one of {SEVERITIES:?}"
        ))
    })?;

    let audit_raw = match invoke_cargo(&["audit", "--json"]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("argo: cargo audit failed: {e}");
            std::process::exit(2);
        }
    };
    let deny_raw = match invoke_cargo(&["deny", "check", "advisories", "--format", "json"]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("argo: cargo deny failed: {e}");
            std::process::exit(2);
        }
    };

    let mut all = parse_cargo_audit(&audit_raw)?;
    all.extend(parse_cargo_deny(&deny_raw)?);
    let merged = dedup_by_advisory(all);
    let totals = count_by_severity(&merged);

    let filtered: Vec<Vuln> = merged
        .into_iter()
        .filter(|v| severity_rank(&v.severity).is_some_and(|r| r >= floor_rank))
        .collect();
    let unmitigated_above_floor = filtered.iter().any(|v| v.status == "unmitigated");

    let env = Envelope { vulnerabilities: filtered, total_count_by_severity: totals };
    let rendered = if compact {
        serde_json::to_string(&env)
    } else {
        serde_json::to_string_pretty(&env)
    }
    .map_err(|e| ArgoError::Schema(format!("render envelope: {e}")))?;
    println!("{rendered}");

    if unmitigated_above_floor {
        std::process::exit(1);
    }
    Ok(())
}

fn severity_rank(s: &str) -> Option<usize> {
    SEVERITIES.iter().position(|sev| sev.eq_ignore_ascii_case(s))
}

/// Shell out to `cargo <args...>` and return UTF-8 stdout. We treat
/// a non-zero exit status with non-empty stdout as recoverable
/// (both `cargo audit` and `cargo deny` exit non-zero whenever they
/// find advisories — that's their happy path, not a tooling
/// failure). Only an entirely empty stdout with a non-zero exit is
/// flagged as a tooling error.
fn invoke_cargo(args: &[&str]) -> Result<String, String> {
    let out = Command::new("cargo")
        .args(args)
        .output()
        .map_err(|e| format!("spawn cargo {}: {e}", args.join(" ")))?;
    if out.stdout.is_empty() && !out.status.success() {
        return Err(format!(
            "cargo {} exit {}: {}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("utf8: {e}"))
}

// ---------- cargo audit JSON shape -------------------------------

#[derive(Debug, Default, Deserialize)]
struct CargoAuditReport {
    #[serde(default)]
    vulnerabilities: CargoAuditVulnsBlock,
}

#[derive(Debug, Default, Deserialize)]
struct CargoAuditVulnsBlock {
    #[serde(default)]
    list: Vec<CargoAuditVuln>,
}

#[derive(Debug, Deserialize)]
struct CargoAuditVuln {
    advisory: CargoAuditAdvisory,
    #[serde(default)]
    package: CargoAuditPkg,
    #[serde(default)]
    versions: CargoAuditVersions,
}

#[derive(Debug, Deserialize)]
struct CargoAuditAdvisory {
    id: String,
    #[serde(default)]
    severity: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CargoAuditPkg {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Default, Deserialize)]
struct CargoAuditVersions {
    #[serde(default)]
    patched: Vec<String>,
}

/// Parse the JSON produced by `cargo audit --json`. Empty / blank
/// input maps to no rows.
pub fn parse_cargo_audit(json: &str) -> ArgoResult<Vec<Vuln>> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let report: CargoAuditReport = serde_json::from_str(trimmed)
        .map_err(|e| ArgoError::Schema(format!("cargo audit JSON: {e}")))?;
    Ok(report
        .vulnerabilities
        .list
        .into_iter()
        .map(|v| Vuln {
            advisory_id: v.advisory.id,
            severity: v
                .advisory
                .severity
                .unwrap_or_else(|| "unknown".into())
                .to_ascii_lowercase(),
            affected_crate: v.package.name,
            fixed_in: v.versions.patched.into_iter().next(),
            status: "unmitigated".into(),
            mitigation_note: None,
        })
        .collect())
}

// ---------- cargo deny JSON shape --------------------------------

#[derive(Debug, Deserialize)]
struct CargoDenyEvent {
    #[serde(default)]
    advisory: Option<CargoDenyAdvisory>,
}

#[derive(Debug, Deserialize)]
struct CargoDenyAdvisory {
    id: String,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    package: Option<CargoDenyPkg>,
    #[serde(default)]
    patched: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoDenyPkg {
    #[serde(default)]
    name: String,
}

/// Parse the JSON produced by `cargo deny check advisories
/// --format json`. Accepts both the array form (`[{...}, {...}]`)
/// and the NDJSON form (one object per line) that newer cargo-deny
/// versions emit; older versions write a top-level array, while
/// the streaming form is newline-delimited.
pub fn parse_cargo_deny(json: &str) -> ArgoResult<Vec<Vuln>> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    if trimmed.starts_with('[') {
        let events: Vec<CargoDenyEvent> = serde_json::from_str(trimmed)
            .map_err(|e| ArgoError::Schema(format!("cargo deny JSON: {e}")))?;
        for ev in events {
            if let Some(adv) = ev.advisory {
                out.push(deny_to_vuln(adv));
            }
        }
    } else {
        for line in trimmed.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let ev: CargoDenyEvent = serde_json::from_str(line)
                .map_err(|e| ArgoError::Schema(format!("cargo deny NDJSON line: {e}")))?;
            if let Some(adv) = ev.advisory {
                out.push(deny_to_vuln(adv));
            }
        }
    }
    Ok(out)
}

fn deny_to_vuln(adv: CargoDenyAdvisory) -> Vuln {
    Vuln {
        advisory_id: adv.id,
        severity: adv
            .severity
            .unwrap_or_else(|| "unknown".into())
            .to_ascii_lowercase(),
        affected_crate: adv.package.map(|p| p.name).unwrap_or_default(),
        fixed_in: adv.patched,
        status: "unmitigated".into(),
        mitigation_note: None,
    }
}

/// Drop duplicate advisory ids; first occurrence wins. Both
/// upstream feeds may surface the same `RUSTSEC-YYYY-NNNN` ID, so
/// the merged envelope needs a single row per advisory.
pub fn dedup_by_advisory(vulns: Vec<Vuln>) -> Vec<Vuln> {
    let mut seen: BTreeMap<String, Vuln> = BTreeMap::new();
    for v in vulns {
        seen.entry(v.advisory_id.clone()).or_insert(v);
    }
    seen.into_values().collect()
}

/// Emit `{critical, high, medium, low, unknown}` with zero defaults
/// so downstream consumers don't have to handle missing keys.
pub fn count_by_severity(vulns: &[Vuln]) -> BTreeMap<String, u32> {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for sev in SEVERITIES {
        counts.insert(sev.to_string(), 0);
    }
    counts.insert("unknown".into(), 0);
    for v in vulns {
        *counts.entry(v.severity.clone()).or_insert(0) += 1;
    }
    counts
}
