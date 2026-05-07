//! Pure comparison logic for persisted eval run reports.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Grouping dimension for comparison output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonGroup {
    /// One output row per fixture id.
    Fixture,
    /// Aggregate output rows by skill name.
    Skill,
    /// Aggregate output rows by metric name.
    Metric,
}

impl ComparisonGroup {
    /// Parse the CLI spelling for a comparison grouping.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fixture" => Some(Self::Fixture),
            "skill" => Some(Self::Skill),
            "metric" => Some(Self::Metric),
            _ => None,
        }
    }
}

/// One persisted eval run with normalized per-fixture results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalRunReport {
    /// Stable run id.
    pub id: String,
    /// Vertical id the run evaluated.
    pub vertical: String,
    /// RFC3339 timestamp or other lexically sortable created-at value.
    pub created_at: String,
    /// Per-fixture outcomes.
    pub fixtures: Vec<EvalRunFixture>,
}

impl EvalRunReport {
    /// Return the pass rate across fixtures, or `0.0` for an empty run.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.fixtures.is_empty() {
            return 0.0;
        }
        let passed = self.fixtures.iter().filter(|f| f.passed).count();
        passed as f64 / self.fixtures.len() as f64
    }
}

/// Normalized result for one fixture inside an eval run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalRunFixture {
    /// Fixture id.
    pub fixture: String,
    /// Skill or workflow capability under test.
    pub skill: String,
    /// Metric name.
    pub metric: String,
    /// Whether this fixture passed.
    pub passed: bool,
    /// Numeric score, where higher is better.
    pub score: f64,
}

/// Complete comparison report between two eval runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonReport {
    /// Left-hand run id.
    pub run_a: String,
    /// Right-hand run id.
    pub run_b: String,
    /// Grouping dimension used for `groups`.
    pub group_by: ComparisonGroup,
    /// Fixture count in run A.
    pub total_a: usize,
    /// Fixture count in run B.
    pub total_b: usize,
    /// Pass rate in run A.
    pub pass_rate_a: f64,
    /// Pass rate in run B.
    pub pass_rate_b: f64,
    /// `pass_rate_b - pass_rate_a`.
    pub pass_rate_delta: f64,
    /// Median per-fixture score delta over the fixture union.
    pub median_score_delta: f64,
    /// Fixtures whose B result is worse than A.
    pub regressions: Vec<ComparisonItem>,
    /// Fixtures whose B result is better than A.
    pub improvements: Vec<ComparisonItem>,
    /// Aggregates by the requested grouping dimension.
    pub groups: Vec<GroupComparison>,
}

/// Per-fixture comparison row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonItem {
    /// Fixture id.
    pub fixture: String,
    /// Skill name.
    pub skill: String,
    /// Metric name.
    pub metric: String,
    /// Pass flag from run A.
    pub passed_a: bool,
    /// Pass flag from run B.
    pub passed_b: bool,
    /// Score from run A.
    pub score_a: f64,
    /// Score from run B.
    pub score_b: f64,
    /// `score_b - score_a`.
    pub score_delta: f64,
}

/// Aggregate comparison row for one group key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupComparison {
    /// Group key.
    pub key: String,
    /// Number of fixtures in this group.
    pub fixtures: usize,
    /// Pass rate for run A within this group.
    pub pass_rate_a: f64,
    /// Pass rate for run B within this group.
    pub pass_rate_b: f64,
    /// Median score delta within this group.
    pub median_score_delta: f64,
    /// Count of worse fixtures in this group.
    pub regressions: usize,
    /// Count of better fixtures in this group.
    pub improvements: usize,
}

/// Compare two eval run reports.
#[must_use]
pub fn compare_runs(
    run_a: &EvalRunReport,
    run_b: &EvalRunReport,
    group_by: ComparisonGroup,
) -> ComparisonReport {
    let items = comparison_items(run_a, run_b);
    let regressions = items.iter().filter(|i| is_regression(i)).cloned().collect();
    let improvements = items
        .iter()
        .filter(|i| is_improvement(i))
        .cloned()
        .collect();
    let groups = group_items(&items, group_by);
    ComparisonReport {
        run_a: run_a.id.clone(),
        run_b: run_b.id.clone(),
        group_by,
        total_a: run_a.fixtures.len(),
        total_b: run_b.fixtures.len(),
        pass_rate_a: run_a.pass_rate(),
        pass_rate_b: run_b.pass_rate(),
        pass_rate_delta: run_b.pass_rate() - run_a.pass_rate(),
        median_score_delta: median(items.iter().map(|i| i.score_delta).collect()),
        regressions,
        improvements,
        groups,
    }
}

fn comparison_items(run_a: &EvalRunReport, run_b: &EvalRunReport) -> Vec<ComparisonItem> {
    let map_a = fixture_map(&run_a.fixtures);
    let map_b = fixture_map(&run_b.fixtures);
    let keys: BTreeSet<String> = map_a.keys().chain(map_b.keys()).cloned().collect();
    keys.into_iter()
        .map(|key| item_for_key(&key, map_a.get(&key), map_b.get(&key)))
        .collect()
}

fn fixture_map(fixtures: &[EvalRunFixture]) -> BTreeMap<String, &EvalRunFixture> {
    fixtures.iter().map(|f| (f.fixture.clone(), f)).collect()
}

fn item_for_key(
    key: &str,
    left: Option<&&EvalRunFixture>,
    right: Option<&&EvalRunFixture>,
) -> ComparisonItem {
    let labels = right.or(left);
    let skill = labels.map_or_else(|| "(unknown)".into(), |f| f.skill.clone());
    let metric = labels.map_or_else(|| "(unknown)".into(), |f| f.metric.clone());
    let score_a = left.map_or(0.0, |f| f.score);
    let score_b = right.map_or(0.0, |f| f.score);
    ComparisonItem {
        fixture: key.to_string(),
        skill,
        metric,
        passed_a: left.is_some_and(|f| f.passed),
        passed_b: right.is_some_and(|f| f.passed),
        score_a,
        score_b,
        score_delta: score_b - score_a,
    }
}

fn is_regression(item: &ComparisonItem) -> bool {
    (item.passed_a && !item.passed_b) || item.score_delta < 0.0
}

fn is_improvement(item: &ComparisonItem) -> bool {
    (!item.passed_a && item.passed_b) || item.score_delta > 0.0
}

fn group_items(items: &[ComparisonItem], group_by: ComparisonGroup) -> Vec<GroupComparison> {
    let mut grouped: BTreeMap<String, Vec<&ComparisonItem>> = BTreeMap::new();
    for item in items {
        grouped
            .entry(group_key(item, group_by))
            .or_default()
            .push(item);
    }
    grouped
        .into_iter()
        .map(|(key, rows)| group_report(key, &rows))
        .collect()
}

fn group_key(item: &ComparisonItem, group_by: ComparisonGroup) -> String {
    match group_by {
        ComparisonGroup::Fixture => item.fixture.clone(),
        ComparisonGroup::Skill => item.skill.clone(),
        ComparisonGroup::Metric => item.metric.clone(),
    }
}

fn group_report(key: String, rows: &[&ComparisonItem]) -> GroupComparison {
    let total = rows.len();
    let pass_rate_a = pass_rate(rows.iter().filter(|i| i.passed_a).count(), total);
    let pass_rate_b = pass_rate(rows.iter().filter(|i| i.passed_b).count(), total);
    GroupComparison {
        key,
        fixtures: total,
        pass_rate_a,
        pass_rate_b,
        median_score_delta: median(rows.iter().map(|i| i.score_delta).collect()),
        regressions: rows.iter().filter(|i| is_regression(i)).count(),
        improvements: rows.iter().filter(|i| is_improvement(i)).count(),
    }
}

fn pass_rate(passed: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        passed as f64 / total as f64
    }
}

fn median(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(id: &str, fixtures: Vec<(&str, &str, &str, bool, f64)>) -> EvalRunReport {
        EvalRunReport {
            id: id.into(),
            vertical: "support".into(),
            created_at: "2026-05-07T00:00:00Z".into(),
            fixtures: fixtures
                .into_iter()
                .map(|(fixture, skill, metric, passed, score)| EvalRunFixture {
                    fixture: fixture.into(),
                    skill: skill.into(),
                    metric: metric.into(),
                    passed,
                    score,
                })
                .collect(),
        }
    }

    #[test]
    fn pass_rate_delta_is_b_minus_a() {
        let a = run(
            "a",
            vec![("f1", "s", "m", true, 1.0), ("f2", "s", "m", false, 0.0)],
        );
        let b = run(
            "b",
            vec![("f1", "s", "m", true, 1.0), ("f2", "s", "m", true, 1.0)],
        );
        let report = compare_runs(&a, &b, ComparisonGroup::Fixture);
        assert_eq!(report.pass_rate_a, 0.5);
        assert_eq!(report.pass_rate_b, 1.0);
        assert_eq!(report.pass_rate_delta, 0.5);
    }

    #[test]
    fn true_to_false_pass_flip_is_regression() {
        let a = run("a", vec![("f1", "invoice", "exact", true, 1.0)]);
        let b = run("b", vec![("f1", "invoice", "exact", false, 0.0)]);
        let report = compare_runs(&a, &b, ComparisonGroup::Fixture);
        assert_eq!(report.regressions.len(), 1);
        assert_eq!(report.regressions[0].fixture, "f1");
    }

    #[test]
    fn false_to_true_pass_flip_is_improvement() {
        let a = run("a", vec![("f1", "support", "exact", false, 0.0)]);
        let b = run("b", vec![("f1", "support", "exact", true, 1.0)]);
        let report = compare_runs(&a, &b, ComparisonGroup::Fixture);
        assert_eq!(report.improvements.len(), 1);
        assert_eq!(report.improvements[0].skill, "support");
    }

    #[test]
    fn median_score_delta_handles_even_counts() {
        let a = run(
            "a",
            vec![("f1", "s", "m", true, 0.2), ("f2", "s", "m", true, 0.4)],
        );
        let b = run(
            "b",
            vec![("f1", "s", "m", true, 0.6), ("f2", "s", "m", true, 0.6)],
        );
        let report = compare_runs(&a, &b, ComparisonGroup::Fixture);
        assert!((report.median_score_delta - 0.3).abs() < 1e-9);
    }

    #[test]
    fn grouping_by_skill_aggregates_rows() {
        let a = run(
            "a",
            vec![
                ("f1", "invoice", "exact", true, 1.0),
                ("f2", "invoice", "exact", true, 1.0),
                ("f3", "reply", "exact", false, 0.0),
            ],
        );
        let b = run(
            "b",
            vec![
                ("f1", "invoice", "exact", false, 0.0),
                ("f2", "invoice", "exact", true, 1.0),
                ("f3", "reply", "exact", true, 1.0),
            ],
        );
        let report = compare_runs(&a, &b, ComparisonGroup::Skill);
        let invoice = report.groups.iter().find(|g| g.key == "invoice").unwrap();
        assert_eq!(invoice.fixtures, 2);
        assert_eq!(invoice.regressions, 1);
        assert_eq!(invoice.improvements, 0);
    }

    #[test]
    fn missing_fixture_in_b_counts_as_regression() {
        let a = run("a", vec![("f1", "invoice", "exact", true, 0.8)]);
        let b = run("b", vec![]);
        let report = compare_runs(&a, &b, ComparisonGroup::Fixture);
        assert_eq!(report.total_a, 1);
        assert_eq!(report.total_b, 0);
        assert_eq!(report.regressions.len(), 1);
        assert_eq!(report.regressions[0].score_b, 0.0);
    }
}
