use argo_identity::{AutoApprovalPolicy, IdentityStore, Persona, Role, Worker};
use argo_memory::{MemoryFile, MemoryStore};
use argo_orchestrator::{EscalationStep, Handoff, OrchestratorStore, Session};
use argo_store::{anomaly, SessionStore};
use chrono::Utc;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn verticals_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .join("verticals")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

#[test]
fn worker_portrait_renders_json_and_markdown_sections() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".skycot")).unwrap();
    std::fs::write(
        workspace.path().join(".skycot/auto_approval.toml"),
        "enabled = true\ndaily_budget_usd_micros = 5000000\n",
    )
    .unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (tenant, worker_id) = rt.block_on(seed(home.path(), workspace.path()));
    let tenant = tenant.to_string();

    let out = run(
        workspace.path(),
        home.path(),
        &["worker", "portrait", &worker_id, "--tenant", &tenant, "--json"],
    );
    assert_success(&out);
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["ok"], true);
    let portrait = &body["worker_portrait"];
    assert_eq!(portrait["identity"]["persona"], "Maya");
    assert_eq!(portrait["budget"]["spend_today_usd_micros"], 1_250_000);
    assert_eq!(portrait["recent_action_stats"]["completed"], 1);
    assert_eq!(portrait["recent_action_stats"]["failed"], 1);
    assert_eq!(portrait["open_handoffs"].as_array().unwrap().len(), 1);
    assert_eq!(portrait["skill_candidates"].as_array().unwrap().len(), 1);
    assert_eq!(portrait["anomalies"].as_array().unwrap().len(), 1);
    assert_eq!(portrait["circuit_breaker"]["state"], "closed");

    let md = run(
        workspace.path(),
        home.path(),
        &["worker", "portrait", &worker_id, "--tenant", &tenant],
    );
    assert_success(&md);
    let stdout = String::from_utf8_lossy(&md.stdout);
    assert!(stdout.contains("## Identity"));
    assert!(stdout.contains("## Anomalies (24h)"));
    assert!(!stdout.contains('\u{1b}'));
}

async fn seed(home: &Path, workspace: &Path) -> (argo_types::TenantId, String) {
    let store = Arc::new(SessionStore::open(home.join("sessions.db")).await.unwrap());
    store.migrate().await.unwrap();
    let root = std::fs::canonicalize(workspace).unwrap();
    let name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let (tenant, ws) = store.ensure_workspace(name, &root.to_string_lossy()).await.unwrap();
    let identities = IdentityStore::new(&store);
    let mut worker = Worker::draft(
        tenant,
        "gtm-engineer",
        Persona::named("Maya", "operator@example.test"),
        Role::new("Own approved marketplace follow-up."),
    );
    worker.auto_approval = AutoApprovalPolicy {
        enabled: true,
        daily_budget_usd_micros: Some(5_000_000),
        ..Default::default()
    };
    identities.create_worker(&worker).await.unwrap();
    seed_budget(&store, &worker.id.to_string()).await;
    seed_actions(&store, ws, &worker.id.to_string()).await;
    seed_handoff(&store, tenant, worker.id).await;
    seed_candidate(&store, ws, &worker.id.to_string()).await;
    seed_anomaly(&store, &worker.id.to_string()).await;
    (tenant, worker.id.to_string())
}

async fn seed_budget(store: &SessionStore, worker_id: &str) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS worker_budget (
            tenant_id BLOB NOT NULL,
            worker_id TEXT NOT NULL,
            action_id TEXT NOT NULL,
            usd_micros INTEGER NOT NULL,
            charged_at INTEGER NOT NULL,
            PRIMARY KEY (tenant_id, worker_id, action_id, charged_at)
        )",
    )
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at)
         VALUES (?, ?, 'act-budget', 1250000, ?)",
    )
    .bind(vec![0_u8; 16])
    .bind(worker_id)
    .bind(Utc::now().timestamp())
    .execute(store.pool())
    .await
    .unwrap();
}

async fn seed_actions(store: &Arc<SessionStore>, ws: argo_types::WorkspaceId, worker_id: &str) {
    let memory = MemoryStore::new(Arc::clone(store));
    for (action_id, state, secs, evidence) in [
        ("act-ok", "completed", 2, json!([])),
        ("act-fail", "failed", 1, json!(["argo://action/old"])),
    ] {
        let now = Utc::now();
        let body = json!({
            "schema_key":"worker_action","action_id":action_id,"job_id":"job",
            "worker_id":worker_id,"action_type":"post_listing","risk_level":"low",
            "requires_approval":false,"state":state,"payload":{},
            "executed_at":(now - chrono::Duration::seconds(secs)).to_rfc3339(),
            "evidence":evidence,"created_at":now.to_rfc3339()
        });
        memory
            .append_for_vertical(ws, MemoryFile::Memory, body.to_string(), None, "gtm-engineer")
            .await
            .unwrap();
    }
}

async fn seed_handoff(store: &Arc<SessionStore>, tenant: argo_types::TenantId, to: argo_identity::WorkerId) {
    let orch = OrchestratorStore::new(Arc::clone(store));
    let session = Session::new(tenant, "handoff smoke");
    orch.insert_session(&session).await.unwrap();
    let handoff = Handoff::new(
        session.id,
        argo_identity::WorkerId::new(),
        EscalationStep::Worker(to),
        "finish review",
        json!({"case":"portrait"}),
    );
    orch.insert_handoff(tenant, &handoff).await.unwrap();
}

async fn seed_candidate(store: &Arc<SessionStore>, ws: argo_types::WorkspaceId, worker_id: &str) {
    let memory = MemoryStore::new(Arc::clone(store));
    let body = json!({"schema_key":"learned_skill_candidates","skill_name":"post_listing_followup","worker_id":worker_id,"problem":"repeatable follow-up"});
    memory
        .append_for_vertical(ws, MemoryFile::Memory, body.to_string(), None, "gtm-engineer")
        .await
        .unwrap();
}

async fn seed_anomaly(store: &SessionStore, worker_id: &str) {
    let baseline = anomaly::BehaviorBaseline {
        worker_id: worker_id.into(),
        action_kind: "post_listing".into(),
        latency_mean: 100.0,
        latency_m2: 25.0,
        retry_mean: 0.0,
        retry_m2: 1.0,
        sample_count: 8,
        updated_at: Utc::now(),
    };
    anomaly::insert_event(
        store.pool(),
        worker_id,
        "act-spike",
        "post_listing",
        "latency_ms",
        5000.0,
        &baseline,
        4.0,
        3.0,
        Utc::now(),
    )
    .await
    .unwrap();
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
