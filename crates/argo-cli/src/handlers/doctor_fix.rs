//! `argo doctor fix` — bounded remediation for doctor findings.

use crate::home::{db_path, resolve_home};
use argo_audit::compute_chain_hash;
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const AUDIT_KIND: &str = "doctor_fix";
const POLICY_RELPATH: &str = ".skycot/auto_approval.toml";
const POLICY_TEMPLATE: &str = "enabled = false\nmax_usd_micros_per_action = 500000\ndaily_budget_usd_micros = 50000000\nwake_action_types = []\n";
const STALE_AFTER: Duration = Duration::from_secs(300);
const SAFE: [&str; 4] = ["home_dir", "manifest_dir", "stale_lock", "workspace_policy"];
const CONFIRM: [&str; 2] = ["migrations", "vertical_required_fields"];
const UNSAFE: [&str; 3] = ["database_corruption", "missing_api_keys", "workspace_policy_bare"];

#[derive(Debug, Clone)]
struct FixContext { cwd: PathBuf, home: PathBuf }

#[derive(Debug, Clone, Serialize)]
struct FixReport {
    id: &'static str,
    category: &'static str,
    status: &'static str,
    detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remediation: Option<String>,
    audit_written: bool,
}

#[derive(Debug, Serialize)]
struct FixEnvelope { ok: bool, auto: bool, issue: Option<String>, fixes: Vec<FixReport> }

pub async fn run(auto: bool, issue: Option<&str>, json_out: bool) -> ArgoResult<()> {
    let ctx = FixContext::discover()?;
    let envelope = run_with_context(&ctx, auto, issue).await?;
    if json_out { println!("{}", serde_json::to_string(&envelope).map_err(|e| ArgoError::Provider(format!("doctor fix JSON: {e}")))?); }
    else { print_text(&envelope); }
    if envelope.ok { Ok(()) } else { Err(ArgoError::Provider("doctor fix did not complete cleanly".into())) }
}

async fn run_with_context(ctx: &FixContext, auto: bool, issue: Option<&str>) -> ArgoResult<FixEnvelope> {
    let mut fixes = Vec::new();
    for id in ids_for(issue)? {
        let mut report = run_one(ctx, id, auto).await?;
        report.audit_written = append_audit(ctx, &report, auto).await.is_ok();
        fixes.push(report);
    }
    Ok(FixEnvelope { ok: fixes.iter().all(|r| !matches!(r.status, "blocked" | "unsafe")), auto, issue: issue.map(str::to_string), fixes })
}

async fn run_one(ctx: &FixContext, id: &'static str, auto: bool) -> ArgoResult<FixReport> {
    match kind_for(id)? {
        "safe" => run_safe(ctx, id),
        "confirmation" if auto => run_confirmation(ctx, id).await,
        "confirmation" => Ok(FixReport::blocked(id, "confirmation", "requires --auto before applying confirmation-level remediation", "rerun with `argo doctor fix --auto --issue <id>` after reviewing the fix")),
        "unsafe" => Ok(run_unsafe(ctx, id)),
        other => Err(ArgoError::Schema(format!("unknown fix category {other}"))),
    }
}

fn run_safe(ctx: &FixContext, id: &'static str) -> ArgoResult<FixReport> {
    match id {
        "home_dir" => ensure_dir(id, "safe", &ctx.home, "created ARGO_HOME directory"),
        "manifest_dir" => ensure_dir(id, "safe", &ctx.cwd.join(".argo/manifests"), "created workspace manifest directory"),
        "stale_lock" => remove_stale_lock(ctx),
        "workspace_policy" => ensure_policy(ctx),
        _ => Err(ArgoError::Schema(format!("unknown safe fix {id}"))),
    }
}

async fn run_confirmation(ctx: &FixContext, id: &'static str) -> ArgoResult<FixReport> {
    match id {
        "migrations" => run_migrations(ctx).await,
        "vertical_required_fields" => patch_verticals(ctx),
        _ => Err(ArgoError::Schema(format!("unknown confirmation fix {id}"))),
    }
}

fn run_unsafe(ctx: &FixContext, id: &'static str) -> FixReport {
    match id {
        "database_corruption" => FixReport::unsafe_issue(id, format!("database at {} requires manual recovery", db_path(&ctx.home).display()), "restore sessions.db from backup, or move it aside and re-run setup"),
        "missing_api_keys" => FixReport::unsafe_issue(id, "missing API keys require operator-supplied secrets", "set the missing keys in the operator environment or credential store"),
        "workspace_policy_bare" => FixReport::unsafe_issue(id, "workspace policy bare-mode enabled", "disable enabled=true or add per-action/daily spend caps").with_path(&ctx.cwd.join(POLICY_RELPATH)),
        _ => FixReport::blocked(id, "unsafe", "unknown unsafe fix", "choose a known issue id"),
    }
}

fn ensure_dir(id: &'static str, category: &'static str, path: &Path, detail: &str) -> ArgoResult<FixReport> {
    if path.is_dir() { return Ok(FixReport::noop(id, category, format!("{} already exists", path.display()), path)); }
    if path.exists() { return Ok(FixReport::blocked(id, category, format!("{} exists but is not a directory", path.display()), "move the file aside and rerun doctor fix").with_path(path)); }
    std::fs::create_dir_all(path).map_err(|e| ArgoError::Provider(format!("create {}: {e}", path.display())))?;
    Ok(FixReport::applied(id, category, detail, path))
}

fn remove_stale_lock(ctx: &FixContext) -> ArgoResult<FixReport> {
    let path = ctx.cwd.join("cron/.tick.lock");
    if !path.exists() { return Ok(FixReport::noop("stale_lock", "safe", "cron lock file is absent", &path)); }
    let stale = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|age| age > STALE_AFTER);
    if !stale { return Ok(FixReport::noop("stale_lock", "safe", "cron lock file is not stale", &path)); }
    std::fs::remove_file(&path).map_err(|e| ArgoError::Provider(format!("remove {}: {e}", path.display())))?;
    Ok(FixReport::applied("stale_lock", "safe", "removed stale cron lock file", &path))
}

fn ensure_policy(ctx: &FixContext) -> ArgoResult<FixReport> {
    let path = ctx.cwd.join(POLICY_RELPATH);
    if path.exists() { return Ok(FixReport::noop("workspace_policy", "safe", "workspace policy already exists", &path)); }
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| ArgoError::Provider(format!("create {}: {e}", parent.display())))?; }
    std::fs::write(&path, POLICY_TEMPLATE).map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))?;
    Ok(FixReport::applied("workspace_policy", "safe", "wrote default disabled workspace policy", &path))
}

async fn run_migrations(ctx: &FixContext) -> ArgoResult<FixReport> {
    std::fs::create_dir_all(&ctx.home).map_err(|e| ArgoError::Provider(format!("create {}: {e}", ctx.home.display())))?;
    let db = db_path(&ctx.home);
    let existed = db.exists();
    let store = SessionStore::open(&db).await?;
    let before = store.schema_version().await.unwrap_or(0);
    store.migrate().await?;
    let after = store.schema_version().await.unwrap_or(before);
    Ok(FixReport::new("migrations", "confirmation", if !existed || after > before { "applied" } else { "noop" }, format!("schema version {before} -> {after}"), Some(&db), None))
}

fn patch_verticals(ctx: &FixContext) -> ArgoResult<FixReport> {
    let dir = ctx.cwd.join("verticals");
    if !dir.exists() { return Ok(FixReport::noop("vertical_required_fields", "confirmation", "verticals directory is absent", &dir)); }
    let mut patched = 0_u64;
    for entry in std::fs::read_dir(&dir).map_err(|e| ArgoError::Provider(format!("read {}: {e}", dir.display())))? {
        let entry = entry.map_err(|e| ArgoError::Provider(format!("read vertical entry: {e}")))?;
        let manifest = entry.path().join("vertical.toml");
        if entry.path().is_dir() && manifest.exists() && patch_one_vertical(&manifest, &entry.file_name().to_string_lossy())? { patched += 1; }
    }
    let status = if patched == 0 { "noop" } else { "applied" };
    let detail = if patched == 0 { "no vertical manifests needed required-field patches".to_string() } else { format!("patched {patched} vertical manifest(s) with required fields") };
    Ok(FixReport::new("vertical_required_fields", "confirmation", status, detail, Some(&dir), None))
}

fn patch_one_vertical(path: &Path, fallback_id: &str) -> ArgoResult<bool> {
    let body = std::fs::read_to_string(path).map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let mut value: toml::Value = toml::from_str(&body).map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    let root = value.as_table_mut().ok_or_else(|| ArgoError::Provider(format!("{} is not a TOML table", path.display())))?;
    let mut changed = insert_string(root, "schema_version", "1");
    let vertical = root.entry("vertical".to_string()).or_insert_with(|| toml::Value::Table(toml::value::Table::new())).as_table_mut().ok_or_else(|| ArgoError::Provider(format!("{} [vertical] is not a table", path.display())))?;
    let id = vertical.get("id").and_then(toml::Value::as_str).filter(|s| !s.trim().is_empty()).unwrap_or(fallback_id).to_string();
    changed |= insert_string(vertical, "id", &id);
    changed |= insert_string(vertical, "name", &id.replace('-', " "));
    changed |= insert_string(vertical, "version", "0.1.0");
    if changed {
        let out = toml::to_string_pretty(&value).map_err(|e| ArgoError::Provider(format!("serialize {}: {e}", path.display())))?;
        std::fs::write(path, out).map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))?;
    }
    Ok(changed)
}

fn insert_string(table: &mut toml::value::Table, key: &str, value: &str) -> bool {
    if table.get(key).and_then(toml::Value::as_str).is_some_and(|s| !s.trim().is_empty()) { return false; }
    table.insert(key.to_string(), toml::Value::String(value.to_string()));
    true
}

async fn append_audit(ctx: &FixContext, report: &FixReport, auto: bool) -> ArgoResult<()> {
    std::fs::create_dir_all(&ctx.home).map_err(|e| ArgoError::Provider(format!("create {}: {e}", ctx.home.display())))?;
    let store = SessionStore::open(db_path(&ctx.home)).await?;
    store.migrate().await?;
    let parent = last_global_hash(&store).await?;
    let payload = json!({"issue_id": report.id, "category": report.category, "status": report.status, "detail": report.detail, "path": report.path, "remediation": report.remediation, "auto": auto});
    let canonical = serde_json::to_string(&payload).map_err(|e| ArgoError::Provider(format!("doctor fix audit JSON: {e}")))?;
    let self_hash = compute_chain_hash(&parent, AUDIT_KIND, &canonical);
    sqlx::query("INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) VALUES (?, NULL, ?, ?, ?, ?, ?)")
        .bind(Uuid::now_v7().as_bytes().as_slice()).bind(AUDIT_KIND).bind(canonical)
        .bind(parent.as_slice()).bind(self_hash.as_slice()).bind(Utc::now().timestamp_micros())
        .execute(store.pool()).await.map_err(|e| ArgoError::Provider(format!("doctor fix audit: {e}")))?;
    Ok(())
}

async fn last_global_hash(store: &SessionStore) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar("SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1").fetch_optional(store.pool()).await.map_err(|e| ArgoError::Provider(format!("doctor fix last audit hash: {e}")))?;
    let mut out = [0_u8; 32];
    if let Some(bytes) = bytes { if bytes.len() == 32 { out.copy_from_slice(&bytes); } }
    Ok(out)
}

fn ids_for(issue: Option<&str>) -> ArgoResult<Vec<&'static str>> {
    issue.map_or_else(|| Ok(SAFE.to_vec()), |raw| Ok(vec![canonical_id(raw)?]))
}

fn canonical_id(raw: &str) -> ArgoResult<&'static str> {
    SAFE.iter().chain(CONFIRM.iter()).chain(UNSAFE.iter()).copied().find(|id| *id == raw).ok_or_else(|| ArgoError::Schema(format!("unknown doctor fix issue '{raw}'")))
}

fn kind_for(id: &str) -> ArgoResult<&'static str> {
    if SAFE.contains(&id) { Ok("safe") } else if CONFIRM.contains(&id) { Ok("confirmation") } else if UNSAFE.contains(&id) { Ok("unsafe") } else { Err(ArgoError::Schema(format!("unknown doctor fix issue '{id}'"))) }
}

fn print_text(envelope: &FixEnvelope) {
    println!("argo doctor fix - {}", if envelope.ok { "ok" } else { "needs attention" });
    for fix in &envelope.fixes {
        println!("  {} {}: {}", fix.status, fix.id, fix.detail);
        if let Some(remediation) = &fix.remediation { println!("    remediation: {remediation}"); }
    }
}

impl FixContext {
    fn discover() -> ArgoResult<Self> {
        let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
        let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
        let home = resolve_home(&cwd).unwrap_or_else(|_| cfg.core.resolved_home());
        Ok(Self { cwd, home })
    }
}

impl FixReport {
    fn new(id: &'static str, category: &'static str, status: &'static str, detail: impl Into<String>, path: Option<&Path>, remediation: Option<String>) -> Self {
        Self { id, category, status, detail: detail.into(), path: path.map(|p| p.display().to_string()), remediation, audit_written: false }
    }
    fn applied(id: &'static str, category: &'static str, detail: impl Into<String>, path: &Path) -> Self { Self::new(id, category, "applied", detail, Some(path), None) }
    fn noop(id: &'static str, category: &'static str, detail: impl Into<String>, path: &Path) -> Self { Self::new(id, category, "noop", detail, Some(path), None) }
    fn blocked(id: &'static str, category: &'static str, detail: impl Into<String>, remediation: &str) -> Self { Self::new(id, category, "blocked", detail, None, Some(remediation.to_string())) }
    fn unsafe_issue(id: &'static str, detail: impl Into<String>, remediation: &str) -> Self { Self::new(id, "unsafe", "unsafe", detail, None, Some(remediation.to_string())) }
    fn with_path(mut self, path: &Path) -> Self { self.path = Some(path.display().to_string()); self }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tc() -> (TempDir, TempDir, FixContext) {
        let cwd = TempDir::new().unwrap(); let home = TempDir::new().unwrap();
        let ctx = FixContext { cwd: cwd.path().to_path_buf(), home: home.path().join("argo-home") };
        (cwd, home, ctx)
    }

    async fn audits(ctx: &FixContext) -> i64 {
        let store = SessionStore::open(db_path(&ctx.home)).await.unwrap(); store.migrate().await.unwrap();
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'doctor_fix'").fetch_one(store.pool()).await.unwrap()
    }

    #[tokio::test]
    async fn safe_fixes_apply_without_auto_and_write_audit() {
        let (_cwd, _home, ctx) = tc(); let out = run_with_context(&ctx, false, None).await.unwrap();
        assert!(out.ok); assert_eq!(out.fixes.len(), 4); assert!(ctx.home.is_dir());
        assert!(ctx.cwd.join(".argo/manifests").is_dir()); assert!(ctx.cwd.join(POLICY_RELPATH).is_file());
        assert_eq!(audits(&ctx).await, 4);
    }

    #[tokio::test]
    async fn issue_filter_runs_only_one_fix() {
        let (_cwd, _home, ctx) = tc(); let out = run_with_context(&ctx, false, Some("manifest_dir")).await.unwrap();
        assert!(out.ok); assert_eq!(out.fixes[0].id, "manifest_dir"); assert!(!ctx.cwd.join(POLICY_RELPATH).exists());
        assert_eq!(audits(&ctx).await, 1);
    }

    #[tokio::test]
    async fn stale_lock_is_removed() {
        let (_cwd, _home, ctx) = tc(); let lock = ctx.cwd.join("cron/.tick.lock");
        std::fs::create_dir_all(lock.parent().unwrap()).unwrap(); std::fs::write(&lock, "old").unwrap();
        assert!(std::process::Command::new("touch").args(["-t", "202001010000"]).arg(&lock).status().unwrap().success());
        let out = run_with_context(&ctx, false, Some("stale_lock")).await.unwrap();
        assert!(out.ok); assert_eq!(out.fixes[0].status, "applied"); assert!(!lock.exists());
    }

    #[tokio::test]
    async fn confirmation_fix_requires_auto() {
        let (_cwd, _home, ctx) = tc(); let out = run_with_context(&ctx, false, Some("migrations")).await.unwrap();
        assert!(!out.ok); assert_eq!(out.fixes[0].status, "blocked"); assert_eq!(audits(&ctx).await, 1);
    }

    #[tokio::test]
    async fn vertical_required_fields_auto_patches_manifest() {
        let (_cwd, _home, ctx) = tc(); let root = ctx.cwd.join("verticals/alpha"); std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("vertical.toml"), "[vertical]\nid = \"alpha\"\n").unwrap();
        assert!(!run_with_context(&ctx, false, Some("vertical_required_fields")).await.unwrap().ok);
        let out = run_with_context(&ctx, true, Some("vertical_required_fields")).await.unwrap(); assert!(out.ok);
        let body = std::fs::read_to_string(root.join("vertical.toml")).unwrap(); assert!(body.contains("schema_version") && body.contains("version"));
    }

    #[tokio::test]
    async fn unsafe_issue_returns_remediation_and_not_ok() {
        let (_cwd, _home, ctx) = tc(); let out = run_with_context(&ctx, false, Some("database_corruption")).await.unwrap();
        assert!(!out.ok); assert_eq!(out.fixes[0].status, "unsafe"); assert!(out.fixes[0].remediation.as_deref().unwrap().contains("restore"));
    }
}
