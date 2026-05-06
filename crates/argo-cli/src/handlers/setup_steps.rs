//! Step collection for the guided `argo setup` wizard.
use crate::home::db_path;
use argo_store::SessionStore;
use argo_types::ArgoResult;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// One setup checklist status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step passed and contributed to readiness.
    Ok,
    /// Step failed and blocks readiness.
    Fail,
    /// Step was intentionally skipped, usually because it is optional or already complete.
    Skip,
}

/// One rendered setup checklist row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SetupStep {
    /// Stable step name.
    pub name: String,
    /// Machine-readable status.
    pub status: StepStatus,
    /// Human-facing detail or next command.
    pub detail: String,
}

impl SetupStep {
    fn ok(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: StepStatus::Ok,
            detail: detail.into(),
        }
    }

    fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: StepStatus::Fail,
            detail: detail.into(),
        }
    }

    fn skip(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: StepStatus::Skip,
            detail: detail.into(),
        }
    }
}

pub const GENESIS_STEP: &str = "Genesis archetype suggestions";

pub async fn collect_steps(cwd: &Path, home: &Path, interactive: bool) -> ArgoResult<Vec<SetupStep>> {
    let doctor = super::doctor::collect().await?;
    Ok(vec![
        doctor_step(&doctor),
        workspace_step(cwd, home),
        vertical_step(cwd),
        worker_step(home, interactive).await,
        notify_step(&doctor),
        genesis_step(),
        audit_step(home).await,
    ])
}

pub fn doctor_step(payload: &Value) -> SetupStep {
    let toml_ok = payload["argo_toml"]["ok"].as_bool().unwrap_or(false);
    let bundle_missing = payload["bundle_dirs_missing"]
        .as_array()
        .map(|v| v.len())
        .unwrap_or(0);
    let mem_missing = payload["memory_files_missing"]
        .as_array()
        .map(|v| v.len())
        .unwrap_or(0);
    if toml_ok && bundle_missing == 0 && mem_missing == 0 {
        SetupStep::skip("Doctor invariants", "argo doctor already passes")
    } else {
        SetupStep::fail(
            "Doctor invariants",
            "run `argo workspace init` then `argo doctor` to clear missing workspace files",
        )
    }
}

pub fn workspace_step(cwd: &Path, _home: &Path) -> SetupStep {
    if !cwd.exists() {
        return SetupStep::fail("Workspace manifests", format!("{} is missing", cwd.display()));
    }
    let manifests = cwd.join(".argo").join("manifests");
    match ensure_writeable_dir(&manifests) {
        Ok(()) => SetupStep::ok(
            "Workspace manifests",
            format!("{} is writeable", manifests.display()),
        ),
        Err(e) => SetupStep::fail("Workspace manifests", e),
    }
}

pub fn vertical_step(cwd: &Path) -> SetupStep {
    let dir = vertical_dir(cwd);
    match argo_vertical::load_dir(&dir) {
        Ok((registry, _errors)) if !registry.is_empty() => SetupStep::ok(
            "Vertical registry",
            format!("{} vertical(s) registered", registry.len()),
        ),
        Ok((_registry, errors)) if !errors.is_empty() => SetupStep::fail(
            "Vertical registry",
            format!("{} vertical load error(s); run `argo vertical lint`", errors.len()),
        ),
        Ok((_registry, _errors)) => SetupStep::skip(
            "Vertical registry",
            format!("no vertical.toml found under {}; run `argo genesis create`", dir.display()),
        ),
        Err(e) => SetupStep::fail("Vertical registry", format!("load verticals: {e}")),
    }
}

pub async fn worker_step(home: &Path, interactive: bool) -> SetupStep {
    match count_workers(home).await {
        Ok(n) if n > 0 => SetupStep::ok("Worker bootstrap", format!("{n} worker(s) found")),
        Ok(_) => {
            let detail = if interactive {
                "no workers found; offer: argo worker spawn --vertical <id> --persona Maya"
            } else {
                "no workers found; run: argo worker spawn --vertical <id> --persona Maya"
            };
            SetupStep::skip("Worker bootstrap", detail)
        }
        Err(e) => SetupStep::fail("Worker bootstrap", format!("worker lookup: {e}")),
    }
}

pub fn notify_step(payload: &Value) -> SetupStep {
    let Some(channels) = payload["channels"].as_object() else {
        return SetupStep::fail("Notify channels", "doctor JSON did not include channels");
    };
    let statuses: Vec<&str> = channels
        .values()
        .filter_map(|item| item["status"].as_str())
        .collect();
    if statuses.iter().any(|s| *s == "OK" || *s == "WARN") {
        SetupStep::ok("Notify channels", "at least one NotifyChannel manifest validated")
    } else if statuses.iter().all(|s| *s == "MISSING") {
        SetupStep::skip("Notify channels", "no notify-channel env is configured yet")
    } else {
        SetupStep::fail("Notify channels", "one or more configured channels failed validation")
    }
}

pub fn genesis_step() -> SetupStep {
    let archetypes = argo_genesis::archetype::list_archetypes();
    if archetypes.is_empty() {
        return SetupStep::fail(GENESIS_STEP, "no Genesis archetypes registered");
    }
    let names: Vec<String> = archetypes.iter().take(3).map(|a| a.name.clone()).collect();
    SetupStep::ok(
        GENESIS_STEP,
        format!("{} archetypes available; suggestions: {}", archetypes.len(), names.join(", ")),
    )
}

pub async fn audit_step(home: &Path) -> SetupStep {
    match audit_count(home).await {
        Ok(n) => SetupStep::ok("Audit query", format!("fresh audit query succeeded ({n} row(s))")),
        Err(e) => SetupStep::fail("Audit query", format!("audit query: {e}")),
    }
}

fn vertical_dir(cwd: &Path) -> PathBuf {
    let dir = argo_vertical::default_dir();
    if dir.is_absolute() {
        dir
    } else {
        cwd.join(dir)
    }
}

fn ensure_writeable_dir(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let probe = dir.join(".setup-write-probe");
    std::fs::write(&probe, b"ok").map_err(|e| format!("write {}: {e}", probe.display()))?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

async fn count_workers(home: &Path) -> ArgoResult<i64> {
    let store = SessionStore::open(db_path(home)).await?;
    store.migrate().await?;
    argo_identity::IdentityStore::new(&store).ensure_schema().await?;
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM worker")
        .fetch_one(store.pool())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("count workers: {e}")))?;
    Ok(count)
}

async fn audit_count(home: &Path) -> ArgoResult<i64> {
    let store = SessionStore::open(db_path(home)).await?;
    store.migrate().await?;
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("count audit rows: {e}")))?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn workspace_step_creates_writeable_manifest_dir() {
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let step = workspace_step(cwd.path(), home.path());
        assert_eq!(step.status, StepStatus::Ok);
        assert!(cwd.path().join(".argo/manifests").is_dir());
    }

    #[test]
    fn vertical_step_skips_empty_registry() {
        let cwd = tempfile::tempdir().unwrap();
        let step = vertical_step(cwd.path());
        assert_eq!(step.status, StepStatus::Skip);
        assert!(step.detail.contains("no vertical.toml"));
    }

    #[tokio::test]
    async fn worker_step_offers_spawn_when_zero_workers() {
        let home = tempfile::tempdir().unwrap();
        let step = worker_step(home.path(), false).await;
        assert_eq!(step.status, StepStatus::Skip);
        assert!(step.detail.contains("argo worker spawn"));
    }

    #[test]
    fn notify_step_skips_when_channels_missing() {
        let payload = json!({"channels": {
            "email": {"status": "MISSING"},
            "slack": {"status": "MISSING"},
            "sms": {"status": "MISSING"},
            "push": {"status": "MISSING"}
        }});
        let step = notify_step(&payload);
        assert_eq!(step.status, StepStatus::Skip);
    }

    #[tokio::test]
    async fn audit_step_succeeds_on_fresh_db() {
        let home = tempfile::tempdir().unwrap();
        let step = audit_step(home.path()).await;
        assert_eq!(step.status, StepStatus::Ok);
    }
}
