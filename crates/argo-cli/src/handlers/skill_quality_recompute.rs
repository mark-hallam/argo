//! `argo skill quality-recompute` — derive each in-scope skill's
//! [`QualityInputs`] from the audit log
//! ([`QualityInputs::from_audit_history`]) and persist the new
//! [`QualityScore`] via the existing [`SkillQualityStore`].
//!
//! Operator-facing path for periodic recompute (Mission Claim 4
//! item #78). Designed to be cron-suitable: stateless, exits 0 on
//! an empty marketplace, prints one line per skill. The actual cron
//! / systemd-timer wrapper is out of scope — operators wire this
//! via `crontab -e`, e.g.
//!
//! ```text
//! @hourly /usr/local/bin/argo skill quality-recompute --all --since 24h
//! ```
//!
//! Pairs with #77 (`mission-skill-quality-from-audit`) which lands
//! the offline constructor this handler exercises. See brief
//! `streams/mission-skill-quality-cron.md`.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{
    compute_score, AuditStoreReader, QualityInputs, SessionStoreAuditReader, Skill,
    SkillQualityStore, SkillStore,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::sync::Arc;

/// Progress threshold for `--all`: print a per-N-skill heartbeat so
/// an operator running the command interactively gets feedback.
const PROGRESS_EVERY: usize = 50;

/// CLI entry point.
///
/// # Errors
/// Propagates any audit-store or quality-store error. Missing
/// `--skill <name>` (when neither `--skill` nor `--all` is set) is
/// rejected by clap.
pub async fn run(
    skill: Option<&str>,
    all: bool,
    since: Option<&str>,
    dry_run: bool,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let skills_store = SkillStore::new(Arc::clone(&store));
    let quality_store = SkillQualityStore::new(Arc::clone(&store));
    quality_store.ensure_schema().await?;

    let since_dt = parse_since(since)?;
    let now = Utc::now();
    let targets = resolve_targets(&skills_store, skill, all).await?;
    if all && targets.is_empty() {
        println!("no skills found");
        return Ok(());
    }
    let total = targets.len();
    for (idx, skill) in targets.into_iter().enumerate() {
        recompute_one(&store, &quality_store, &skill, since_dt, now, dry_run).await?;
        if all && total > PROGRESS_EVERY && (idx + 1) % PROGRESS_EVERY == 0 {
            eprintln!("… {n}/{total} processed", n = idx + 1);
        }
    }
    Ok(())
}

async fn resolve_targets(
    skills: &SkillStore,
    skill_name: Option<&str>,
    all: bool,
) -> ArgoResult<Vec<Skill>> {
    if all {
        return skills.list_active().await;
    }
    let name = skill_name.ok_or_else(|| {
        ArgoError::Provider("quality-recompute: pass --skill <name> or --all".into())
    })?;
    let skill = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill '{name}'")))?;
    Ok(vec![skill])
}

async fn recompute_one(
    store: &Arc<SessionStore>,
    quality_store: &SkillQualityStore,
    skill: &Skill,
    since: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    dry_run: bool,
) -> ArgoResult<()> {
    let reader: Arc<dyn AuditStoreReader> = Arc::new(SessionStoreAuditReader::new(
        Arc::clone(store),
        skill.frontmatter.name.clone(),
    ));
    let inputs = QualityInputs::from_audit_history(&skill.id, reader.as_ref(), since).await?;
    let new_score = compute_score(&inputs);
    let old_score = quality_store
        .latest(skill.id)
        .await?
        .map_or(0_u8, |s| s.score);
    if !dry_run {
        quality_store.record(skill.id, &inputs, now).await?;
    }
    print_score_line(skill, old_score, new_score);
    Ok(())
}

fn print_score_line(skill: &Skill, old_score: u8, new_score: u8) {
    let delta = i16::from(new_score) - i16::from(old_score);
    println!(
        "{skill_id} {old} -> {new} ({delta:+})",
        skill_id = skill.id,
        old = old_score,
        new = new_score,
    );
}

/// Parse the `--since` flag. Accepts RFC-3339 timestamps and
/// short relative durations (`30m`, `24h`, `7d`). Mirrors the
/// `budget watch` helper but keeps the surface local — the budget
/// module is not a public re-export.
pub(crate) fn parse_since(raw: Option<&str>) -> ArgoResult<Option<DateTime<Utc>>> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(Some(dt.with_timezone(&Utc)));
    }
    if let Some(lookback) = parse_lookback(s) {
        return Ok(Some(Utc::now() - lookback));
    }
    Err(ArgoError::Provider(format!(
        "invalid --since '{s}' (expected duration like 30m/1h/7d or RFC3339)"
    )))
}

fn parse_lookback(raw: &str) -> Option<ChronoDuration> {
    let (value, unit) = raw.split_at(raw.len().checked_sub(1)?);
    let value = value.parse::<i64>().ok().filter(|v| *v > 0)?;
    match unit {
        "s" => Some(ChronoDuration::seconds(value)),
        "m" => Some(ChronoDuration::minutes(value)),
        "h" => Some(ChronoDuration::hours(value)),
        "d" => Some(ChronoDuration::days(value)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "skill_quality_recompute_tests.rs"]
mod tests;
