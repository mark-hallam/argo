//! `argo eval validate <vertical>` — vertical-eval-harness
//! pass-rate runner (`mvp-vertical-eval-harness`).
//!
//! The legacy `eval validate <path>` command still validates a
//! generic argo-eval task JSONL file. When `<target>` is not a file
//! on disk, the dispatch in `eval_run::validate_target` falls
//! through here and treats it as a vertical id: we load the
//! vertical via the registry, walk every `eval/*.jsonl` suite
//! declared in `vertical.toml [evaluation]`, score each fixture
//! deterministically (existing `argo_vertical::score_fixture`
//! heuristic — same path the `eval run --score` flow uses), and
//! emit the `{vertical, total, passed, failed, pass_rate, seeds}`
//! envelope the brief specifies.
//!
//! Determinism: `dry_run_actual` (in `eval_run`) synthesises the
//! agent output from the fixture's own `expected` clauses, and
//! `score_fixture` is pure. No LLM calls — the runner is
//! reproducible without provider credentials so vertical authors
//! can verify their seeds before shipping.
//!
//! Metric channel: every run writes a Prometheus-exposition
//! snippet (`vertical_eval_pass_rate{vertical=<id>}` plus the
//! count gauges) to stderr. Stderr keeps stdout pure so
//! `argo eval validate <id> --json | jq …` doesn't choke on
//! interleaved metric lines, while CI can still tee the metrics
//! to a file (`2>>metrics.prom`). The gateway pull-model
//! `/metrics` endpoint can't be addressed from a separate CLI
//! process, so this is the durable channel.
use super::eval_run::{collect_fixtures, dry_run_actual, load_vertical};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{score_fixture, FixtureStatus};
use serde_json::{json, Value};

/// Entry point invoked from `eval_run::validate_target` when the
/// CLI argument is not an existing file path.
pub(super) fn run(id: &str, json_out: bool) -> ArgoResult<()> {
    let (_registry, loaded) = load_vertical(id)?;
    let fixtures = collect_fixtures(&loaded)?;
    let total = fixtures.len();
    let mut seeds: Vec<Value> = Vec::with_capacity(total);
    let mut passed = 0_usize;
    for fixture in &fixtures {
        let actual = dry_run_actual(&fixture.expected);
        let mut row = fixture.raw.clone();
        if let Some(obj) = row.as_object_mut() {
            obj.insert("actual".into(), Value::String(actual.clone()));
        }
        let score = score_fixture(&row);
        let judged_pass = matches!(score.status, FixtureStatus::Scored) && score.passed;
        if judged_pass {
            passed += 1;
        }
        seeds.push(json!({
            "id": fixture.id,
            "suite": fixture.suite,
            "line": fixture.line,
            "input": fixture.input,
            "actual": actual,
            "passed": judged_pass,
            "status": fixture_status_label(score.status),
        }));
    }
    let failed = total.saturating_sub(passed);
    let pass_rate = if total == 0 {
        0.0_f64
    } else {
        passed as f64 / total as f64
    };

    emit_pass_rate_metric(id, pass_rate, total, passed, failed);
    print_envelope(id, json_out, total, passed, failed, pass_rate, &seeds)?;

    if total == 0 {
        return Err(ArgoError::Provider(format!(
            "vertical '{id}': no eval seeds declared in vertical.toml [evaluation]"
        )));
    }
    if failed > 0 {
        return Err(ArgoError::Schema(format!(
            "vertical '{id}': eval validate had {failed} failing fixture(s)"
        )));
    }
    Ok(())
}

fn print_envelope(
    id: &str,
    json_out: bool,
    total: usize,
    passed: usize,
    failed: usize,
    pass_rate: f64,
    seeds: &[Value],
) -> ArgoResult<()> {
    if json_out {
        let body = serde_json::to_string(&json!({
            "ok": failed == 0 && total > 0,
            "vertical": id,
            "total": total,
            "passed": passed,
            "failed": failed,
            "pass_rate": pass_rate,
            "seeds": seeds,
        }))
        .map_err(|e| ArgoError::Provider(format!("serialize eval validate output: {e}")))?;
        println!("{body}");
        return Ok(());
    }
    println!("vertical '{id}': eval validate");
    for seed in seeds {
        let marker = if seed["passed"].as_bool().unwrap_or(false) {
            "PASS"
        } else {
            "FAIL"
        };
        println!(
            "  [{marker}] {} ({})",
            seed["id"].as_str().unwrap_or("(unknown)"),
            seed["suite"].as_str().unwrap_or("(unknown)")
        );
    }
    println!("summary: {passed}/{total} passed (pass_rate={pass_rate:.4})");
    Ok(())
}

fn emit_pass_rate_metric(id: &str, pass_rate: f64, total: usize, passed: usize, failed: usize) {
    let safe_id = id.replace('"', "\\\"");
    eprintln!("# HELP vertical_eval_pass_rate Pass rate for `argo eval validate <vertical>`");
    eprintln!("# TYPE vertical_eval_pass_rate gauge");
    eprintln!("vertical_eval_pass_rate{{vertical=\"{safe_id}\"}} {pass_rate:.6}");
    eprintln!(
        "vertical_eval_seeds_total{{vertical=\"{safe_id}\"}} {total}\n\
         vertical_eval_seeds_passed{{vertical=\"{safe_id}\"}} {passed}\n\
         vertical_eval_seeds_failed{{vertical=\"{safe_id}\"}} {failed}"
    );
}

fn fixture_status_label(status: FixtureStatus) -> &'static str {
    match status {
        FixtureStatus::Scored => "scored",
        FixtureStatus::NoActual => "no_actual",
        FixtureStatus::Malformed => "malformed",
    }
}

#[cfg(test)]
mod tests {
    use super::{fixture_status_label, print_envelope};
    use argo_vertical::FixtureStatus;
    use serde_json::json;

    #[test]
    fn fixture_status_label_maps_each_variant() {
        assert_eq!(fixture_status_label(FixtureStatus::Scored), "scored");
        assert_eq!(fixture_status_label(FixtureStatus::NoActual), "no_actual");
        assert_eq!(fixture_status_label(FixtureStatus::Malformed), "malformed");
    }

    #[test]
    fn print_envelope_json_emits_required_fields() {
        let seeds = vec![json!({
            "id": "seed_a",
            "suite": "eval/seed.jsonl",
            "passed": true,
        })];
        // Smoke that serialize succeeds with the canonical inputs;
        // the brief's smoke test asserts the envelope shape end-to-
        // end against the real CLI.
        print_envelope("test-vert", true, 1, 1, 0, 1.0, &seeds).unwrap();
        print_envelope("test-vert", false, 1, 1, 0, 1.0, &seeds).unwrap();
    }
}
