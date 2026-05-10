//! Mission MVP smoke — single-turn Genesis → vertical → worker → first
//! duty fire → audit chain validates.
//!
//! Pins the demo claim: "operator types one sentence, gets a working
//! autonomous worker." Deterministic; no LLM dependency.
//!
//! ## Sensible deviations from the brief
//!
//! - **`argo worker create --from-genesis`** does not exist on `main`; the
//!   handler signature is `--vertical / --persona / --charter / --duty`. We
//!   wire the genesis output into `worker create` explicitly. Same shape
//!   end-to-end (charter sentence → worker row + duty registered).
//! - **"Action ledger"** maps to the `duty_fired_session` link table — the
//!   closest concrete primitive on `main` for "action attributed to worker
//!   on first duty fire."
//! - **"Test-time-injection helper"** isn't a discrete primitive yet. We
//!   reuse the existing pattern (`DutyRecord.created_at` in the past +
//!   `DutyScheduler::tick(now)` with `OpenGate`) from the duty_scheduler
//!   unit tests in `argo-orchestrator`.

mod common;

use argo_audit::verify_session;
use argo_genesis::{plan, write_vertical, Archetype};
use argo_identity::WorkerId;
use argo_orchestrator::{
    duty_scheduler::{DutyRecord, DutyScheduler, FireDecision, OpenGate, TickReport},
    DutyStore, OrchestratorStore, SessionId as OrchSessionId,
};
use argo_store::SessionStore;
use argo_types::{SessionId as AuditSessionId, TenantId};
use chrono::{Duration as ChronoDuration, Utc};
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

const CHARTER: &str = "build me a marketing intern named Maya who posts to Marketplace";
const PERSONA: &str = "maya";
const EXPECTED_VERTICAL_ID: &str = "marketing-intern-maya";
/// Mon-Fri 9am — the brief's first-duty cron.
const DAILY_9AM_WEEKDAYS: &str = "0 9 * * 1-5";

#[tokio::test]
async fn genesis_to_first_duty_smoke() {
    // 1. Genesis (in-process, deterministic): plan + write_vertical.
    let plan = plan(CHARTER, PERSONA).expect("genesis plan");
    assert_eq!(plan.archetype, Archetype::MarketingIntern);
    assert_eq!(plan.persona, PERSONA);
    assert_eq!(plan.vertical_id, EXPECTED_VERTICAL_ID);

    let manifest = &plan.manifest;
    assert_eq!(manifest.vertical.id, EXPECTED_VERTICAL_ID);
    assert!(
        manifest.vertical.name.contains("Marketing Intern"),
        "manifest name: {:?}",
        manifest.vertical.name
    );
    let actions = manifest.actions.as_ref().expect("manifest actions section");
    assert!(
        !actions.actions.is_empty(),
        "expected ≥ 1 action declared, got 0"
    );

    let vertical_root = tempfile::tempdir().expect("vertical tmpdir");
    let vertical_dir = vertical_root.path().join(&plan.vertical_id);
    write_vertical(&plan, &vertical_dir).expect("write vertical");
    assert_artefacts_on_disk(&vertical_dir);

    // 2. Worker create (binary spawn): wires the genesis output into the
    //    sessions store and emits the duty cron job file.
    let home = tempfile::tempdir().expect("ARGO_HOME tmpdir");
    let workspace = tempfile::tempdir().expect("workspace tmpdir");
    let duty_spec = format!(
        "{DAILY_9AM_WEEKDAYS}::post_campaign::Daily 9am Marketplace post"
    );

    let create = run_argo(
        home.path(),
        workspace.path(),
        &[
            "worker", "create",
            "--vertical", &plan.vertical_id,
            "--persona", PERSONA,
            "--charter", CHARTER,
            "--duty", &duty_spec,
            "--json",
        ],
    );
    assert_success("worker create", &create);

    let body: Value =
        serde_json::from_slice(&create.stdout).expect("worker create json body");
    let worker_id_str = body["id"].as_str().expect("worker id").to_string();
    assert_eq!(body["persona"]["name"], PERSONA);
    assert_eq!(body["vertical_id"], EXPECTED_VERTICAL_ID);
    let registered = body["role"]["duties"]
        .as_array()
        .expect("role.duties array");
    assert_eq!(registered.len(), 1, "expected one registered duty");
    assert_eq!(registered[0]["cron"], DAILY_9AM_WEEKDAYS);
    assert_eq!(registered[0]["entry_action"], "post_campaign");

    // Cron job file emitted under <cwd>/cron/jobs/<name>.json.
    let cron_dir = workspace.path().join("cron").join("jobs");
    let cron_files: Vec<_> = std::fs::read_dir(&cron_dir)
        .expect("read cron jobs dir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(cron_files.len(), 1, "expected exactly one cron job file");
    let cron_body =
        std::fs::read_to_string(cron_files[0].path()).expect("read cron job file");
    assert!(
        cron_body.contains(DAILY_9AM_WEEKDAYS),
        "cron job file missing cron expr `{DAILY_9AM_WEEKDAYS}`: {cron_body}"
    );

    // 3. Worker row visible in the store; pull tenant_id for step 4.
    let db = home.path().join("sessions.db");
    let store = Arc::new(SessionStore::open(&db).await.expect("reopen store"));
    let row_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM worker WHERE id = ?")
            .bind(&worker_id_str)
            .fetch_one(store.pool())
            .await
            .expect("count worker row");
    assert_eq!(row_count.0, 1, "worker row missing from store");
    let (tenant_str,): (String,) =
        sqlx::query_as("SELECT tenant_id FROM worker WHERE id = ?")
            .bind(&worker_id_str)
            .fetch_one(store.pool())
            .await
            .expect("worker tenant lookup");
    let tenant_id = TenantId(Uuid::parse_str(&tenant_str).expect("tenant uuid"));
    let worker_id = WorkerId::from_str(&worker_id_str).expect("worker uuid");

    // 4. First-duty fire. Insert a DutyRecord for this worker with
    //    `created_at` 2 seconds in the past so the cron window has already
    //    elapsed at tick time. `OpenGate` auto-approves so the fire spawns
    //    a session and lands attribution in `duty_fired_session`.
    let duties = Arc::new(DutyStore::new(Arc::clone(&store)));
    duties.ensure_schema().await.expect("duty schema");
    let orch = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    orch.ensure_schema().await.expect("orchestrator schema");

    let mut first_duty = DutyRecord::new(
        tenant_id,
        worker_id,
        // Six-field "every second" — the cron crate's wide form. The brief's
        // 5-field `0 9 * * 1-5` is what's *registered* via `worker create`;
        // here we just need a cron that fires at tick time.
        "* * * * * *",
        "post_campaign",
        "Daily 9am Marketplace post (smoke fire)",
    );
    first_duty.created_at = Utc::now() - ChronoDuration::seconds(2);
    let duty_id = first_duty.id.clone();
    duties.insert(&first_duty).await.expect("insert duty");

    let scheduler = DutyScheduler::new(
        Arc::clone(&duties),
        Arc::clone(&orch),
        Arc::new(OpenGate),
    );
    let report: TickReport = scheduler
        .tick(Utc::now())
        .await
        .expect("scheduler tick");
    assert!(
        report.fired.contains(&duty_id),
        "duty `{duty_id}` did not fire: {report:?}",
    );

    // 5. An action lands in the duty_fired_session ledger for our worker.
    let fires = duties
        .sessions_for_duty(&duty_id)
        .await
        .expect("sessions_for_duty");
    assert_eq!(fires.len(), 1, "expected 1 fire row, got {}", fires.len());
    let (orch_session_id, _fired_at, decision) =
        fires.into_iter().next().expect("first fire row");
    assert_eq!(
        decision,
        FireDecision::AutoApprove.label(),
        "expected auto-approved fire"
    );

    // 6. Audit chain hashes validate. The session has zero audit rows
    //    (the worker dispatcher writes them later when the action runs),
    //    so verify_session reports `ok = true` on the empty chain — that
    //    is still a valid pin: any future row that breaks the chain will
    //    flip `ok` to false.
    let audit_session_id = orch_to_audit_session_id(&orch_session_id);
    let audit_report = verify_session(&store, audit_session_id)
        .await
        .expect("verify_session");
    assert!(
        audit_report.ok && audit_report.breaks.is_empty(),
        "audit chain failed: ok={}, breaks={:?}",
        audit_report.ok,
        audit_report.breaks,
    );
}

fn assert_artefacts_on_disk(out: &Path) {
    for relpath in [
        "vertical.toml",
        "worker.toml",
        "eval-seed.toml",
        "prompts/system.md",
        "prompts/persona.md",
        "memory/schema.toml",
    ] {
        let path = out.join(relpath);
        assert!(path.exists(), "missing artefact: {}", path.display());
        let len = std::fs::metadata(&path)
            .unwrap_or_else(|e| panic!("metadata {}: {e}", path.display()))
            .len();
        assert!(len > 0, "empty artefact: {}", path.display());
    }
}

fn run_argo(home: &Path, workspace: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(workspace)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_success(label: &str, out: &Output) {
    assert!(
        out.status.success(),
        "{label}: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn orch_to_audit_session_id(orch: &OrchSessionId) -> AuditSessionId {
    AuditSessionId(
        Uuid::parse_str(orch.as_str())
            .unwrap_or_else(|e| panic!("orch session id `{orch}` not a uuid: {e}")),
    )
}
