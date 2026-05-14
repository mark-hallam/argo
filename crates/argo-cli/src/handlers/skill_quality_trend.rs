//! `argo skill quality-trend <name>` — ASCII sparkline of the past
//! `--days` daily quality scores (`claude-skill-quality-judge` follow-
//! up). Reuses `SkillQualityStore::history` from #886.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{QualityScore, SkillQualityStore, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{Days, NaiveDate, Utc};
use serde_json::json;
use std::sync::Arc;

const SPARK_BARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const EMPTY_DAY: char = '·';
const DEFAULT_DAYS: u32 = 30;
const MAX_DAYS: u32 = 365;

pub async fn run(name: &str, days: Option<u32>, json_output: bool) -> ArgoResult<()> {
    let days = days.unwrap_or(DEFAULT_DAYS).clamp(1, MAX_DAYS);
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let skills = SkillStore::new(Arc::clone(&store));
    let skill = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    let qstore = SkillQualityStore::new(store);
    qstore.ensure_schema().await?;
    let history = qstore.history(skill.id).await?;
    let today = Utc::now().date_naive();
    let cutoff = today
        .checked_sub_days(Days::new((days as u64).saturating_sub(1)))
        .unwrap_or(today);
    let buckets = bucket_history(&history, cutoff, today);
    let sparkline = render_sparkline(&buckets);
    if json_output {
        let envelope = json!({
            "ok": true,
            "kind": "skill_quality_trend",
            "skill_name": name,
            "skill_id": skill.id.to_string(),
            "days": days,
            "buckets": buckets
                .iter()
                .enumerate()
                .map(|(i, score)| {
                    let day = cutoff
                        .checked_add_days(Days::new(i as u64))
                        .unwrap_or(cutoff);
                    json!({
                        "date": day.format("%Y-%m-%d").to_string(),
                        "score": score,
                    })
                })
                .collect::<Vec<_>>(),
            "sparkline": sparkline,
        });
        let pretty = serde_json::to_string_pretty(&envelope)
            .map_err(|e| ArgoError::Provider(format!("serialize trend envelope: {e}")))?;
        println!("{pretty}");
    } else {
        print_text(name, &buckets, &sparkline, cutoff, today);
    }
    Ok(())
}

/// Project history rows onto a fixed-length per-day bucket vector.
/// `cutoff` is the first day; `today` is the last. Each bucket is the
/// most recent score recorded for that day, or `None` when no score
/// landed.
fn bucket_history(
    history: &[QualityScore],
    cutoff: NaiveDate,
    today: NaiveDate,
) -> Vec<Option<u8>> {
    let span = today.signed_duration_since(cutoff).num_days() as usize + 1;
    let mut buckets = vec![None; span];
    for record in history {
        let day = record
            .computed_at
            .with_timezone(&Utc)
            .date_naive();
        if day < cutoff || day > today {
            continue;
        }
        let idx = day.signed_duration_since(cutoff).num_days() as usize;
        // History is newest-first; first hit per day wins (most
        // recent score that day). After we set, leave it.
        if buckets[idx].is_none() {
            buckets[idx] = Some(record.score);
        }
    }
    buckets
}

fn render_sparkline(buckets: &[Option<u8>]) -> String {
    buckets
        .iter()
        .map(|b| match b {
            Some(score) => bar_for(*score),
            None => EMPTY_DAY,
        })
        .collect()
}

fn bar_for(score: u8) -> char {
    if score == 0 {
        return SPARK_BARS[0];
    }
    let idx = ((score as usize).saturating_sub(1) * SPARK_BARS.len()) / 100;
    SPARK_BARS[idx.min(SPARK_BARS.len() - 1)]
}

fn print_text(
    name: &str,
    buckets: &[Option<u8>],
    sparkline: &str,
    cutoff: NaiveDate,
    today: NaiveDate,
) {
    let scored: Vec<u8> = buckets.iter().filter_map(|b| *b).collect();
    let avg = if scored.is_empty() {
        0.0
    } else {
        scored.iter().map(|s| *s as f64).sum::<f64>() / scored.len() as f64
    };
    let min = scored.iter().min().copied().unwrap_or(0);
    let max = scored.iter().max().copied().unwrap_or(0);
    let last = buckets.iter().rev().find_map(|b| *b);
    println!("skill: {name}");
    println!(
        "window: {} → {} ({} days)",
        cutoff.format("%Y-%m-%d"),
        today.format("%Y-%m-%d"),
        buckets.len()
    );
    println!("trend: {sparkline}");
    if scored.is_empty() {
        println!("(no quality scores in window)");
    } else {
        println!(
            "samples: {} / {}   min={min}  max={max}  avg={:.1}",
            scored.len(),
            buckets.len(),
            avg
        );
        if let Some(latest) = last {
            println!("latest: {latest}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_skill::QualityInputs;
    use argo_types::SkillId;
    use chrono::{Datelike, TimeZone};
    use uuid::Uuid;

    fn score_at(score: u8, day: NaiveDate, hour: u32) -> QualityScore {
        QualityScore {
            skill_id: SkillId(Uuid::nil()),
            score,
            inputs: QualityInputs::default(),
            computed_at: Utc
                .with_ymd_and_hms(day.year(), day.month(), day.day(), hour, 0, 0)
                .unwrap(),
        }
    }

    #[test]
    fn bar_for_low_high_endpoints_select_extreme_bars() {
        assert_eq!(bar_for(0), SPARK_BARS[0]);
        assert_eq!(bar_for(1), SPARK_BARS[0]);
        assert_eq!(bar_for(100), SPARK_BARS[7]);
    }

    #[test]
    fn bar_for_midpoint_is_middle_bar() {
        let mid = bar_for(50);
        assert!(SPARK_BARS.contains(&mid));
    }

    #[test]
    fn bucket_history_drops_records_outside_window() {
        let cutoff = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 7).unwrap();
        let outside = NaiveDate::from_ymd_opt(2026, 3, 25).unwrap();
        let inside = NaiveDate::from_ymd_opt(2026, 4, 3).unwrap();
        let history = vec![score_at(50, outside, 12), score_at(80, inside, 12)];
        let buckets = bucket_history(&history, cutoff, today);
        assert_eq!(buckets.len(), 7);
        // index for inside: 2026-04-03 - 2026-04-01 = 2
        assert_eq!(buckets[2], Some(80));
        assert!(buckets.iter().filter(|b| b.is_some()).count() == 1);
    }

    #[test]
    fn bucket_history_keeps_first_score_per_day_when_history_is_newest_first() {
        let cutoff = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let day = cutoff;
        // Newest-first: hour 18 (latest) before hour 6 (earlier in
        // the same day). Latest should win.
        let history = vec![score_at(95, day, 18), score_at(10, day, 6)];
        let buckets = bucket_history(&history, cutoff, today);
        assert_eq!(buckets, vec![Some(95)]);
    }

    #[test]
    fn render_sparkline_marks_gaps_with_dot() {
        let buckets = vec![Some(80), None, Some(20)];
        let s = render_sparkline(&buckets);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars.len(), 3);
        assert_eq!(chars[1], EMPTY_DAY);
        assert_ne!(chars[0], EMPTY_DAY);
        assert_ne!(chars[2], EMPTY_DAY);
    }

    #[test]
    fn empty_history_renders_all_dots() {
        let buckets: Vec<Option<u8>> = vec![None; 5];
        let s = render_sparkline(&buckets);
        assert_eq!(s.chars().filter(|c| *c == EMPTY_DAY).count(), 5);
    }
}
