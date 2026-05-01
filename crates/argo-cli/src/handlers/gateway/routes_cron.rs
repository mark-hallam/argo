//! `GET /cron[?limit=N]` and `GET /cron/<id>` — read-side
//! exposure of the workspace's `cron/jobs/<name>.json` files.
//!
//! Lives in its own module because (a) the source is a
//! filesystem directory rather than the SQLite store and (b)
//! `routes.rs` + `routes_meta.rs` are both already close to
//! the 500-LOC ceiling.
use argo_cron::CronJob;
use std::path::PathBuf;

/// `GET /cron` — list every job in `<cwd>/cron/jobs/`. Returns
/// a JSON array of canonical `CronJob` rows. Returns 200 + an
/// empty array when the directory doesn't exist (mirrors the
/// CLI's "no cron jobs" message rather than 404'ing).
pub(super) async fn cron_list_response() -> (&'static str, String) {
    let dir = match cron_dir() {
        Ok(d) => d,
        Err((status, body)) => return (status, body),
    };
    if !dir.exists() {
        return ("200 OK", "[]".to_string());
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"read_dir {}: {e}\"}}", dir.display()),
            );
        }
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let p = entry.path();
        if p.extension().is_some_and(|x| x == "json") {
            paths.push(p);
        }
    }
    paths.sort();
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(paths.len());
    for p in &paths {
        match read_job(p) {
            Ok(job) => out.push(serialize_job(&job)),
            Err(line) => out.push(line),
        }
    }
    (
        "200 OK",
        serde_json::to_string(&out).unwrap_or_else(|_| "[]".to_string()),
    )
}

/// `GET /cron/<name>` — fetch one job by `name` (the file's
/// basename, NOT the uuid). Returns 404 when no `<name>.json`
/// exists. The CLI uses `argo cron list` + `argo cron delete
/// <name>` against names too, so this matches the semantics
/// users already know.
pub(super) async fn cron_show_response(raw_name: &str) -> (&'static str, String) {
    let dir = match cron_dir() {
        Ok(d) => d,
        Err((status, body)) => return (status, body),
    };
    if raw_name.contains('/') || raw_name.contains('\\') || raw_name.contains("..") {
        return (
            "400 Bad Request",
            "{\"error\":\"job name must be a single path segment\"}".to_string(),
        );
    }
    let path = dir.join(format!("{raw_name}.json"));
    if !path.exists() {
        return (
            "404 Not Found",
            format!("{{\"error\":\"no cron job named '{raw_name}'\"}}"),
        );
    }
    match read_job(&path) {
        Ok(job) => (
            "200 OK",
            serde_json::to_string(&serialize_job(&job)).unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(line) => (
            "500 Internal Server Error",
            serde_json::to_string(&line).unwrap_or_else(|_| "{}".to_string()),
        ),
    }
}

fn cron_dir() -> Result<PathBuf, (&'static str, String)> {
    let cwd = std::env::current_dir()
        .map_err(|e| ("500 Internal Server Error", format!("{{\"error\":\"cwd: {e}\"}}")))?;
    Ok(cwd.join("cron").join("jobs"))
}

fn read_job(path: &std::path::Path) -> Result<CronJob, serde_json::Value> {
    let body = std::fs::read_to_string(path).map_err(|e| {
        serde_json::json!({
            "error": format!("read {}: {e}", path.display()),
        })
    })?;
    serde_json::from_str::<CronJob>(&body).map_err(|e| {
        serde_json::json!({
            "error": format!("parse {}: {e}", path.display()),
        })
    })
}

fn serialize_job(job: &CronJob) -> serde_json::Value {
    serde_json::json!({
        "id": job.id.to_string(),
        "tenant_id": job.tenant_id.0.to_string(),
        "name": job.name,
        "schedule": job.schedule.source,
        "prompt": job.prompt,
        "platform": job.target.platform,
        "chat": job.target.chat,
        "enabled": job.enabled,
        "last_fired_at": job.last_fired_at.map(|d| d.to_rfc3339()),
    })
}
