//! Gateway boot smoke for the autonomous duty scheduler.
mod common;

use argo_identity::WorkerId;
use argo_orchestrator::{DutyRecord, DutyStore, FireDecision};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{Duration as ChronoDuration, Utc};
use common::{argo_bin, gateway_lock_guard, http_get, pick_free_port};
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::test]
async fn gateway_boot_spawns_closed_gate_duty_scheduler() {
    let _gateway_lock = gateway_lock_guard();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let workspace = tempfile::tempdir().expect("workspace");
    let port = pick_free_port();
    let db_path = home.path().join("sessions.db");

    let store = Arc::new(SessionStore::open(&db_path).await.expect("open store"));
    store.migrate().await.expect("migrate");
    let duties = DutyStore::new(Arc::clone(&store));
    duties.ensure_schema().await.expect("duty schema");

    let mut duty = DutyRecord::new(
        TenantId::new(),
        WorkerId::new(),
        "* * * * * *",
        "review_queue",
        "test duty",
    );
    duty.created_at = Utc::now() - ChronoDuration::seconds(2);
    let duty_id = duty.id.clone();
    duties.insert(&duty).await.expect("insert duty");
    drop(duties);
    drop(store);

    let mut child = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .env("ARGO_DUTY_SCHEDULER_SECS", "1")
        .args([
            "gateway",
            "start",
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");

    let mut stderr = BufReader::new(child.stderr.take().expect("stderr"));
    await_gateway_listening(&mut stderr);

    let store = Arc::new(SessionStore::open(&db_path).await.expect("reopen store"));
    let duties = DutyStore::new(store);
    let fires = wait_for_fire(&duties, &duty_id).await;
    assert_eq!(fires.len(), 1);
    assert_eq!(fires[0].2, FireDecision::DeniedGate.label());

    let health = http_get(port, "/health");
    assert!(health.contains("HTTP/1.1 200 OK"), "health={health}");
    wait_for_success(child);
}

fn await_gateway_listening(stderr: &mut BufReader<impl Read>) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        stderr.read_line(&mut line).expect("read stderr");
        seen.push_str(&line);
        if line.contains("listening on") {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "listener did not announce in 10s; stderr={seen}"
        );
    }
}

async fn wait_for_fire(
    duties: &DutyStore,
    duty_id: &str,
) -> Vec<(argo_orchestrator::SessionId, chrono::DateTime<Utc>, String)> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let fires = duties
            .sessions_for_duty(duty_id)
            .await
            .expect("query duty fires");
        if !fires.is_empty() {
            return fires;
        }
        assert!(Instant::now() < deadline, "duty scheduler did not fire");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn wait_for_success(mut child: Child) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            assert!(status.success(), "gateway exit status: {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("gateway did not exit after max request");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
