//! Multi-vertical isolation integration test (#741).
//!
//! Headline customer-facing safety claim: actions emitted by a
//! worker under vertical A are NEVER visible to commands
//! scoped to vertical B. Memory rows carry their `vertical_id`;
//! `actions list --vertical X` filters at the SQL level.
//!
//! This test builds a tempdir verticals directory containing
//! gtm-engineer (the launch vertical) AND a fixture
//! `demo-shop` vertical with a different action vocabulary.
//! Runs a worker job under gtm-engineer in mock mode (which
//! emits 2 actions), then asserts:
//!
//! 1. `actions list --vertical gtm-engineer` returns the 2
//!    emitted rows.
//! 2. `actions list --vertical demo-shop` returns 0 rows —
//!    cross-vertical isolation holds.
//! 3. `actions list --state all` (no filter) walks both
//!    verticals and still only finds the 2 rows (because
//!    nothing emitted under demo-shop).

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// Build a tempdir that contains BOTH the production
/// gtm-engineer vertical (copied from in-tree) AND a fixture
/// `demo-shop` vertical with its own action vocabulary so the
/// isolation test has two real namespaces to exercise.
fn build_two_vertical_dir() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("verticals tempdir");
    // Copy gtm-engineer from the in-tree workspace.
    let src = workspace_root().join("verticals").join("gtm-engineer");
    let dst = root.path().join("gtm-engineer");
    copy_dir(&src, &dst);
    // Synthesise demo-shop fixture with a different action.
    let demo = root.path().join("demo-shop");
    std::fs::create_dir_all(demo.join("prompts")).unwrap();
    std::fs::write(
        demo.join("prompts").join("system.md"),
        "Demo shop fixture vertical for isolation test.\n",
    )
    .unwrap();
    std::fs::write(demo.join("prompts").join("persona.md"), "Demo persona.\n").unwrap();
    std::fs::write(
        demo.join("vertical.toml"),
        r#"schema_version = "1"

[vertical]
id          = "demo-shop"
name        = "Demo Shop"
description = "Fixture vertical exercising cross-vertical isolation."
version     = "0.0.1"
authors     = ["test"]
tags        = ["test"]

[role]
system_prompt = "prompts/system.md"
persona       = "prompts/persona.md"

[provider]
default   = "mock"
fallback  = "mock"
cost_tier = "balanced"

[tools]
allowed = ["web_search", "web_extract"]
denied  = []

[actions.send_promo_email]
risk              = "high"
requires_approval = true
description       = "Demo-shop-only action; gtm-engineer should never see this."
executor          = "human"
"#,
    )
    .unwrap();
    root
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to);
        } else {
            std::fs::copy(&from, &to).unwrap();
        }
    }
}

fn run_argo(home: &Path, work: &Path, verticals: &Path, args: &[&str]) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .current_dir(work)
        .output()
        .expect("spawn argo")
}

fn list_actions_for(
    home: &Path,
    work: &Path,
    verticals: &Path,
    vertical: Option<&str>,
) -> Vec<Value> {
    let mut args = vec!["worker", "actions", "list", "--state", "all", "--json"];
    if let Some(v) = vertical {
        args.push("--vertical");
        args.push(v);
    }
    let out = run_argo(home, work, verticals, &args);
    assert!(
        out.status.success(),
        "list: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[test]
fn actions_emitted_under_gtm_engineer_are_invisible_to_demo_shop() {
    let verticals = build_two_vertical_dir();
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");

    // Bootstrap workspace.
    let init = run_argo(
        home.path(),
        work.path(),
        verticals.path(),
        &["workspace", "init"],
    );
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    // Confirm both verticals are loaded.
    let v_list = run_argo(
        home.path(),
        work.path(),
        verticals.path(),
        &["vertical", "list"],
    );
    assert!(v_list.status.success());
    let stdout = String::from_utf8_lossy(&v_list.stdout);
    assert!(stdout.contains("gtm-engineer"), "got:\n{stdout}");
    assert!(stdout.contains("demo-shop"), "got:\n{stdout}");

    // Run a worker job under gtm-engineer (mock mode emits 2
    // actions: verify_price + post_listing).
    let _add = run_argo(
        home.path(),
        work.path(),
        verticals.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "isolation smoke",
            "--json",
        ],
    );
    let serve = run_argo(
        home.path(),
        work.path(),
        verticals.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--once",
            "--max-steps",
            "1",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "serve: {}",
        String::from_utf8_lossy(&serve.stderr)
    );

    // 1. Scoped to gtm-engineer — both actions visible.
    let gtm_rows = list_actions_for(
        home.path(),
        work.path(),
        verticals.path(),
        Some("gtm-engineer"),
    );
    assert_eq!(
        gtm_rows.len(),
        2,
        "gtm-engineer should see 2 action rows; got: {gtm_rows:?}"
    );

    // 2. Scoped to demo-shop — zero rows visible. Cross-vertical
    //    isolation: an action emitted under gtm-engineer is NOT
    //    visible to a list scoped to demo-shop, even though both
    //    verticals are loaded in the same workspace.
    let demo_rows = list_actions_for(
        home.path(),
        work.path(),
        verticals.path(),
        Some("demo-shop"),
    );
    assert_eq!(
        demo_rows.len(),
        0,
        "demo-shop must NOT see gtm-engineer's actions; got: {demo_rows:?}"
    );

    // 3. Unscoped — sees the 2 gtm-engineer rows (none from
    //    demo-shop because nothing emitted there).
    let all_rows = list_actions_for(home.path(), work.path(), verticals.path(), None);
    assert_eq!(
        all_rows.len(),
        2,
        "unscoped list across both verticals should still find only the 2 emitted rows; got: {all_rows:?}"
    );
    // Every row's job_id matches the gtm-engineer job we ran.
    let serve_out = String::from_utf8_lossy(&serve.stdout);
    let job_id = serve_out
        .lines()
        .find_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            v["job_id"].as_str().map(String::from)
        })
        .expect("job_id in serve --json");
    for row in &all_rows {
        assert_eq!(
            row["job_id"].as_str(),
            Some(job_id.as_str()),
            "row job_id should match gtm-engineer job; got: {row:?}"
        );
    }
}
