//! `argo doctor fix` — bounded remediation for doctor findings.
//!
//! Sibling modules:
//! - [`doctor_fix_safe`][s] — safe checks (home/manifest dirs, stale lock, policy).
//! - [`doctor_fix_confirmation`][c] — confirmation-tier (migrations, vertical fields).
//! - [`doctor_fix_unsafe`][u] — unsafe remediation hints.
//! - [`doctor_fix_audit`][a] — hash-chained audit append.
//!
//! [s]: super::doctor_fix_safe
//! [c]: super::doctor_fix_confirmation
//! [u]: super::doctor_fix_unsafe
//! [a]: super::doctor_fix_audit

use super::doctor_fix_audit::append_audit;
use super::doctor_fix_confirmation::run_confirmation;
use super::doctor_fix_safe::run_safe;
use super::doctor_fix_unsafe::run_unsafe;
use crate::home::resolve_home;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) const AUDIT_KIND: &str = "doctor_fix";
pub(super) const POLICY_RELPATH: &str = ".skycot/auto_approval.toml";
pub(super) const POLICY_TEMPLATE: &str = "enabled = false\nmax_usd_micros_per_action = 500000\ndaily_budget_usd_micros = 50000000\nwake_action_types = []\n";
pub(super) const STALE_AFTER: Duration = Duration::from_secs(300);
const SAFE: [&str; 4] = ["home_dir", "manifest_dir", "stale_lock", "workspace_policy"];
const CONFIRM: [&str; 2] = ["migrations", "vertical_required_fields"];
const UNSAFE: [&str; 3] = [
    "database_corruption",
    "missing_api_keys",
    "workspace_policy_bare",
];

#[derive(Debug, Clone)]
pub(super) struct FixContext {
    pub(super) cwd: PathBuf,
    pub(super) home: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct FixReport {
    pub(super) id: &'static str,
    pub(super) category: &'static str,
    pub(super) status: &'static str,
    pub(super) detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) remediation: Option<String>,
    pub(super) audit_written: bool,
}

#[derive(Debug, Serialize)]
struct FixEnvelope {
    ok: bool,
    auto: bool,
    issue: Option<String>,
    fixes: Vec<FixReport>,
}

pub async fn run(auto: bool, issue: Option<&str>, json_out: bool) -> ArgoResult<()> {
    let ctx = FixContext::discover()?;
    let envelope = run_with_context(&ctx, auto, issue).await?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&envelope)
                .map_err(|e| ArgoError::Provider(format!("doctor fix JSON: {e}")))?
        );
    } else {
        print_text(&envelope);
    }
    if envelope.ok {
        Ok(())
    } else {
        Err(ArgoError::Provider(
            "doctor fix did not complete cleanly".into(),
        ))
    }
}

async fn run_with_context(
    ctx: &FixContext,
    auto: bool,
    issue: Option<&str>,
) -> ArgoResult<FixEnvelope> {
    let mut fixes = Vec::new();
    for id in ids_for(issue)? {
        let mut report = run_one(ctx, id, auto).await?;
        report.audit_written = append_audit(ctx, &report, auto).await.is_ok();
        fixes.push(report);
    }
    Ok(FixEnvelope {
        ok: fixes
            .iter()
            .all(|r| !matches!(r.status, "blocked" | "unsafe")),
        auto,
        issue: issue.map(str::to_string),
        fixes,
    })
}

async fn run_one(ctx: &FixContext, id: &'static str, auto: bool) -> ArgoResult<FixReport> {
    match kind_for(id)? {
        "safe" => run_safe(ctx, id),
        "confirmation" if auto => run_confirmation(ctx, id).await,
        "confirmation" => Ok(FixReport::blocked(
            id,
            "confirmation",
            "requires --auto before applying confirmation-level remediation",
            "rerun with `argo doctor fix --auto --issue <id>` after reviewing the fix",
        )),
        "unsafe" => Ok(run_unsafe(ctx, id)),
        other => Err(ArgoError::Schema(format!("unknown fix category {other}"))),
    }
}

fn ids_for(issue: Option<&str>) -> ArgoResult<Vec<&'static str>> {
    issue.map_or_else(|| Ok(SAFE.to_vec()), |raw| Ok(vec![canonical_id(raw)?]))
}

fn canonical_id(raw: &str) -> ArgoResult<&'static str> {
    SAFE.iter()
        .chain(CONFIRM.iter())
        .chain(UNSAFE.iter())
        .copied()
        .find(|id| *id == raw)
        .ok_or_else(|| ArgoError::Schema(format!("unknown doctor fix issue '{raw}'")))
}

fn kind_for(id: &str) -> ArgoResult<&'static str> {
    if SAFE.contains(&id) {
        Ok("safe")
    } else if CONFIRM.contains(&id) {
        Ok("confirmation")
    } else if UNSAFE.contains(&id) {
        Ok("unsafe")
    } else {
        Err(ArgoError::Schema(format!(
            "unknown doctor fix issue '{id}'"
        )))
    }
}

fn print_text(envelope: &FixEnvelope) {
    println!(
        "argo doctor fix - {}",
        if envelope.ok { "ok" } else { "needs attention" }
    );
    for fix in &envelope.fixes {
        println!("  {} {}: {}", fix.status, fix.id, fix.detail);
        if let Some(remediation) = &fix.remediation {
            println!("    remediation: {remediation}");
        }
    }
}

impl FixContext {
    fn discover() -> ArgoResult<Self> {
        let cwd =
            std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
        let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
        let home = resolve_home(&cwd).unwrap_or_else(|_| cfg.core.resolved_home());
        Ok(Self { cwd, home })
    }
}

impl FixReport {
    pub(super) fn new(
        id: &'static str,
        category: &'static str,
        status: &'static str,
        detail: impl Into<String>,
        path: Option<&Path>,
        remediation: Option<String>,
    ) -> Self {
        Self {
            id,
            category,
            status,
            detail: detail.into(),
            path: path.map(|p| p.display().to_string()),
            remediation,
            audit_written: false,
        }
    }
    pub(super) fn applied(
        id: &'static str,
        category: &'static str,
        detail: impl Into<String>,
        path: &Path,
    ) -> Self {
        Self::new(id, category, "applied", detail, Some(path), None)
    }
    pub(super) fn noop(
        id: &'static str,
        category: &'static str,
        detail: impl Into<String>,
        path: &Path,
    ) -> Self {
        Self::new(id, category, "noop", detail, Some(path), None)
    }
    pub(super) fn blocked(
        id: &'static str,
        category: &'static str,
        detail: impl Into<String>,
        remediation: &str,
    ) -> Self {
        Self::new(
            id,
            category,
            "blocked",
            detail,
            None,
            Some(remediation.to_string()),
        )
    }
    pub(super) fn unsafe_issue(
        id: &'static str,
        detail: impl Into<String>,
        remediation: &str,
    ) -> Self {
        Self::new(
            id,
            "unsafe",
            "unsafe",
            detail,
            None,
            Some(remediation.to_string()),
        )
    }
    pub(super) fn with_path(mut self, path: &Path) -> Self {
        self.path = Some(path.display().to_string());
        self
    }
}

#[cfg(test)]
#[path = "doctor_fix_tests.rs"]
mod tests;
