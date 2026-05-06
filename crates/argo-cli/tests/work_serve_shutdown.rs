//! Integration test for `argo work serve` graceful shutdown
//! (#770). Verifies that SIGTERM (Unix only) and SIGINT
//! (Ctrl+C, all platforms) cause the long-running worker to
//! exit cleanly within a reasonable window — the
//! "I-can-`systemctl-stop`-this" contract for multi-day
//! operation.

mod common;

use common::argo_bin;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn fresh_workspace() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = Command::new(argo_bin())
        .arg("workspace")
        .arg("init")
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work.path())
        .output()
        .expect("workspace init");
    assert!(
        init.status.success(),
        "workspace init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    (home, work)
}

#[cfg(unix)]
#[test]
fn serve_exits_cleanly_on_sigterm() {
    use std::process::Stdio;

    let (home, work) = fresh_workspace();

    // Boot serve in long-poll mode (no --once). Empty queue +
    // 30s interval means the worker will sit in the sleep
    // tokio::select waiting for either the timer or a signal.
    let mut child = Command::new(argo_bin())
        .args([
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--max-steps",
            "1",
            "--interval-seconds",
            "30",
        ])
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work.path())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo work serve");

    // Give it ~500ms to install the watcher + enter the sleep.
    std::thread::sleep(Duration::from_millis(500));

    // Send SIGTERM via libc::kill. `signal-hook`, `nix`, or
    // similar would be cleaner but introducing a test-only dep
    // for one syscall feels heavy.
    let pid = child.id() as i32;
    // SAFETY: kill(2) is signal-safe; we hold the Child so the
    // pid is valid until we wait() below.
    let rc = unsafe { libc::kill(pid, libc::SIGTERM) };
    assert_eq!(rc, 0, "kill(SIGTERM) returned {rc} (errno?)");

    // The worker should exit within a few seconds (200ms poll
    // interval + boot/teardown time). Anything longer means
    // the shutdown path is not wired up.
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    panic!(
                        "argo work serve did not exit within 5s of SIGTERM \
                         — graceful shutdown not wired up"
                    );
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("try_wait error: {e}"),
        }
    };

    assert!(
        status.success(),
        "expected clean exit (code 0) on SIGTERM; got {status:?}"
    );

    // Verify the shutdown log line landed on stderr — operator
    // visibility is part of the contract.
    let stderr = child
        .stderr
        .take()
        .map(|mut s| {
            use std::io::Read;
            let mut out = String::new();
            let _ = s.read_to_string(&mut out);
            out
        })
        .unwrap_or_default();
    assert!(
        stderr.contains("graceful shutdown"),
        "stderr should announce graceful shutdown; got:\n{stderr}"
    );
}

/// Same contract for SIGINT (Ctrl+C). Cross-platform via
/// `tokio::signal::ctrl_c`. On Unix this is the same path as
/// the SIGTERM test above; on Windows this is the only path.
#[cfg(unix)]
#[test]
fn serve_exits_cleanly_on_sigint() {
    use std::process::Stdio;

    let (home, work) = fresh_workspace();

    let mut child = Command::new(argo_bin())
        .args([
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--max-steps",
            "1",
            "--interval-seconds",
            "30",
        ])
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work.path())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo work serve");

    std::thread::sleep(Duration::from_millis(500));

    let pid = child.id() as i32;
    // SAFETY: see SIGTERM test above.
    let rc = unsafe { libc::kill(pid, libc::SIGINT) };
    assert_eq!(rc, 0, "kill(SIGINT) returned {rc}");

    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    panic!("argo work serve did not exit within 5s of SIGINT");
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("try_wait error: {e}"),
        }
    };
    assert!(
        status.success(),
        "expected clean exit (code 0) on SIGINT; got {status:?}"
    );
}
