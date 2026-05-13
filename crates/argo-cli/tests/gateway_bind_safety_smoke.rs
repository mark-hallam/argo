//! Integration smoke for the load-bearing bind-safety guard
//! (`mission-gateway-tls-termination-or-proxy-doc`). Two scenarios:
//! (1) `--bind 0.0.0.0` without `[gateway.behind_proxy]` config MUST
//! exit non-zero with a runbook-pointing stderr; (2) with the
//! contract signed in `argo.toml`, the gateway MUST boot. Together
//! these prove today's plaintext-to-the-world failure mode is gone.

mod common;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

fn spawn_gateway(home: &std::path::Path, bind: &str, port: u16) -> Child {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(home)
        .args([
            "gateway", "start", "--bind", bind, "--port", &port.to_string(),
            "--max-requests", "1", "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn")
}

fn wait_with_timeout(child: &mut Child, dur: Duration) -> ExitStatus {
    let deadline = Instant::now() + dur;
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

#[test]
fn bind_non_loopback_without_contract_exits_nonzero() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("tempdir");
    // No argo.toml ⇒ default config (expected_proxy=false).
    let mut child = spawn_gateway(home.path(), "0.0.0.0", port);
    let status = wait_with_timeout(&mut child, Duration::from_secs(15));
    assert!(!status.success(), "must exit non-zero, got {status:?}");
    let mut stderr_buf = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut stderr_buf);
    }
    assert!(
        stderr_buf.contains("gateway-tls-deployment.md"),
        "stderr must reference runbook, got: {stderr_buf}"
    );
    assert!(
        stderr_buf.contains("expected_proxy=true") || stderr_buf.contains("expected_proxy = true"),
        "stderr must mention config opt-in, got: {stderr_buf}"
    );
}

#[test]
fn bind_non_loopback_with_contract_boots_cleanly() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        home.path().join("argo.toml"),
        "[gateway.behind_proxy]\nexpected_proxy = true\ntrusted_proxies = []\n",
    )
    .expect("write argo.toml");
    let mut child = spawn_gateway(home.path(), "127.0.0.1", port);
    let mut handled = false;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            let _ =
                stream.write_all(b"GET /livez HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
            let mut buf = String::new();
            let _ = stream.read_to_string(&mut buf);
            handled = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let status = wait_with_timeout(&mut child, Duration::from_secs(15));
    assert!(handled, "gateway never accepted a connection — boot failed");
    assert!(status.success(), "gateway exited non-zero: {status:?}");
}
