//! End-to-end smoke for `argo forge cost-report` — Stream
//! `mission-forge-cost-attribution`.
//!
//! Persists three `ForgeRun` rows with known costs, then shells out
//! to the `argo forge cost-report --json` CLI and asserts the
//! per-skill aggregation matches the seeded values (count of runs,
//! total cost, mean per run, llm / e2b split).

mod common;

use argo_forge::{
    accumulate_e2b_seconds, accumulate_llm_call, persist_forge_run, ForgeOptions, ForgeRun,
};
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn cost_report_aggregates_seeded_runs_per_skill() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store
        .ensure_workspace("forge-cost-smoke", &work.path().to_string_lossy())
        .await
        .unwrap();

    // Two runs on `alpha`, one on `beta`. Costs picked so the
    // arithmetic is easy to eyeball: each `alpha` run = 10_500
    // USD-micros LLM (3 $/MTok × 1000 input + 15 $/MTok × 500
    // output) + 1_450 USD-micros e2b (5s × 290 USD-micros/s);
    // each `beta` run = 4_500 USD-micros LLM (3 × 1000 + 15 × 100)
    // + 0 e2b.
    let mut r1 = ForgeRun::pending(&ForgeOptions::new("alpha", "0.1"));
    accumulate_llm_call(&mut r1.cost, "claude-sonnet-4-6", 1_000, 500, 0);
    accumulate_e2b_seconds(&mut r1.cost, 5.0);
    persist_forge_run(&store, &r1).await.unwrap();

    let mut r2 = ForgeRun::pending(&ForgeOptions::new("alpha", "0.1"));
    accumulate_llm_call(&mut r2.cost, "claude-sonnet-4-6", 1_000, 500, 0);
    accumulate_e2b_seconds(&mut r2.cost, 5.0);
    persist_forge_run(&store, &r2).await.unwrap();

    let mut r3 = ForgeRun::pending(&ForgeOptions::new("beta", "0.1"));
    accumulate_llm_call(&mut r3.cost, "claude-sonnet-4-6", 1_000, 100, 0);
    persist_forge_run(&store, &r3).await.unwrap();

    // CLI invocation — `--json` returns a single envelope.
    let out = run(&home, work.path(), &["forge", "cost-report", "--json"]);
    assert_success(&out);
    let env: Value = serde_json::from_slice(&out.stdout).unwrap();
    let rows = env["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 2, "expected 2 skill rollups, got {rows:?}");

    // Highest total first — alpha (2 runs × (10_500 + 1_450) = 23_900)
    // beats beta (1 run × 4_500).
    let alpha = &rows[0];
    assert_eq!(alpha["skill_name"], "alpha");
    assert_eq!(alpha["run_count"], 2);
    assert_eq!(alpha["total_cost_usd_micros"], 23_900);
    assert_eq!(alpha["llm_cost_usd_micros"], 21_000);
    assert_eq!(alpha["e2b_cost_usd_micros"], 2_900);
    assert_eq!(alpha["mean_cost_usd_micros"], 11_950);
    assert_eq!(alpha["llm_input_tokens"], 2_000);
    assert_eq!(alpha["llm_output_tokens"], 1_000);

    let beta = &rows[1];
    assert_eq!(beta["skill_name"], "beta");
    assert_eq!(beta["run_count"], 1);
    assert_eq!(beta["total_cost_usd_micros"], 4_500);

    // Totals envelope sums across rows.
    let totals = &env["totals"];
    assert_eq!(totals["run_count"], 3);
    assert_eq!(totals["total_cost_usd_micros"], 23_900 + 4_500);
    assert_eq!(totals["llm_cost_usd_micros"], 21_000 + 4_500);
    assert_eq!(totals["e2b_cost_usd_micros"], 2_900);

    // --skill filter narrows the aggregation.
    let out = run(
        &home,
        work.path(),
        &["forge", "cost-report", "--skill", "beta", "--json"],
    );
    assert_success(&out);
    let env: Value = serde_json::from_slice(&out.stdout).unwrap();
    let rows = env["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "skill=beta should yield 1 row");
    assert_eq!(rows[0]["skill_name"], "beta");
    assert_eq!(env["skill"], "beta");
}

fn run(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
