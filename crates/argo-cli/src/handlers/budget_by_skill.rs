//! `argo budget by-skill` spend attribution.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::budget::{
    spend_by_skill_history, spend_by_skill_top_n, SkillSpendDay, SkillSpendRow,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct TopEnvelope {
    ok: bool,
    mode: &'static str,
    since: Option<String>,
    vertical_id: Option<String>,
    limit: u32,
    rows: Vec<SkillSpendRow>,
}

#[derive(Debug, Serialize)]
struct HistoryEnvelope {
    ok: bool,
    mode: &'static str,
    skill_name: String,
    since: Option<String>,
    vertical_id: Option<String>,
    rows: Vec<SkillSpendDay>,
}

#[derive(Debug)]
struct SinceFilter {
    label: String,
    cutoff_micros: i64,
}

pub async fn run(
    skill_name: Option<&str>,
    history: bool,
    since: Option<&str>,
    limit: u32,
    vertical: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let since = parse_since_duration(since, Utc::now())?;
    match (skill_name, history) {
        (Some(name), true) => history_command(&store, name, since, vertical, json).await,
        (None, false) => top_command(&store, since, vertical, limit, json).await,
        (Some(_), false) => Err(ArgoError::Provider(
            "budget by-skill <skill-name> requires --history".into(),
        )),
        (None, true) => Err(ArgoError::Provider(
            "budget by-skill --history requires <skill-name>".into(),
        )),
    }
}

async fn top_command(
    store: &SessionStore,
    since: Option<SinceFilter>,
    vertical: Option<&str>,
    limit: u32,
    json: bool,
) -> ArgoResult<()> {
    let rows = spend_by_skill_top_n(
        store.pool(),
        since.as_ref().map(|s| s.cutoff_micros),
        vertical,
        limit,
    )
    .await?;
    if json {
        return print_json(&TopEnvelope {
            ok: true,
            mode: "top",
            since: since.map(|s| s.label),
            vertical_id: vertical.map(str::to_string),
            limit: limit.clamp(1, 500),
            rows,
        });
    }
    print_top(&rows, since.as_ref(), vertical, limit);
    Ok(())
}

async fn history_command(
    store: &SessionStore,
    skill_name: &str,
    since: Option<SinceFilter>,
    vertical: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let rows = spend_by_skill_history(
        store.pool(),
        skill_name,
        since.as_ref().map(|s| s.cutoff_micros),
        vertical,
    )
    .await?;
    if json {
        return print_json(&HistoryEnvelope {
            ok: true,
            mode: "history",
            skill_name: skill_name.to_string(),
            since: since.map(|s| s.label),
            vertical_id: vertical.map(str::to_string),
            rows,
        });
    }
    print_history(skill_name, &rows, since.as_ref(), vertical);
    Ok(())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn parse_since_duration(raw: Option<&str>, now: DateTime<Utc>) -> ArgoResult<Option<SinceFilter>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    let Some(days) = trimmed.strip_suffix('d') else {
        return Err(ArgoError::Provider(format!(
            "invalid --since '{trimmed}' (expected 7d, 30d, or 90d)"
        )));
    };
    let days: i64 = days.parse().map_err(|e| {
        ArgoError::Provider(format!(
            "invalid --since '{trimmed}' (expected positive day duration): {e}"
        ))
    })?;
    if days <= 0 {
        return Err(ArgoError::Provider(format!(
            "invalid --since '{trimmed}' (duration must be positive)"
        )));
    }
    Ok(Some(SinceFilter {
        label: trimmed.to_string(),
        cutoff_micros: (now - Duration::days(days)).timestamp_micros(),
    }))
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialize budget by-skill: {e}")))?;
    println!("{body}");
    Ok(())
}

fn print_top(
    rows: &[SkillSpendRow],
    since: Option<&SinceFilter>,
    vertical: Option<&str>,
    limit: u32,
) {
    if rows.is_empty() {
        println!("(no skill-attributed spend rows)");
        return;
    }
    let mut header = format!("budget by-skill (top {}", limit.clamp(1, 500));
    if let Some(since) = since {
        header.push_str(&format!(", since={}", since.label));
    }
    if let Some(vertical) = vertical {
        header.push_str(&format!(", vertical={vertical}"));
    }
    header.push_str("):");
    println!("{header}");
    for row in rows {
        println!(
            "  ${:>10}  {}@{}  sessions={} charges={}",
            format_usd(row.spend_usd_micros),
            row.skill_name,
            row.version,
            row.session_count,
            row.charge_count
        );
    }
}

fn print_history(
    skill_name: &str,
    rows: &[SkillSpendDay],
    since: Option<&SinceFilter>,
    vertical: Option<&str>,
) {
    if rows.is_empty() {
        println!("(no spend history for skill `{skill_name}`)");
        return;
    }
    let mut header = format!("budget by-skill {skill_name} history");
    if let Some(since) = since {
        header.push_str(&format!(" since={}", since.label));
    }
    if let Some(vertical) = vertical {
        header.push_str(&format!(" vertical={vertical}"));
    }
    header.push(':');
    println!("{header}");
    for row in rows {
        println!(
            "  {}  ${:>10}  charges={}",
            row.day,
            format_usd(row.spend_usd_micros),
            row.charge_count
        );
    }
}

fn format_usd(micros: u64) -> String {
    let usd = micros as f64 / 1_000_000.0;
    if micros > 0 && usd < 0.01 {
        format!("{usd:.4}")
    } else {
        format!("{usd:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_since_accepts_day_duration() {
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 0, 0, 0).single().unwrap();
        let parsed = parse_since_duration(Some("7d"), now).unwrap().unwrap();
        assert_eq!(parsed.label, "7d");
        assert_eq!(
            parsed.cutoff_micros,
            Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0)
                .single()
                .unwrap()
                .timestamp_micros()
        );
    }

    #[test]
    fn parse_since_rejects_non_day_duration() {
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 0, 0, 0).single().unwrap();
        let err = parse_since_duration(Some("1w"), now).unwrap_err();
        assert!(err.to_string().contains("invalid --since"));
    }
}
