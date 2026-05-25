//! Audit contract for the E2B-backed computer-use substrate.
//!
//! This deliberately stays offline: it uses the crate's public API,
//! source snapshots, and existing test-harness mocks. It must not open
//! a real E2B sandbox or call Anthropic.

use argo_executor_computer_use::anthropic::{CuStepResponse, StopReason};
use argo_executor_computer_use::budget::{BudgetSnapshot, BudgetTracker, BudgetVerdict};
use argo_executor_computer_use::config::DEFAULT_TOOL_VARIANT;
use argo_executor_computer_use::e2b::{ActionResult, CuAction, E2bCall, E2bClient};
use argo_executor_computer_use::pool::{SandboxPool, SandboxPoolKey};
use argo_executor_computer_use::restart::{
    restore_from, take_snapshot, CuRestartDecision, RestartCoordinator, RestartStrategy,
};
use argo_executor_computer_use::testing::{
    cu_harness_run, CannedResponse, MockE2bClient, ScriptedAction, TestScript,
};
use argo_executor_computer_use::{
    ComputerUseExecutor, CuConfig, CuError, CuOutcome, CuStepRunner, E2bReapReport, E2bSandboxInfo,
    Profile, ProfileRegistry, RunRequest, RunResult,
};
use chrono::{Duration as ChronoDuration, Utc};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

const EXPECTED_PUBLIC_MODULES: &[&str] = &[
    "pub mod anthropic;",
    "pub mod audit;",
    "pub mod budget;",
    "pub mod config;",
    "pub mod cu_step_runner;",
    "pub mod e2b;",
    "pub mod e2b_reap;",
    "pub mod e2b_reap_cron;",
    "pub mod executor;",
    "pub mod executor_types;",
    "pub mod learn;",
    "pub mod pool;",
    "pub mod profile;",
    "pub mod restart;",
    "pub mod sandbox_prewarm;",
    "pub mod screenshot_persist;",
    "pub mod success_metric;",
    "pub mod testing;",
    "pub mod testing_mocks;",
];

const EXPECTED_PUBLIC_REEXPORTS: &[&str] = &[
    "pub use config::CuConfig;",
    "pub use cu_step_runner::CuStepRunner;",
    "pub use e2b_reap::{E2bReapReport, E2bSandboxInfo};",
    "pub use error::CuError;",
    "pub use executor::{ComputerUseExecutor, CuOutcome, RunRequest, RunResult};",
    "pub use profile::{Profile, ProfileRegistry};",
];

const PR_2171_ALLOWED_CALLERS: &[&str] = &[
    "crates/argo-cli/src/handlers/action_replay_executors.rs",
    "crates/argo-cli/src/handlers/cu_replay.rs",
    "crates/argo-cli/src/handlers/cu_replay_tests.rs",
    "crates/argo-cli/src/handlers/cu_screenshots.rs",
    "crates/argo-cli/src/handlers/sandbox_e2b.rs",
    "crates/argo-cli/src/handlers/worker_actions_executor_cu_linux.rs",
    "crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs",
    "crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs",
];

const PRODUCTION_CALLER_CRATES: &[&str] = &["argo-cli", "argo-agent-runner", "argo-orchestrator"];

#[test]
fn lib_rs_public_surface_matches_snapshot() {
    let lib_rs = std::fs::read_to_string(crate_root().join("src/lib.rs"))
        .expect("read argo-executor-computer-use src/lib.rs");
    let modules = public_lines(&lib_rs, "pub mod ");
    let reexports = public_lines(&lib_rs, "pub use ");

    assert_eq!(
        modules, EXPECTED_PUBLIC_MODULES,
        "public module surface drifted; update this audit snapshot only with \
         an intentional substrate promotion or contraction"
    );
    assert_eq!(
        reexports, EXPECTED_PUBLIC_REEXPORTS,
        "lib.rs re-export surface drifted; update this audit snapshot only with \
         an intentional public API change"
    );
}

#[test]
fn public_reexports_compile_at_expected_paths() {
    fn assert_runner<T: CuStepRunner>() {}
    assert_runner::<E2bClient>();

    let config = CuConfig::for_tests("http://127.0.0.1:1".into(), "http://127.0.0.1:2".into());
    assert_eq!(config.tool_variant, DEFAULT_TOOL_VARIANT);
    let registry = ProfileRegistry::with_builtin_marketplace_browser();
    let _executor = ComputerUseExecutor::new(config, registry);

    let _profile = Profile::default();
    let _request = RunRequest::demo("act-surface", "marketplace_browser");
    let result = RunResult {
        final_text: Some("done".into()),
        actions_taken: vec!["step 1: click (1,2)".into()],
        tokens_used: 12,
    };
    let budget = BudgetSnapshot {
        tokens_used: 12,
        tokens_cap: 100,
        steps_used: 1,
        steps_cap: 5,
        wallclock_seconds_used: 0,
        wallclock_seconds_cap: 60,
    };
    let outcome = CuOutcome::Completed {
        result,
        bundle_dir: PathBuf::from("/tmp/act-surface"),
        budget,
    };
    assert!(matches!(outcome, CuOutcome::Completed { .. }));

    let _err = CuError::MissingEnv("E2B_API_KEY");
    let _report = E2bReapReport::default();
    let _info = E2bSandboxInfo {
        id: "sb-surface".into(),
        template: "skycot/test".into(),
        started_at: Utc::now(),
        last_activity_at: None,
        status: "running".into(),
    };
}

#[tokio::test]
async fn offline_harness_covers_current_action_config_and_budget_contracts() {
    let cfg = CuConfig::for_tests("http://e2b.invalid".into(), "http://anth.invalid".into());
    assert_eq!(cfg.e2b_base_url, "http://e2b.invalid");
    assert_eq!(cfg.anthropic_base_url, "http://anth.invalid");
    assert_eq!(cfg.per_request_timeout_seconds, 5);
    assert_eq!(cfg.tool_variant, DEFAULT_TOOL_VARIANT);

    let profile = Profile {
        max_tokens_per_action: 40,
        max_wallclock_seconds: 60,
        max_steps: 4,
        ..Profile::default()
    };
    let mut tracker = BudgetTracker::new(&profile).with_usd_cap(0.02, 1_000);
    assert_eq!(tracker.effective_token_cap(), 20);
    assert_eq!(tracker.charge_step(12), BudgetVerdict::Ok);
    assert!(matches!(
        tracker.charge_step(9),
        BudgetVerdict::Exhausted { reason } if reason.contains("token cap")
    ));

    let script = TestScript {
        profile,
        ..TestScript::new(
            vec![CannedResponse::Ok(vec![
                CuStepResponse::ToolCall {
                    action: CuAction::DoubleClick { x: 11, y: 12 },
                    tokens_used: 10,
                },
                CuStepResponse::ToolCall {
                    action: CuAction::Scroll {
                        x: 11,
                        y: 12,
                        dy: -4,
                    },
                    tokens_used: 0,
                },
                CuStepResponse::ToolCall {
                    action: CuAction::Navigate {
                        url: "https://example.com/done".into(),
                    },
                    tokens_used: 0,
                },
                CuStepResponse::Stop {
                    reason: StopReason::Success,
                    text: Some("complete".into()),
                    tokens_used: 0,
                },
            ])],
            vec![
                scripted(CuAction::DoubleClick { x: 11, y: 12 }, ok_response()),
                scripted(
                    CuAction::Scroll {
                        x: 11,
                        y: 12,
                        dy: -4,
                    },
                    ok_response(),
                ),
                scripted(
                    CuAction::Navigate {
                        url: "https://example.com/done".into(),
                    },
                    ActionResult {
                        ok: true,
                        text: Some("https://example.com/done".into()),
                        screenshot_b64: Some("UE5H".into()),
                    },
                ),
            ],
        )
    };

    let run = cu_harness_run(script).await;
    let outcome = run.outcome.expect("offline harness returns outcome");
    let CuOutcome::Completed { result, .. } = outcome else {
        panic!("expected Completed; got {outcome:?}");
    };
    assert_eq!(result.final_text.as_deref(), Some("complete"));
    assert_eq!(
        run.received_actions,
        vec![
            CuAction::DoubleClick { x: 11, y: 12 },
            CuAction::Scroll {
                x: 11,
                y: 12,
                dy: -4
            },
            CuAction::Navigate {
                url: "https://example.com/done".into()
            },
        ]
    );
    assert_eq!(
        run.screenshot_calls, 1,
        "inline screenshot from the final action should avoid a post-action HTTP fetch"
    );
}

#[tokio::test]
async fn pool_reaper_and_restart_contracts_stay_offline() {
    let client = E2bClient::new("http://127.0.0.1:1".into(), "test-key".into(), 1)
        .expect("reqwest client builds without network");
    let pool = SandboxPool::new(client).with_idle_ttl(StdDuration::from_millis(1));
    let key = SandboxPoolKey {
        tenant_id: "tenant-a".into(),
        persona_id: "persona-a".into(),
        profile: "marketplace_browser".into(),
    };
    pool.release(&key).await;
    assert_eq!(pool.occupancy().await, 0);
    assert_eq!(pool.reap_idle().await.expect("empty pool reaps"), 0);

    let now = Utc::now();
    let old = E2bSandboxInfo {
        id: "sb-old".into(),
        template: "skycot/test".into(),
        started_at: now - ChronoDuration::seconds(900),
        last_activity_at: None,
        status: "running".into(),
    };
    let fresh = E2bSandboxInfo {
        id: "sb-fresh".into(),
        template: "skycot/test".into(),
        started_at: now - ChronoDuration::seconds(10),
        last_activity_at: Some(now - ChronoDuration::seconds(10)),
        status: "running".into(),
    };
    let mock =
        MockE2bClient::scripted(Vec::new(), b"PNG".to_vec()).with_sandboxes(vec![old, fresh]);
    let report = mock
        .reap_idle(600)
        .await
        .expect("mock reaper stays offline");
    assert_eq!(report.scanned, 2);
    assert_eq!(report.reaped, vec!["sb-old".to_string()]);
    assert!(report.errors.is_empty());

    let mut retry = RestartCoordinator::new(RestartStrategy::RetryWithContext {
        max_attempts: 1,
        backoff_ms: 7,
    });
    assert_eq!(retry.backoff_ms(), 7);
    assert_eq!(
        retry.should_restart(&CuError::Protocol("model parse failed".into())),
        CuRestartDecision::Retry { with_context: true }
    );
    assert_eq!(retry.attempts(), 1);
    assert_eq!(
        retry.should_restart(&CuError::Protocol("model parse failed again".into())),
        CuRestartDecision::Bail
    );

    let mut reset = RestartCoordinator::new(RestartStrategy::SnapshotResetRetry {
        max_attempts: 2,
        snapshot_every_n_steps: 2,
    });
    assert!(!reset.should_snapshot(1));
    assert!(reset.should_snapshot(2));
    let snapshot = take_snapshot(
        b"PNG_BYTES",
        2,
        "Finish the checkout",
        &["clicked cart".into(), "typed address".into()],
    );
    reset.record_snapshot(snapshot);
    let mut prompt = "Base prompt".to_string();
    let mut screenshot = Vec::new();
    restore_from(
        reset.last_snapshot().expect("snapshot recorded"),
        &mut prompt,
        &mut screenshot,
    )
    .expect("non-empty snapshot restores");
    assert!(prompt.contains("RESTART CONTEXT"));
    assert_eq!(screenshot, b"PNG_BYTES");
}

#[test]
fn production_callers_match_pr_2171_allowlist() {
    let root = workspace_root();
    let observed = observed_callers_for("argo_executor_computer_use", &root);
    let expected: BTreeSet<String> = PR_2171_ALLOWED_CALLERS
        .iter()
        .map(|path| (*path).to_string())
        .collect();

    assert_eq!(
        observed, expected,
        "E2B computer-use production caller set drifted from PR #2171 allowlist"
    );
}

fn scripted(expected: CuAction, response: ActionResult) -> ScriptedAction {
    ScriptedAction { expected, response }
}

fn ok_response() -> ActionResult {
    ActionResult {
        ok: true,
        text: None,
        screenshot_b64: None,
    }
}

fn public_lines<'a>(contents: &'a str, prefix: &str) -> Vec<&'a str> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with(prefix))
        .collect()
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root()
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("crate lives under crates/argo-executor-computer-use")
}

fn observed_callers_for(rust_path: &str, root: &Path) -> BTreeSet<String> {
    let mut observed = BTreeSet::new();
    for caller in PRODUCTION_CALLER_CRATES {
        let src = root.join("crates").join(caller).join("src");
        for rel in collect_rs_files(&src, root) {
            let abs = root.join(&rel);
            let contents = match std::fs::read_to_string(&abs) {
                Ok(contents) => contents,
                Err(_) => continue,
            };
            if file_references_path(&contents, rust_path) {
                observed.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    observed
}

fn collect_rs_files(dir: &Path, relative_to: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            out.extend(collect_rs_files(&path, relative_to));
        } else if file_type.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
        {
            if let Ok(rel) = path.strip_prefix(relative_to) {
                out.push(rel.to_path_buf());
            }
        }
    }
    out
}

fn file_references_path(contents: &str, rust_path: &str) -> bool {
    let path_anchor = format!("{rust_path}::");
    if contents.contains(&path_anchor) {
        return true;
    }
    let extern_prefix = format!("extern crate {rust_path}");
    contents.lines().any(|line| {
        let Some(start) = line.find(&extern_prefix) else {
            return false;
        };
        let after = start + extern_prefix.len();
        matches!(
            line.as_bytes().get(after).copied(),
            None | Some(b';') | Some(b' ') | Some(b'\t')
        )
    })
}
