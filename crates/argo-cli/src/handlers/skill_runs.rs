//! `argo skill runs` - chronological skill invocation history.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::{runs_for_skill, runs_for_tenant, SessionStore, SkillRunRow};
use argo_types::{ArgoError, ArgoResult};
use chrono::{Duration, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct SkillRunsOut {
    mode: String,
    skill_name: Option<String>,
    tenant_id: Option<String>,
    worker_id: Option<String>,
    since: Option<String>,
    limit: usize,
    include_failed: bool,
    rows: Vec<SkillRunRow>,
}

pub async fn run(
    skill_name: Option<&str>,
    tenant: Option<&str>,
    worker: Option<&str>,
    since: Option<&str>,
    limit: usize,
    include_failed: bool,
    json: bool,
) -> ArgoResult<()> {
    let since_micros = since_micros(since)?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;

    let (mode, rows) = match tenant {
        Some(tenant_id) => (
            "tenant".to_string(),
            runs_for_tenant(&store, tenant_id, since_micros, limit, include_failed).await?,
        ),
        None => {
            let name = skill_name.ok_or_else(|| {
                ArgoError::Provider("skill name is required unless --tenant is set".into())
            })?;
            (
                "skill".to_string(),
                runs_for_skill(&store, name, worker, since_micros, limit, include_failed).await?,
            )
        }
    };

    if json {
        let out = SkillRunsOut {
            mode,
            skill_name: skill_name.map(str::to_string),
            tenant_id: tenant.map(str::to_string),
            worker_id: worker.map(str::to_string),
            since: since.map(str::to_string),
            limit,
            include_failed,
            rows,
        };
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"ok": true, "skill_runs": out}))
                .map_err(|e| ArgoError::Provider(format!("skill runs JSON: {e}")))?
        );
    } else {
        render_text(&rows, tenant.is_some());
    }
    Ok(())
}

fn since_micros(raw: Option<&str>) -> ArgoResult<Option<i64>> {
    raw.map(parse_duration)
        .transpose()
        .map(|duration| duration.map(|d| (Utc::now() - d).timestamp_micros()))
}

fn parse_duration(raw: &str) -> ArgoResult<Duration> {
    let raw = raw.trim();
    if raw.len() < 2 {
        return Err(ArgoError::Schema(format!("invalid duration '{raw}'")));
    }
    let (digits, unit) = raw.split_at(raw.len() - 1);
    let value = digits
        .parse::<i64>()
        .map_err(|e| ArgoError::Schema(format!("invalid duration '{raw}': {e}")))?;
    if value <= 0 {
        return Err(ArgoError::Provider(format!(
            "duration must be positive: {raw}"
        )));
    }
    match unit {
        "s" => Ok(Duration::seconds(value)),
        "m" => Ok(Duration::minutes(value)),
        "h" => Ok(Duration::hours(value)),
        "d" => Ok(Duration::days(value)),
        _ => Err(ArgoError::Provider(format!(
            "unsupported duration suffix '{unit}' in '{raw}'"
        ))),
    }
}

fn render_text(rows: &[SkillRunRow], show_skill: bool) {
    if rows.is_empty() {
        println!("(no skill runs)");
        return;
    }
    if show_skill {
        println!("timestamp                  skill                 worker        decision  latency_ms  action_id");
    } else {
        println!("timestamp                  worker        decision  latency_ms  action_id");
    }
    for row in rows {
        let latency = row
            .latency_ms
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        if show_skill {
            println!(
                "{:<26} {:<21} {:<13} {:<9} {:>10}  {}",
                row.timestamp,
                row.skill_name,
                row.worker_id,
                row.decision,
                latency,
                short(&row.action_id)
            );
        } else {
            println!(
                "{:<26} {:<13} {:<9} {:>10}  {}",
                row.timestamp,
                row.worker_id,
                row.decision,
                latency,
                short(&row.action_id)
            );
        }
    }
}

fn short(id: &str) -> String {
    if id.len() > 14 {
        format!("{}...", &id[..11])
    } else {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_accepts_common_units() {
        assert_eq!(parse_duration("24h").unwrap(), Duration::hours(24));
        assert_eq!(parse_duration("7d").unwrap(), Duration::days(7));
    }
}
