//! `argo eval compare`.
use super::eval_list::{load_run_arg, print_json};
use argo_eval::{compare_runs, ComparisonGroup, ComparisonItem, ComparisonReport};
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;

pub async fn run(
    run_a: &str,
    run_b: &str,
    by: &str,
    show_regressions: bool,
    show_improvements: bool,
    json_out: bool,
) -> ArgoResult<()> {
    let group_by = ComparisonGroup::parse(by)
        .ok_or_else(|| ArgoError::Provider("--by must be one of: fixture, skill, metric".into()))?;
    let left = load_run_arg(run_a)?;
    let right = load_run_arg(run_b)?;
    let report = compare_runs(&left, &right, group_by);
    if json_out {
        return print_json(&json!({"ok": true, "comparison": report}));
    }
    print!(
        "{}",
        render_markdown(&report, show_regressions, show_improvements)
    );
    Ok(())
}

fn render_markdown(
    report: &ComparisonReport,
    show_regressions: bool,
    show_improvements: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "## Comparison: {} vs {}\n",
        report.run_a, report.run_b
    ));
    out.push_str(&format!(
        "- Total fixtures: A={}, B={} ({})\n",
        report.total_a,
        report.total_b,
        if report.total_a == report.total_b {
            "same"
        } else {
            "different"
        }
    ));
    out.push_str(&format!(
        "- Pass rate: A={} -> B={} ({})\n",
        percent(report.pass_rate_a),
        percent(report.pass_rate_b),
        signed_percent(report.pass_rate_delta)
    ));
    if show_regressions || !show_improvements {
        out.push_str(&format!(
            "- Regressions: {} fixtures ({})\n",
            report.regressions.len(),
            skill_summary(&report.regressions)
        ));
    }
    if show_improvements || !show_regressions {
        out.push_str(&format!(
            "- Improvements: {} fixtures ({})\n",
            report.improvements.len(),
            skill_summary(&report.improvements)
        ));
    }
    out.push_str(&format!(
        "- Score delta median: {:+.2}\n",
        report.median_score_delta
    ));
    if !show_regressions && !show_improvements {
        render_groups(&mut out, report);
    }
    if show_regressions || !show_improvements {
        render_items(&mut out, "Regressions", &report.regressions);
    }
    if show_improvements || !show_regressions {
        render_items(&mut out, "Improvements", &report.improvements);
    }
    out
}

fn render_groups(out: &mut String, report: &ComparisonReport) {
    out.push_str(&format!(
        "\n### Groups by {}\n",
        group_name(report.group_by)
    ));
    out.push_str(
        "| Key | Fixtures | Pass A | Pass B | Median delta | Regressions | Improvements |\n",
    );
    out.push_str("|---|---:|---:|---:|---:|---:|---:|\n");
    for group in &report.groups {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {:+.2} | {} | {} |\n",
            group.key,
            group.fixtures,
            percent(group.pass_rate_a),
            percent(group.pass_rate_b),
            group.median_score_delta,
            group.regressions,
            group.improvements
        ));
    }
}

fn render_items(out: &mut String, title: &str, items: &[ComparisonItem]) {
    out.push_str(&format!("\n### {title}\n"));
    if items.is_empty() {
        out.push_str("(none)\n");
        return;
    }
    for item in items {
        out.push_str(&format!(
            "- {} (skill: {}, metric: {}): {:.2} -> {:.2} ({:+.2})\n",
            item.fixture, item.skill, item.metric, item.score_a, item.score_b, item.score_delta
        ));
    }
}

fn group_name(group: ComparisonGroup) -> &'static str {
    match group {
        ComparisonGroup::Fixture => "fixture",
        ComparisonGroup::Skill => "skill",
        ComparisonGroup::Metric => "metric",
    }
}

fn percent(value: f64) -> String {
    format!("{:.0}%", value * 100.0)
}

fn signed_percent(value: f64) -> String {
    format!("{:+.0}%", value * 100.0)
}

fn skill_summary(items: &[ComparisonItem]) -> String {
    let mut skills: Vec<&str> = items.iter().map(|i| i.skill.as_str()).collect();
    skills.sort_unstable();
    skills.dedup();
    if skills.is_empty() {
        "skill: none".into()
    } else {
        format!("skill: {}", skills.join(", "))
    }
}
