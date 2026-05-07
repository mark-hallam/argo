//! `argo action timeline`.

use super::action_history::{action_micros, collect_action_views, parse_duration, print_json, ActionView};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TimelineOut {
    worker_id: String,
    since: String,
    bucket_mins: u32,
    bucket_count: usize,
    histogram: String,
}

pub(crate) async fn run(
    worker_id: &str,
    since: &str,
    bucket_mins: u32,
    json: bool,
) -> ArgoResult<()> {
    let since_duration = parse_duration(since)?;
    let rows = collect_action_views(false).await?;
    let out = build_timeline(worker_id, &rows, since, since_duration, bucket_mins, Utc::now())?;
    if json {
        print_json("action_timeline", out)
    } else {
        println!(
            "worker {} ({} buckets, {}m)",
            out.worker_id, out.bucket_count, out.bucket_mins
        );
        println!("{}", out.histogram);
        Ok(())
    }
}

pub(crate) fn build_timeline(
    worker_id: &str,
    rows: &[ActionView],
    since_label: &str,
    since: Duration,
    bucket_mins: u32,
    now: DateTime<Utc>,
) -> ArgoResult<TimelineOut> {
    if bucket_mins == 0 {
        return Err(ArgoError::Provider(
            "bucket-mins must be greater than zero".to_string(),
        ));
    }
    let bucket = Duration::minutes(i64::from(bucket_mins));
    let bucket_micros = bucket.num_microseconds().unwrap_or(1).max(1);
    let span_micros = since.num_microseconds().unwrap_or(bucket_micros).max(bucket_micros);
    let bucket_count = usize::try_from((span_micros + bucket_micros - 1) / bucket_micros)
        .unwrap_or(usize::MAX)
        .min(10_000);
    let start = now - since;
    let start_micros = start.timestamp_micros();
    let mut buckets = vec!['·'; bucket_count];
    for row in rows.iter().filter(|row| row.worker_id == worker_id) {
        let micros = action_micros(row);
        if micros < start_micros || micros > now.timestamp_micros() {
            continue;
        }
        let idx = usize::try_from((micros - start_micros) / bucket_micros).unwrap_or(0);
        if let Some(bucket) = buckets.get_mut(idx.min(bucket_count.saturating_sub(1))) {
            *bucket = '█';
        }
    }
    Ok(TimelineOut {
        worker_id: worker_id.to_string(),
        since: since_label.to_string(),
        bucket_mins,
        bucket_count,
        histogram: buckets.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn row(id: &str, minutes_ago: i64) -> ActionView {
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        ActionView {
            action_id: id.to_string(),
            worker_id: "w-1".to_string(),
            tenant_id: Some("tenant-a".to_string()),
            created_at: (now - Duration::minutes(minutes_ago)).to_rfc3339(),
            action_type: "post_listing".to_string(),
            state: "completed".to_string(),
            decision: "completed".to_string(),
            spend_usd_micros: 0,
            spend_usd: 0.0,
            skill_used: None,
        }
    }

    #[test]
    fn timeline_builds_288_buckets_for_24h_at_5m() {
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let rows = vec![row("a", 10), row("b", 60)];

        let out = build_timeline("w-1", &rows, "24h", Duration::hours(24), 5, now).unwrap();

        assert_eq!(out.bucket_count, 288);
        assert_eq!(out.histogram.chars().count(), 288);
        assert!(out.histogram.contains('█'));
        assert!(out.histogram.contains('·'));
    }

    #[test]
    fn timeline_rejects_zero_bucket_size() {
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let err = build_timeline("w-1", &[], "1h", Duration::hours(1), 0, now)
            .unwrap_err()
            .to_string();

        assert!(err.contains("bucket-mins"));
    }
}
