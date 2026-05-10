//! End-to-end worker-lifecycle smoke (mvp-worker-lifecycle-smoke).
//!
//! Drives the full v1.0 happy path through the real `argo` binary
//! against a fresh ARGO_HOME tempdir + a booted gateway:
//!
//!   gateway boot
//!   → `argo workspace init`
//!   → `argo worker create`           (persists a worker)
//!   → `argo memory append`            (emits a pending Ask, requires_approval=true)
//!   → `argo worker actions execute`   (1st — refused: approval required)
//!   → `argo worker actions approve`   (operator approval)
//!   → `argo worker actions execute`   (2nd — executor runs, writes bundle)
//!   → `argo worker actions complete`  (terminal state recorded)
//!   → assertions on the latest snapshot + transition history
//!
//! Failing this smoke = v1.0 cannot ship.
//!
//! ## Sensible deviations from the brief
//!
//! - **Approval surface.** The brief mentions `argo session approve`
//!   but `session approve` issues a session-scoped tool grant
//!   (`AuditKind::ApprovalGranted`) — the action-level Ask is
//!   resolved through `argo worker actions approve`, which is the
//!   surface a finance-AR operator actually uses. Used the latter.
//! - **Audit kind.** The brief asks for an audit row with
//!   `kind=ActionExecuted` and a matching session_id. No such
//!   `AuditKind` variant exists — the V3 ledger emits state-row
//!   snapshots into `memory_entry` (`schema_key=worker_action`) and
//!   the lifecycle is verified through `worker actions show --json`
//!   (latest state = completed) plus `worker actions status --json`
//!   (the full append-only `pending → approved → executing →
//!   completed` log).
//! - **Manifest fixture.** Uses the in-tree
//!   `verticals/finance-ar` `escalate_overdue` action
//!   (`requires_approval=true`, `executor="human"`) so the smoke
//!   stays offline + dependency-free. The brief explicitly allows
//!   this ("use `verticals/finance-ar` or a minimal in-test
//!   fixture").

mod common;

use common::listener_signal::{await_listener_signal, StderrDrain};
use common::{argo_bin, gateway_lock_guard, http_get, pick_free_port, GatewayLock};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const VERTICAL_ID: &str = "finance-ar";
const ACTION_TYPE: &str = "escalate_overdue";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
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

fn assert_success(label: &str, out: &std::process::Output) {
    assert!(
        out.status.success(),
        "{label} failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Gateway handle + listener-readiness drain + filesystem lock.
/// Drop kills the child so a panicking test never leaks the
/// gateway process. Per the brief: "Test cleans up gateway
/// process on exit (use `tests/common/listener_signal.rs`
/// pattern)."
struct Gateway {
    child: Child,
    _drain: StderrDrain,
    _lock: GatewayLock,
}

impl Drop for Gateway {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn boot_gateway(home: &Path, port: u16, lock: GatewayLock) -> Gateway {
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            // The boot path's `/sessions` probe consumes 1
            // request; the smoke makes no further HTTP calls so a
            // generous ceiling keeps the gateway alive until the
            // test drops the guard.
            "--max-requests",
            "32",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let drain = await_listener_signal(stderr).expect("gateway listener signal");
    let resp = http_get(port, "/sessions");
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "gateway store not ready: {resp}"
    );
    Gateway {
        child,
        _drain: drain,
        _lock: lock,
    }
}

#[test]
fn end_to_end_worker_lifecycle_completes_through_real_binary() {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let verticals = workspace_root().join("verticals");

    // 1. workspace init scaffolds argo.toml + .skycot/.
    let init = run_argo(home.path(), work.path(), &verticals, &["workspace", "init"]);
    assert_success("workspace init", &init);

    // 2. Boot the gateway against the SAME ARGO_HOME so the
    //    runtime + store come up end-to-end. The CLI subprocesses
    //    below operate on the same SQLite file (WAL journal
    //    permits the concurrency).
    let port = pick_free_port();
    let _gateway = boot_gateway(home.path(), port, gateway_lock_guard());

    // 3. Create a worker bound to the in-tree finance-ar vertical.
    let create = run_argo(
        home.path(),
        work.path(),
        &verticals,
        &[
            "worker",
            "create",
            "--vertical",
            VERTICAL_ID,
            "--persona",
            "Ari",
            "--charter",
            "Reply within 24h. Escalate after 45d past due.",
            "--json",
        ],
    );
    assert_success("worker create", &create);
    let create_json: Value =
        serde_json::from_slice(&create.stdout).expect("worker create JSON");
    let worker_id = create_json["id"]
        .as_str()
        .expect("worker create id")
        .to_string();
    assert_eq!(create_json["vertical_id"], VERTICAL_ID);

    // 4. Emit a pending action that requires approval (the "Ask").
    //    `worker_actions_emit` in argo-cli is `pub(crate)`, so
    //    tests append the same on-disk memory-row contract
    //    directly — same shape `e2e_v1_demo.rs` and
    //    `worker_actions.rs::append_action` already lock against.
    let action_id = uuid::Uuid::now_v7().to_string();
    let pending = serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "lifecycle-smoke-job",
        "worker_id": worker_id,
        "action_type": ACTION_TYPE,
        "risk_level": "high",
        "requires_approval": true,
        "state": "pending",
        "payload": {
            "invoice_id": "INV-42",
            "customer": "Acme Corp",
            "amount_usd": 1234,
            "days_past_due": 47,
            "last_reminder_at": "2026-04-30T15:00:00Z",
        },
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    let pending_body = serde_json::to_string(&pending).expect("ser pending");
    let append = run_argo(
        home.path(),
        work.path(),
        &verticals,
        &[
            "memory",
            "append",
            "memory",
            &pending_body,
            "--vertical",
            VERTICAL_ID,
        ],
    );
    assert_success("memory append (pending action)", &append);

    // 5. First execute attempt MUST refuse — that's the Ask. The
    //    state stays `pending`; no transition row is written.
    let pre_approve = run_argo(
        home.path(),
        work.path(),
        &verticals,
        &["worker", "actions", "execute", &action_id, "--json"],
    );
    assert!(
        !pre_approve.status.success(),
        "execute on pending+requires_approval must Ask, got: stdout={} stderr={}",
        String::from_utf8_lossy(&pre_approve.stdout),
        String::from_utf8_lossy(&pre_approve.stderr),
    );
    let pre_stderr = String::from_utf8_lossy(&pre_approve.stderr);
    assert!(
        pre_stderr.contains("approval required"),
        "expected 'approval required' diagnostic in stderr, got:\n{pre_stderr}"
    );

    // 6. Operator approves. `pending → approved`.
    let approve = run_argo(
        home.path(),
        work.path(),
        &verticals,
        &[
            "worker",
            "actions",
            "approve",
            &action_id,
            "--approved-by",
            "operator@test",
            "--note",
            "smoke-approved",
            "--json",
        ],
    );
    assert_success("worker actions approve", &approve);

    // 7. Re-execute → succeeds. `approved → executing`. The human
    //    executor drops a per-action bundle dir under
    //    `<work>/.skycot/outputs/<job>/actions/<action_id>/`.
    let exec = run_argo(
        home.path(),
        work.path(),
        &verticals,
        &["worker", "actions", "execute", &action_id, "--json"],
    );
    assert_success("worker actions execute (post-approve)", &exec);
    let exec_json: Value = serde_json::from_slice(&exec.stdout).expect("execute JSON");
    assert_eq!(exec_json["state"], "executing", "got: {exec_json}");
    let bundle = exec_json["execution_bundle_path"]
        .as_str()
        .expect("execution_bundle_path");
    let bundle_dir = Path::new(bundle);
    assert!(
        bundle_dir.join("instructions.md").exists(),
        "executor must drop instructions.md into the per-action bundle dir at {bundle}",
    );
    assert!(
        bundle_dir.join("action.json").exists(),
        "executor must drop action.json into the per-action bundle dir at {bundle}",
    );

    // 8. Operator records completion. `executing → completed`.
    let complete = run_argo(
        home.path(),
        work.path(),
        &verticals,
        &[
            "worker",
            "actions",
            "complete",
            &action_id,
            "--note",
            "escalated to finance lead",
            "--result-json",
            r#"{"escalated_to":"finance-lead","channel":"slack"}"#,
            "--json",
        ],
    );
    assert_success("worker actions complete", &complete);

    // 9. Latest snapshot reflects the terminal state.
    let show = run_argo(
        home.path(),
        work.path(),
        &verticals,
        &["worker", "actions", "show", &action_id, "--json"],
    );
    assert_success("worker actions show", &show);
    let show_json: Value = serde_json::from_slice(&show.stdout).expect("show JSON");
    assert_eq!(show_json["state"], "completed", "got: {show_json}");
    assert_eq!(show_json["action_id"], action_id);
    assert_eq!(show_json["action_type"], ACTION_TYPE);
    assert_eq!(show_json["worker_id"], worker_id);

    // 10. The full append-only transition log must show every step.
    //     This is the V3 ledger's stand-in for the brief's
    //     `kind=ActionExecuted` audit row — a per-`action_id`
    //     snapshot trail the operator can replay.
    let status = run_argo(
        home.path(),
        work.path(),
        &verticals,
        &["worker", "actions", "status", &action_id, "--json"],
    );
    assert_success("worker actions status", &status);
    let states: Vec<String> = String::from_utf8_lossy(&status.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| {
            v.get("state")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .collect();
    assert_eq!(
        states,
        vec!["pending", "approved", "executing", "completed"],
        "expected the full append-only state-transition log; got {states:?}"
    );
}
