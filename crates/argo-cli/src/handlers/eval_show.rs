//! `argo eval show`.
use super::eval_list::{load_run_arg, print_json};
use argo_types::ArgoResult;
use serde_json::json;

pub async fn run(run_id: &str, json_out: bool) -> ArgoResult<()> {
    let run = load_run_arg(run_id)?;
    if json_out {
        return print_json(&json!({"ok": true, "eval_run": run}));
    }
    println!("# Eval run {}", run.id);
    println!("- Vertical: {}", run.vertical);
    println!("- Created: {}", run.created_at);
    println!("- Fixtures: {}", run.fixtures.len());
    println!("- Pass rate: {:.0}%", run.pass_rate() * 100.0);
    println!();
    println!("| Fixture | Skill | Metric | Passed | Score |");
    println!("|---|---|---|---|---:|");
    for fixture in run.fixtures {
        println!(
            "| {} | {} | {} | {} | {:.2} |",
            fixture.fixture, fixture.skill, fixture.metric, fixture.passed, fixture.score
        );
    }
    Ok(())
}
