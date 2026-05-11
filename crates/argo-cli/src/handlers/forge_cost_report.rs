//! `argo forge cost-report` — per-skill cost-attribution rollup over
//! the recorded `forge_run` history (mission-forge-cost-attribution).
//!
//! Sums per-call token counts, model-priced LLM cost, E2B sandbox
//! cost, and a stamped total — grouped by `skill_name`, ordered by
//! total cost descending. Powers the operator workflow described in
//! MISSION_BACKLOG Section 6 #206 (Mission Claim 4 — "skills shared
//! and learned"): identify which skills cost too much to optimize so
//! their budgets can be tuned.
use super::forge_subject::open_store;
use argo_forge::{query_forge_costs, CostQueryFilter, SkillCostRow};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

/// Entry point — parses the `--since` arg, runs the aggregation, and
/// renders the result as a human table or a JSON envelope.
pub async fn run(skill: Option<String>, since: Option<String>, json: bool) -> ArgoResult<()> {
    let since_ts = parse_since(since.as_deref())?;
    let store = open_store().await?;
    let rows = query_forge_costs(
        &store,
        &CostQueryFilter {
            skill: skill.clone(),
            since: since_ts,
        },
    )
    .await?;
    if json {
        render_json(&rows, &skill, since_ts)
    } else {
        render_text(&rows, &skill, since_ts)
    }
}

/// Accept RFC-3339 (`2026-05-01T00:00:00Z`) or `YYYY-MM-DD` (parsed
/// as UTC midnight). Returns `None` when the input is `None`.
fn parse_since(raw: Option<&str>) -> ArgoResult<Option<DateTime<Utc>>> {
    let Some(s) = raw else { return Ok(None) };
    if let Ok(ts) = DateTime::parse_from_rfc3339(s) {
        return Ok(Some(ts.with_timezone(&Utc)));
    }
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            ArgoError::Provider(format!("--since: invalid midnight for date {s}"))
        })?;
        return Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)));
    }
    Err(ArgoError::Provider(format!(
        "--since: expected RFC-3339 or YYYY-MM-DD, got '{s}'"
    )))
}

#[derive(Serialize)]
struct Envelope<'a> {
    rows: &'a [SkillCostRow],
    totals: Totals,
    skill: Option<&'a String>,
    since: Option<String>,
}

#[derive(Serialize)]
struct Totals {
    run_count: i64,
    total_cost_usd_micros: i64,
    llm_cost_usd_micros: i64,
    e2b_cost_usd_micros: i64,
}

fn totals_for(rows: &[SkillCostRow]) -> Totals {
    rows.iter().fold(
        Totals {
            run_count: 0,
            total_cost_usd_micros: 0,
            llm_cost_usd_micros: 0,
            e2b_cost_usd_micros: 0,
        },
        |mut acc, r| {
            acc.run_count = acc.run_count.saturating_add(r.run_count);
            acc.total_cost_usd_micros = acc
                .total_cost_usd_micros
                .saturating_add(r.total_cost_usd_micros);
            acc.llm_cost_usd_micros = acc
                .llm_cost_usd_micros
                .saturating_add(r.llm_cost_usd_micros);
            acc.e2b_cost_usd_micros = acc
                .e2b_cost_usd_micros
                .saturating_add(r.e2b_cost_usd_micros);
            acc
        },
    )
}

fn render_json(
    rows: &[SkillCostRow],
    skill: &Option<String>,
    since: Option<DateTime<Utc>>,
) -> ArgoResult<()> {
    let env = Envelope {
        rows,
        totals: totals_for(rows),
        skill: skill.as_ref(),
        since: since.map(|t| t.to_rfc3339()),
    };
    let s = serde_json::to_string(&env)
        .map_err(|e| ArgoError::Provider(format!("forge cost-report json: {e}")))?;
    println!("{s}");
    Ok(())
}

fn render_text(
    rows: &[SkillCostRow],
    skill: &Option<String>,
    since: Option<DateTime<Utc>>,
) -> ArgoResult<()> {
    let filter_summary = match (skill, since) {
        (Some(s), Some(t)) => format!(" (skill={s}, since={})", t.to_rfc3339()),
        (Some(s), None) => format!(" (skill={s})"),
        (None, Some(t)) => format!(" (since={})", t.to_rfc3339()),
        (None, None) => String::new(),
    };
    if rows.is_empty() {
        println!("argo forge cost-report{filter_summary}: no recorded forge runs");
        return Ok(());
    }
    println!("argo forge cost-report{filter_summary}");
    println!(
        "  {:<28} {:>5}  {:>14}  {:>14}  {:>14}  {:>14}",
        "skill", "runs", "total ($)", "mean/run ($)", "llm ($)", "e2b ($)"
    );
    for r in rows {
        println!(
            "  {skill:<28} {runs:>5}  {total:>14}  {mean:>14}  {llm:>14}  {e2b:>14}",
            skill = trim_to(&r.skill_name, 28),
            runs = r.run_count,
            total = fmt_usd(r.total_cost_usd_micros),
            mean = fmt_usd(r.mean_cost_usd_micros),
            llm = fmt_usd(r.llm_cost_usd_micros),
            e2b = fmt_usd(r.e2b_cost_usd_micros),
        );
    }
    let totals = totals_for(rows);
    println!(
        "\ntotals: runs={} total={} llm={} e2b={}",
        totals.run_count,
        fmt_usd(totals.total_cost_usd_micros),
        fmt_usd(totals.llm_cost_usd_micros),
        fmt_usd(totals.e2b_cost_usd_micros),
    );
    Ok(())
}

fn trim_to(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

/// Format USD-micros as `$0.000000` with 6-decimal precision so
/// fractions-of-a-cent costs (typical for short Forge runs) don't
/// round to zero.
fn fmt_usd(micros: i64) -> String {
    let dollars = (micros as f64) / 1_000_000.0;
    format!("${dollars:.6}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_since_accepts_rfc3339() {
        let got = parse_since(Some("2026-05-01T12:00:00Z")).unwrap().unwrap();
        assert_eq!(got.to_rfc3339(), "2026-05-01T12:00:00+00:00");
    }

    #[test]
    fn parse_since_accepts_date_only() {
        let got = parse_since(Some("2026-05-01")).unwrap().unwrap();
        assert_eq!(got.to_rfc3339(), "2026-05-01T00:00:00+00:00");
    }

    #[test]
    fn parse_since_returns_none_for_none() {
        assert!(parse_since(None).unwrap().is_none());
    }

    #[test]
    fn parse_since_rejects_garbage() {
        assert!(parse_since(Some("not-a-date")).is_err());
    }

    #[test]
    fn fmt_usd_renders_six_decimal_dollars() {
        assert_eq!(fmt_usd(0), "$0.000000");
        assert_eq!(fmt_usd(1), "$0.000001");
        assert_eq!(fmt_usd(1_000_000), "$1.000000");
        assert_eq!(fmt_usd(31_500), "$0.031500");
    }

    #[test]
    fn trim_to_truncates_long_strings_with_ellipsis() {
        assert_eq!(trim_to("short", 28), "short");
        let long = "a-very-very-very-very-very-very-long-skill-name";
        let t = trim_to(long, 8);
        assert_eq!(t.chars().count(), 8);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn totals_sums_columns_across_rows() {
        let rows = vec![
            SkillCostRow {
                skill_name: "a".into(),
                run_count: 3,
                total_cost_usd_micros: 100,
                llm_cost_usd_micros: 80,
                e2b_cost_usd_micros: 20,
                mean_cost_usd_micros: 33,
                llm_input_tokens: 0,
                llm_output_tokens: 0,
                llm_cache_read_tokens: 0,
            },
            SkillCostRow {
                skill_name: "b".into(),
                run_count: 2,
                total_cost_usd_micros: 50,
                llm_cost_usd_micros: 40,
                e2b_cost_usd_micros: 10,
                mean_cost_usd_micros: 25,
                llm_input_tokens: 0,
                llm_output_tokens: 0,
                llm_cache_read_tokens: 0,
            },
        ];
        let t = totals_for(&rows);
        assert_eq!(t.run_count, 5);
        assert_eq!(t.total_cost_usd_micros, 150);
        assert_eq!(t.llm_cost_usd_micros, 120);
        assert_eq!(t.e2b_cost_usd_micros, 30);
    }
}
