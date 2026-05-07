//! Shared helpers for gateway integration tests. Cargo's
//! convention: anything under `tests/common/mod.rs` is
//! `mod common;`-imported by each test binary, but doesn't
//! count as a separate test target.

#![allow(dead_code)]

pub mod product_e2e_helpers;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

pub fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

static NEXT_PORT: AtomicU16 = AtomicU16::new(0);

pub fn pick_free_port() -> u16 {
    loop {
        let candidate = next_candidate_port();
        if reserve_port(candidate) && TcpListener::bind(("127.0.0.1", candidate)).is_ok() {
            return candidate;
        }
    }
}

fn next_candidate_port() -> u16 {
    let current = NEXT_PORT.load(Ordering::SeqCst);
    if current != 0 {
        return NEXT_PORT.fetch_add(1, Ordering::SeqCst);
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    NEXT_PORT
        .compare_exchange(
            0,
            port.saturating_add(1),
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .ok();
    port
}

fn reserve_port(port: u16) -> bool {
    let path = std::env::temp_dir().join(format!("argo-gateway-smoke-port-{port}.lock"));
    clear_stale_port(&path);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut f| writeln!(f, "{}", std::process::id()))
        .is_ok()
}

fn clear_stale_port(path: &PathBuf) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    let Ok(modified) = meta.modified() else {
        return;
    };
    if modified
        .elapsed()
        .is_ok_and(|age| age > Duration::from_secs(600))
    {
        let _ = std::fs::remove_file(path);
    }
}

pub struct GatewayChild {
    child: Child,
    _lock: GatewayLock,
}

impl Deref for GatewayChild {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl DerefMut for GatewayChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

pub struct GatewayLock {
    path: PathBuf,
}

impl Drop for GatewayLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.path);
    }
}

pub fn gateway_lock_guard() -> GatewayLock {
    gateway_lock()
}

fn gateway_lock() -> GatewayLock {
    let path = std::env::temp_dir().join("argo-gateway-smoke.lock");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match std::fs::create_dir(&path) {
            Ok(()) => return GatewayLock { path },
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                clear_stale_gateway_lock(&path);
                assert!(
                    Instant::now() <= deadline,
                    "timed out waiting for {}",
                    path.display()
                );
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => panic!("create {}: {err}", path.display()),
        }
    }
}

fn clear_stale_gateway_lock(path: &Path) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    let Ok(modified) = meta.modified() else {
        return;
    };
    if modified
        .elapsed()
        .is_ok_and(|age| age > Duration::from_secs(120))
    {
        let _ = std::fs::remove_dir_all(path);
    }
}

pub fn http_get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

pub fn http_post(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

pub fn body_of(resp: &str) -> &str {
    resp.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

pub fn await_listening(stderr: &mut BufReader<impl Read>) {
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            panic!("listener did not announce in 2s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }
}

/// Start the gateway and return both the child handle + the
/// stderr BufReader. The reader must stay alive for the whole
/// test — dropping it before the child exits closes the pipe
/// and breaks the gateway's stderr writes (manifest as
/// `ConnectionReset` on the next HTTP request).
pub fn boot(port: u16, max_requests: u32) -> (GatewayChild, BufReader<ChildStderr>) {
    let lock = gateway_lock();
    let home = tempfile::tempdir().unwrap().keep();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &(max_requests + 1).to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);
    let resp = http_get(port, "/sessions");
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "gateway store not ready: {resp}"
    );
    (GatewayChild { child, _lock: lock }, stderr)
}
