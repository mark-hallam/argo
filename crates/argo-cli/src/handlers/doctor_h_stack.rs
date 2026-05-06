//! H-stack checks for `argo doctor`.

use argo_identity::AutoApprovalPolicy;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::Path;

const POLICY_RELPATH: &str = ".skycot/auto_approval.toml";

pub(super) async fn collect(cwd: &Path, home: &Path) -> Value {
    json!({
        "policy_file": policy_file(cwd),
        "worker_budget": table_count(home, "worker_budget").await,
        "orchestrator_persist": orchestrator_tables(home).await,
        "notify_channel_manifests": notify_manifests(cwd),
    })
}

pub(super) fn print_text(v: &Value) {
    println!("  Auto-approval stack:");
    line(v, "policy_file", "policy file");
    line(v, "worker_budget", "worker_budget table");
    line(v, "orchestrator_persist", "session + handoff tables");
    line(v, "notify_channel_manifests", "NotifyChannel manifests");
}

fn line(v: &Value, key: &str, label: &str) {
    let item = &v[key];
    let status = item["status"].as_str().unwrap_or("ERROR");
    let detail = item["detail"].as_str().unwrap_or("unavailable");
    println!("    {label}: {status} - {detail}");
}

fn policy_file(cwd: &Path) -> Value {
    let path = cwd.join(POLICY_RELPATH);
    if !path.exists() {
        return check("MISSING", "auto-approval policy file not found")
            .with("path", path.display().to_string());
    }
    let body = match std::fs::read_to_string(&path) {
        Ok(body) => body,
        Err(e) => return check("ERROR", format!("could not read policy: {e}")),
    };
    let policy: AutoApprovalPolicy = match toml::from_str(&body) {
        Ok(policy) => policy,
        Err(e) => return check("ERROR", format!("invalid policy TOML: {e}")),
    };
    let has_caps = policy.max_usd_micros_per_action.is_some()
        || policy.daily_budget_usd_micros.is_some();
    if policy.enabled && !has_caps {
        return check("WARN", "enabled=true without per-action or daily spend caps")
            .with("path", path.display().to_string())
            .with("enabled", true);
    }
    check("OK", if policy.enabled { "policy parsed and capped" } else { "policy parsed; disabled" })
        .with("path", path.display().to_string())
        .with("enabled", policy.enabled)
}

async fn table_count(home: &Path, table: &str) -> Value {
    let store = match open_store(home).await {
        Ok(store) => store,
        Err(e) => return check("ERROR", format!("store unavailable: {e}")),
    };
    let sql = format!("SELECT COUNT(*) FROM {table}");
    match sqlx::query_scalar::<_, i64>(&sql).fetch_one(store.pool()).await {
        Ok(rows) => check("OK", format!("{table} reachable ({rows} row(s))")).with("rows", rows),
        Err(e) if missing_table(&e) => check("MISSING", format!("{table} table is missing")),
        Err(e) => check("ERROR", format!("{table} query failed: {e}")),
    }
}

async fn orchestrator_tables(home: &Path) -> Value {
    let store = match open_store(home).await {
        Ok(store) => store,
        Err(e) => return check("ERROR", format!("store unavailable: {e}")),
    };
    let sessions = match count_on(&store, "session").await {
        Ok(rows) => rows,
        Err(e) if missing_table(&e) => return check("MISSING", "session table is missing"),
        Err(e) => return check("ERROR", format!("session query failed: {e}")),
    };
    let handoffs = match count_on(&store, "handoff").await {
        Ok(rows) => rows,
        Err(e) if missing_table(&e) => return check("MISSING", "handoff table is missing"),
        Err(e) => return check("ERROR", format!("handoff query failed: {e}")),
    };
    check("OK", format!("session ({sessions}) and handoff ({handoffs}) reachable"))
        .with("sessions", sessions)
        .with("handoffs", handoffs)
}

fn notify_manifests(cwd: &Path) -> Value {
    let dir = cwd.join("manifests").join("channels");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return check("MISSING", "manifests/channels directory not found"),
    };
    let mut ids = Vec::new();
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let Some(id) = channel_id(&path) else {
            return check("ERROR", format!("could not parse channel id from {}", path.display()));
        };
        ids.push(id);
        files.push(path.display().to_string());
    }
    if ids.is_empty() {
        return check("MISSING", "no channel manifests found");
    }
    ids.sort();
    check("OK", format!("configured channel ids: {}", ids.join(", ")))
        .with("channel_ids", ids)
        .with("files", files)
}

fn channel_id(path: &Path) -> Option<String> {
    let body = std::fs::read_to_string(path).ok()?;
    let parsed: toml::Value = toml::from_str(&body).ok()?;
    parsed.get("channel")?.get("id")?.as_str().map(str::to_string)
}

async fn open_store(home: &Path) -> Result<SessionStore, String> {
    let store = SessionStore::open(crate::home::db_path(home))
        .await
        .map_err(|e| e.to_string())?;
    store.migrate().await.map_err(|e| e.to_string())?;
    Ok(store)
}

async fn count_on(store: &SessionStore, table: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(store.pool())
        .await
}

fn missing_table(e: &sqlx::Error) -> bool {
    e.to_string().contains("no such table")
}

fn check(status: impl Into<String>, detail: impl Into<String>) -> Value {
    json!({"status": status.into(), "detail": detail.into()})
}

trait WithField {
    fn with(self, key: &str, value: impl serde::Serialize) -> Self;
}

impl WithField for Value {
    fn with(mut self, key: &str, value: impl serde::Serialize) -> Self {
        if let Some(obj) = self.as_object_mut() {
            obj.insert(key.to_string(), json!(value));
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn s(v: &Value) -> &str { v["status"].as_str().unwrap() }
    fn write(path: PathBuf, body: &str) { std::fs::create_dir_all(path.parent().unwrap()).unwrap(); std::fs::write(path, body).unwrap(); }
    async fn migrated(home: &Path) -> SessionStore { let store = SessionStore::open(crate::home::db_path(home)).await.unwrap(); store.migrate().await.unwrap(); store }

    #[test]
    fn policy_missing_reports_missing() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(s(&policy_file(tmp.path())), "MISSING");
    }

    #[test]
    fn bare_enabled_policy_warns() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path().join(POLICY_RELPATH), "enabled = true\n");
        assert_eq!(s(&policy_file(tmp.path())), "WARN");
    }

    #[test]
    fn capped_policy_is_ok() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path().join(POLICY_RELPATH), "enabled = true\nmax_usd_micros_per_action = 100\n");
        assert_eq!(s(&policy_file(tmp.path())), "OK");
    }

    #[tokio::test]
    async fn worker_budget_missing_reports_missing() {
        let home = TempDir::new().unwrap();
        migrated(home.path()).await;
        assert_eq!(s(&table_count(home.path(), "worker_budget").await), "MISSING");
    }

    #[tokio::test]
    async fn worker_budget_ok_counts_rows() {
        let home = TempDir::new().unwrap();
        let store = migrated(home.path()).await;
        sqlx::query("CREATE TABLE worker_budget (tenant_id BLOB, worker_id TEXT, action_id TEXT, usd_micros INTEGER, charged_at INTEGER)").execute(store.pool()).await.unwrap();
        sqlx::query("INSERT INTO worker_budget VALUES (X'00', 'w', 'a', 1, 1)").execute(store.pool()).await.unwrap();
        let v = table_count(home.path(), "worker_budget").await;
        assert_eq!(s(&v), "OK");
        assert_eq!(v["rows"].as_i64(), Some(1));
    }

    #[tokio::test]
    async fn orchestrator_missing_handoff_reports_missing() {
        let home = TempDir::new().unwrap();
        migrated(home.path()).await;
        assert_eq!(s(&orchestrator_tables(home.path()).await), "MISSING");
    }

    #[tokio::test]
    async fn orchestrator_tables_ok() {
        let home = TempDir::new().unwrap();
        let store = migrated(home.path()).await;
        sqlx::query("CREATE TABLE handoff (id TEXT PRIMARY KEY)").execute(store.pool()).await.unwrap();
        assert_eq!(s(&orchestrator_tables(home.path()).await), "OK");
    }

    #[test]
    fn notify_manifests_missing_reports_missing() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(s(&notify_manifests(tmp.path())), "MISSING");
    }

    #[test]
    fn notify_manifests_ok_lists_ids() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path().join("manifests/channels/email.toml"), "[channel]\nid = \"email\"\n");
        let v = notify_manifests(tmp.path());
        assert_eq!(s(&v), "OK");
        assert_eq!(v["channel_ids"][0].as_str(), Some("email"));
    }
}
