//! End-to-end tests for `argo gateway start` graceful-shutdown:
//! 1. SIGTERM after a clean request → process exits 0 with the
//!    "drain complete" + "session store closed" log lines.
//! 2. A connection that sends its request bytes after SIGTERM has
//!    flipped the controller flag receives the documented 503 body
//!    `{"error":"shutting_down"}`.
//! 3. With `ARGO_SHUTDOWN_DEADLINE_SECS=1`, a stalled-read client
//!    that prevents the accept loop from breaking still gets the
//!    process out via the force-exit watchdog (status 124).
//!
//! These tests spawn the real `argo` binary so they exercise the
//! actual signal-handler wiring, not just the unit-test fakes inside
//! `shutdown.rs`. Each test holds the shared `gateway_lock_guard()`
//! so the gateway PID file at `~/.argo/gateway.pid` doesn't collide
//! across parallel test threads.

#![cfg(unix)]

mod common;

use common::{argo_bin, gateway_lock_guard, pick_free_port};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ChildStderr, Command, Stdio};
use std::time::{Duration, Instant};

/// SIGTERM, defined by POSIX as 15.
const SIGTERM: i32 = 15;

fn spawn_gateway(port: u16, home: &std::path::Path, extra_envs: &[(&str, &str)]) -> Child {
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home).args([
        "gateway",
        "start",
        "--port",
        &port.to_string(),
        // No --max-requests cap: we want the process to live until
        // we send SIGTERM ourselves.
    ]);
    for (k, v) in extra_envs {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway")
}

fn await_listening(stderr: &mut BufReader<ChildStderr>) {
    let mut line = String::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if Instant::now() > deadline {
            panic!("listener did not announce in 5s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            return;
        }
    }
}

/// Drain stderr into a String on a background thread until the pipe
/// closes (i.e. the child exits). Returns the captured text. Without
/// this, large drain logs can fill the pipe buffer and deadlock the
/// child while it's trying to write the "session store closed" line.
fn drain_stderr_in_background(reader: BufReader<ChildStderr>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = String::new();
        let mut reader = reader;
        let _ = reader.read_to_string(&mut buf);
        buf
    })
}

fn send_signal(pid: u32, signal: i32) {
    // Cast through i32 — `nix::sys::signal::kill` would be cleaner but
    // pulling in the dep just for tests is overkill.
    let rc = unsafe { libc::kill(pid as i32, signal) };
    assert_eq!(rc, 0, "kill({pid}, {signal}) returned {rc}");
}

fn http_get_blocking(port: u16, path: &str, timeout: Duration) -> String {
    let stream =
        TcpStream::connect(("127.0.0.1", port)).expect("connect to gateway");
    stream
        .set_read_timeout(Some(timeout))
        .expect("set read timeout");
    stream
        .set_write_timeout(Some(timeout))
        .expect("set write timeout");
    let mut stream = stream;
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn sigterm_drains_inflight_request_then_exits_zero() {
    let _lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = spawn_gateway(port, home.path(), &[]);
    let mut stderr = BufReader::new(child.stderr.take().expect("stderr"));
    await_listening(&mut stderr);

    // 1) Drive a successful in-flight request through the new
    //    routes::dispatch_route guard path. This proves the per-
    //    request drop-guard increments + decrements cleanly.
    let resp = http_get_blocking(port, "/health", Duration::from_secs(5));
    assert!(resp.contains("HTTP/1.1 200 OK"), "expected /health 200: {resp}");

    // 2) SIGTERM and confirm clean exit.
    let stderr_drain = drain_stderr_in_background(stderr);
    send_signal(child.id(), SIGTERM);
    let status = child.wait().expect("wait");
    let captured = stderr_drain.join().expect("stderr drainer");

    assert!(
        status.success(),
        "expected exit 0, got {status:?}; stderr=\n{captured}"
    );
    assert!(
        captured.contains("shutdown signal received"),
        "missing signal log; stderr=\n{captured}"
    );
    assert!(
        captured.contains("drain complete"),
        "missing drain-complete log; stderr=\n{captured}"
    );
    assert!(
        captured.contains("session store closed"),
        "missing store-closed log; stderr=\n{captured}"
    );
}

#[test]
fn post_sigterm_dispatch_returns_503_shutting_down_body() {
    let _lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    // Generous deadline so the gateway stays alive long enough for
    // the slow-write client to reach dispatch_route AFTER the flag
    // has flipped.
    let mut child = spawn_gateway(
        port,
        home.path(),
        &[("ARGO_SHUTDOWN_DEADLINE_SECS", "30")],
    );
    let mut stderr = BufReader::new(child.stderr.take().expect("stderr"));
    await_listening(&mut stderr);

    // Open the TCP connection but don't send the HTTP request bytes
    // yet. Sleep so the gateway has time to accept(), enter the
    // accept-arm body, and park in `stream.read()` — that's the
    // window we need to land SIGTERM in.
    let mut conn = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    conn.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    conn.set_write_timeout(Some(Duration::from_secs(10))).unwrap();
    std::thread::sleep(Duration::from_millis(200));

    // Fire SIGTERM. The signal task triggers the controller and
    // arms the watchdog — but the accept arm body is still parked
    // in `stream.read()` waiting for our request bytes.
    send_signal(child.id(), SIGTERM);
    // Give the signal handler a moment to set the flag before we
    // unblock the read.
    std::thread::sleep(Duration::from_millis(200));

    // Now send the request. The server's read returns, dispatch_route
    // is called, sees the flag, and returns the documented 503 body.
    let req = "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    conn.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    conn.read_to_string(&mut resp).expect("read");

    assert!(
        resp.contains("HTTP/1.1 503"),
        "expected 503, got: {resp}"
    );
    assert!(
        resp.contains("shutting_down"),
        "expected `shutting_down` body, got: {resp}"
    );

    // Process should still exit cleanly afterwards.
    let stderr_drain = drain_stderr_in_background(stderr);
    let status = child.wait().expect("wait");
    let captured = stderr_drain.join().expect("stderr drainer");
    assert!(
        status.success(),
        "expected exit 0, got {status:?}; stderr=\n{captured}"
    );
}

#[test]
fn force_exit_fires_when_drain_hangs_past_deadline() {
    let _lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    // Tight deadline so the test runs in <10s: drain bound = 1s,
    // grace = 5s (hard-coded in shutdown.rs::FORCE_EXIT_GRACE_SECS),
    // total budget = ~6s before force-exit fires.
    let mut child = spawn_gateway(
        port,
        home.path(),
        &[("ARGO_SHUTDOWN_DEADLINE_SECS", "1")],
    );
    let mut stderr = BufReader::new(child.stderr.take().expect("stderr"));
    await_listening(&mut stderr);

    // Stalled client: connect but never send the request bytes. The
    // accept arm parks in `read()` waiting for us. Without the
    // force-exit watchdog the gateway would hang forever.
    let _stalled = TcpStream::connect(("127.0.0.1", port)).expect("connect stalled client");
    // Sleep so the gateway picks up the connection and enters
    // `stream.read()`. Otherwise SIGTERM wins the select! before the
    // accept arm fires, the loop breaks cleanly, and we'd see exit 0
    // instead of the watchdog's 124.
    std::thread::sleep(Duration::from_millis(200));

    send_signal(child.id(), SIGTERM);
    let stderr_drain = drain_stderr_in_background(stderr);

    // Watchdog fires at deadline + 5s = 6s; give a generous +4s
    // tolerance for CI scheduler jitter.
    let started = Instant::now();
    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => panic!("wait: {e}"),
    };
    let elapsed = started.elapsed();
    let captured = stderr_drain.join().expect("stderr drainer");

    assert!(
        elapsed < Duration::from_secs(15),
        "force-exit did not fire within 15s (elapsed={elapsed:?}); stderr=\n{captured}"
    );
    // POSIX: status.code() returns Some(124) when std::process::exit(124) ran.
    // If the runtime trapped the SIGTERM via the default handler instead, we'd
    // see signal()=Some(15) — but our install_signal_handler intercepts first.
    assert_eq!(
        status.code(),
        Some(124),
        "expected exit 124, got status={status:?} (signal={:?}); stderr=\n{captured}",
        status.signal()
    );
    assert!(
        captured.contains("force-exit"),
        "missing force-exit log; stderr=\n{captured}"
    );
}
