//! Approval-policy template surface (Stream
//! `claude-approval-policy-templates`, V1.4).
//!
//! Operators opt into a starter set by name
//! (`template = "auto-approve-low-risk"`) or compose
//! (`extends = ["a", "b"]`). Templates compile down to the
//! existing [`AutoApprovalPolicy`] — no new evaluator. Resolution
//! merges `extends` left-to-right, then overlays the caller's
//! own keys (later wins).
//!
//! Surface: `argo approval policy {list, show <name>, init <name>}`.

use crate::cli::ApprovalPolicyCmd;
use argo_identity::AutoApprovalPolicy;
use argo_types::{ArgoError, ArgoResult};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use toml::value::Table;

/// Bundled starter templates, embedded at compile time so the
/// binary is self-contained. Order is the canonical `list` order.
pub(crate) const BUNDLED_TEMPLATES: &[(&str, &str)] = &[
    (
        "auto-approve-low-risk",
        include_str!("../../assets/approval_policies/auto-approve-low-risk.toml"),
    ),
    (
        "require-human-for-finance",
        include_str!("../../assets/approval_policies/require-human-for-finance.toml"),
    ),
    (
        "deny-network-egress",
        include_str!("../../assets/approval_policies/deny-network-egress.toml"),
    ),
    (
        "read-only-developer",
        include_str!("../../assets/approval_policies/read-only-developer.toml"),
    ),
    (
        "strict-production",
        include_str!("../../assets/approval_policies/strict-production.toml"),
    ),
];

/// Default per-repo template directory. Operators clone bundled
/// templates here via `argo approval policy init <name>` and edit
/// the local copy; the runtime never modifies the embedded
/// strings.
pub(crate) const LOCAL_TEMPLATE_DIR: &str = ".skycot/approval-policies";

/// Maximum depth of `extends` chain before we abort with an
/// error. Catches accidental cycles (`a` extends `b` extends `a`)
/// without needing a full DAG visitor.
const MAX_EXTENDS_DEPTH: usize = 8;

/// Extracts the `template` / `extends` references from a TOML
/// body without disturbing the rest of the parser. Both keys
/// default to absent so bodies without either still parse.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct PolicyTemplateRefs {
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub extends: Vec<String>,
}

impl PolicyTemplateRefs {
    /// Flatten `template` + `extends` into one ordered list. The
    /// shorthand is appended FIRST so a caller mixing both forms
    /// still gets predictable left-to-right semantics.
    fn into_chain(self) -> Vec<String> {
        let mut chain = Vec::with_capacity(self.extends.len() + 1);
        if let Some(name) = self.template {
            chain.push(name);
        }
        chain.extend(self.extends);
        chain
    }
}

pub(crate) async fn run(cmd: ApprovalPolicyCmd) -> ArgoResult<()> {
    match cmd {
        ApprovalPolicyCmd::List { json } => list(json),
        ApprovalPolicyCmd::Show { name, json } => show(&name, json),
        ApprovalPolicyCmd::Init { name, force, json } => init(&name, force, json),
    }
}

fn list(json: bool) -> ArgoResult<()> {
    let bundled: Vec<&str> = BUNDLED_TEMPLATES.iter().map(|(n, _)| *n).collect();
    let local = list_local()?;
    if json {
        let payload = serde_json::json!({
            "bundled": bundled,
            "local": local.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
            "local_dir": LOCAL_TEMPLATE_DIR,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    println!("bundled approval policy templates ({}):", bundled.len());
    for name in &bundled {
        println!("  {name}");
    }
    println!();
    println!(
        "local templates in {LOCAL_TEMPLATE_DIR} ({}):",
        local.len()
    );
    for path in &local {
        println!("  {}", path.display());
    }
    Ok(())
}

fn show(name: &str, json: bool) -> ArgoResult<()> {
    let resolved = resolve(name)?;
    let body = serialize_policy(&resolved)?;
    if json {
        let payload = serde_json::json!({
            "name": name,
            "policy": resolved,
            "toml": body,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    print!("{body}");
    Ok(())
}

fn init(name: &str, force: bool, json: bool) -> ArgoResult<()> {
    let body = bundled_body(name).ok_or_else(|| {
        ArgoError::Provider(format!(
            "no bundled approval policy template named '{name}' (try `argo approval policy list`)"
        ))
    })?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let dir = cwd.join(LOCAL_TEMPLATE_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| {
        ArgoError::Provider(format!("create {}: {e}", dir.display()))
    })?;
    let dest = dir.join(format!("{name}.toml"));
    if dest.exists() && !force {
        return Err(ArgoError::Provider(format!(
            "{} already exists (pass --force to overwrite)",
            dest.display()
        )));
    }
    std::fs::write(&dest, body).map_err(|e| {
        ArgoError::Provider(format!("write {}: {e}", dest.display()))
    })?;
    if json {
        let payload = serde_json::json!({
            "name": name,
            "path": dest.display().to_string(),
            "bytes": body.len(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("wrote {} ({} bytes)", dest.display(), body.len());
    }
    Ok(())
}

/// Resolve a template name into a fully-merged
/// [`AutoApprovalPolicy`]. Walks `extends` left-to-right, then
/// strips `template`/`extends` from the final body.
pub(crate) fn resolve(name: &str) -> ArgoResult<AutoApprovalPolicy> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    resolve_at(name, &cwd)
}

pub(crate) fn resolve_at(name: &str, cwd: &Path) -> ArgoResult<AutoApprovalPolicy> {
    let merged = resolve_to_table(name, cwd, 0)?;
    table_to_policy(merged)
}

fn resolve_to_table(name: &str, cwd: &Path, depth: usize) -> ArgoResult<Table> {
    if depth > MAX_EXTENDS_DEPTH {
        return Err(ArgoError::Provider(format!(
            "approval policy '{name}' exceeds max extends depth {MAX_EXTENDS_DEPTH} (cycle?)"
        )));
    }
    let body = template_body(name, cwd)?;
    parse_and_merge_at(&body, cwd, depth)
}

// Local `.skycot/approval-policies/<name>.toml` shadows any
// bundled template with the same name, so operators can override
// a starter without renaming it.
fn template_body(name: &str, cwd: &Path) -> ArgoResult<String> {
    let local = cwd
        .join(LOCAL_TEMPLATE_DIR)
        .join(format!("{name}.toml"));
    if local.exists() {
        return std::fs::read_to_string(&local).map_err(|e| {
            ArgoError::Provider(format!("read {}: {e}", local.display()))
        });
    }
    bundled_body(name)
        .map(|s| s.to_string())
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "no approval policy template named '{name}' (bundled or local)"
            ))
        })
}

fn bundled_body(name: &str) -> Option<&'static str> {
    BUNDLED_TEMPLATES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, body)| *body)
}

/// Parse a TOML body and merge in any templates it `extends`.
/// Public-by-crate so [`crate::handlers::worker_actions_auto_approval`]
/// can layer the workspace's own `auto_approval.toml` on top of a
/// bundled chain.
pub(crate) fn parse_and_merge_at(body: &str, cwd: &Path, depth: usize) -> ArgoResult<Table> {
    let refs: PolicyTemplateRefs = toml::from_str(body)
        .map_err(|e| ArgoError::Provider(format!("approval policy refs parse: {e}")))?;
    let chain = refs.into_chain();
    let mut merged = Table::new();
    for parent in chain {
        let parent_table = resolve_to_table(&parent, cwd, depth + 1)?;
        merge_into(&mut merged, parent_table);
    }
    let mut local: Table = toml::from_str(body)
        .map_err(|e| ArgoError::Provider(format!("approval policy parse: {e}")))?;
    local.remove("template");
    local.remove("extends");
    merge_into(&mut merged, local);
    Ok(merged)
}

// Shallow-merge `overlay` on top of `base` (later wins). We do
// NOT recurse into nested tables — the `AutoApprovalPolicy`
// shape is flat and a deep merge would surprise callers who
// expect `time_windows = [...]` to fully replace, not append.
fn merge_into(base: &mut Table, overlay: Table) {
    for (k, v) in overlay {
        base.insert(k, v);
    }
}

fn table_to_policy(table: Table) -> ArgoResult<AutoApprovalPolicy> {
    let value = toml::Value::Table(table);
    value
        .try_into()
        .map_err(|e| ArgoError::Provider(format!("approval policy deserialise: {e}")))
}

fn serialize_policy(policy: &AutoApprovalPolicy) -> ArgoResult<String> {
    toml::to_string_pretty(policy)
        .map_err(|e| ArgoError::Provider(format!("approval policy serialise: {e}")))
}

fn list_local() -> ArgoResult<Vec<PathBuf>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let dir = cwd.join(LOCAL_TEMPLATE_DIR);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| {
        ArgoError::Provider(format!("read_dir {}: {e}", dir.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| {
            ArgoError::Provider(format!("read entry under {}: {e}", dir.display()))
        })?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Layer a bundled-template chain under the workspace's own
/// `auto_approval.toml` body and return the resolved policy.
/// Used by the auto-approval gate at config-load time so a config
/// that says `template = "..."` behaves as if the bundled body
/// were inlined.
pub(crate) fn resolve_workspace_body_at(body: &str, cwd: &Path) -> ArgoResult<AutoApprovalPolicy> {
    let merged = parse_and_merge_at(body, cwd, 0)?;
    table_to_policy(merged)
}

/// Cheap probe — does this body reference any templates? Lets
/// the auto-approval loader skip the resolver on the common
/// case (most workspaces have neither key).
pub(crate) fn body_uses_templates(body: &str) -> bool {
    toml::from_str::<PolicyTemplateRefs>(body)
        .ok()
        .map(|r| r.template.is_some() || !r.extends.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "approval_policy_tests.rs"]
mod tests;
