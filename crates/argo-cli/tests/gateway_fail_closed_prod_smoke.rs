//! Smoke coverage for production bearer fail-closed behavior and the
//! legacy first-run wizard bootstrap-token gate.

mod common;

use common::listener_signal::await_listener_signal;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn status_line(resp: &str) -> &str {
    resp.lines().next().unwrap_or("")
}

fn http(port: u16, path: &str, extra_headers: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    for (name, value) in extra_headers {
        req.push_str(&format!("{name}: {value}\r\n"));
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn wait_with_timeout(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            return child.wait().expect("wait after kill");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn fail_closed_output(home: &Path, port: u16, envs: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home)
        .env_remove("ARGO_GATEWAY_PROFILE")
        .env_remove("ARGO_GATEWAY_BIND")
        .env_remove("ARGO_GATEWAY_BEARER_TOKEN")
        .current_dir(home)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--profile-startup",
            "--skip-startup-check",
        ]);
    for (name, value) in envs {
        cmd.env(name, value);
    }
    cmd.output().expect("run gateway start")
}

fn spawn_gateway(
    home: &Path,
    port: u16,
    max_requests: u32,
) -> (Child, common::listener_signal::StderrDrain) {
    // NOTE (`mission-audit-sqlcipher-enterprise-default`): the wizard
    // bootstrap-token gate is orthogonal to production mode (the
    // `X-Argo-Bootstrap` check fires in either profile), so this test
    // runs the gateway in dev mode. Production-mode rejection of a
    // stock (non-SQLCipher) build is covered by
    // `production_profile_without_bearer_fails_before_listen` plus
    // the unit tests in `argo-store::store_open_enterprise_tests`.
    // Setting `ARGO_GATEWAY_PROFILE=production` here would deadlock
    // against the new fail-closed encryption gate (stock build +
    // production mode = refuse to boot, even with a real env key).
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env_remove("ARGO_GATEWAY_PROFILE")
        .env_remove("ARGO_GATEWAY_BIND")
        .env("ARGO_GATEWAY_BEARER_TOKEN", "prod-token")
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .current_dir(home)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let drain = await_listener_signal(stderr).expect("gateway listener signal");
    (child, drain)
}

#[test]
fn production_profile_without_bearer_fails_before_listen() {
    let _gateway_lock = gateway_lock_guard();
    let home = tempfile::tempdir().expect("tempdir");
    let port = pick_free_port();
    let output = fail_closed_output(home.path(), port, &[("ARGO_GATEWAY_PROFILE", "production")]);
    assert!(!output.status.success(), "gateway must fail closed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ARGO_GATEWAY_BEARER_TOKEN"),
        "stderr must name missing token env var: {stderr}"
    );
    assert!(
        stderr.contains("docs/skycot/operator/security.md"),
        "stderr must point at operator docs: {stderr}"
    );

    let by_bind = fail_closed_output(home.path(), port, &[("ARGO_GATEWAY_BIND", "0.0.0.0:7400")]);
    assert!(
        !by_bind.status.success(),
        "non-loopback ARGO_GATEWAY_BIND must also fail closed"
    );
}

#[test]
fn wizard_requires_bootstrap_token_and_returns_gone_after_done() {
    let _gateway_lock = gateway_lock_guard();
    let home = tempfile::tempdir().expect("tempdir");
    let port = pick_free_port();
    let (mut child, _drain) = spawn_gateway(home.path(), port, 3);

    let missing = http(port, "/api/v1/wizard", &[]);
    assert!(
        status_line(&missing).contains("401"),
        "missing bootstrap token should 401: {missing}"
    );

    let token = std::fs::read_to_string(home.path().join(".bootstrap-token"))
        .expect("bootstrap token")
        .trim()
        .to_string();
    let ok = http(port, "/api/v1/wizard", &[("X-Argo-Bootstrap", &token)]);
    assert!(
        status_line(&ok).contains("200"),
        "matching bootstrap token should reach wizard: {ok}"
    );

    std::fs::write(home.path().join(".wizard-done"), b"done\n").expect("write wizard done");
    let gone = http(port, "/api/v1/wizard", &[("X-Argo-Bootstrap", &token)]);
    assert!(
        status_line(&gone).contains("410"),
        "wizard-done sentinel should return 410: {gone}"
    );

    let status = wait_with_timeout(&mut child);
    assert!(status.success(), "gateway exited nonzero: {status:?}");
}

#[tokio::test]
async fn bootstrap_rotate_regenerates_token_and_audits() {
    let home = tempfile::tempdir().expect("tempdir");
    let output = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .current_dir(home.path())
        .args(["gateway", "bootstrap", "rotate"])
        .output()
        .expect("rotate token");
    assert!(
        output.status.success(),
        "rotate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let token = std::fs::read_to_string(home.path().join(".bootstrap-token"))
        .expect("token file")
        .trim()
        .to_string();
    assert_eq!(token.len(), 64, "token should be 32 random bytes as hex");

    let hex = argo_store::encryption::base64_to_hex(SMOKE_DB_KEY_B64).expect("hex key");
    let store = argo_store::SessionStore::open_with_hex_key(
        home.path().join("sessions.db"),
        argo_store::wal_tuning::WalTuningProfile::default(),
        &hex,
    )
    .await
    .expect("open encrypted store");
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'http.request' \
         ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("audit payload");
    assert!(
        payload.contains("gateway.bootstrap_token.rotate"),
        "rotation audit row missing: {payload}"
    );
}
