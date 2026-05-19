//! Seed + boot helper for `codex-multi-tenant-isolation-bench`.
//!
//! `tests::common::boot()` hides the gateway's `ARGO_HOME` tempdir;
//! this fixture exposes the path so the test can open a second
//! `SessionStore` against the same SQLite file (WAL-safe) for direct
//! row seeding and post-load inspection. Seeding bypasses HTTP — the
//! gateway hire/onboard flow is multi-step and out of scope for an
//! isolation correctness test.

#![allow(dead_code)]

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_memory::{MemoryFile, MemoryStore};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId, WorkspaceId};

pub struct TenantHandle {
    pub tenant_id: TenantId,
    pub workspace_id: WorkspaceId,
    pub tenant_audit_str: String,
}

pub struct GatewayHandle {
    pub port: u16,
    pub home: PathBuf,
    child: Child,
    _tempdir: tempfile::TempDir,
    _drain: thread::JoinHandle<()>,
}

impl Drop for GatewayHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn boot_gateway_with_home() -> GatewayHandle {
    let port = pick_free_port();
    let tempdir = tempfile::tempdir().expect("tempdir");
    let home = tempdir.path().to_path_buf();
    let mut child = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", &home)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "100000",
            // No provider keys in the test env — isolation invariants
            // don't exercise providers; bypass the health-check gate.
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo gateway");
    // Drain stderr in a background thread + signal on the "listening
    // on" line. Mirrors the pattern in tests::common::listener_signal
    // (kept inline so the fixture stays self-contained per the
    // brief's may-touch list). The drain thread keeps reading after
    // signaling so the gateway never blocks on a full pipe buffer.
    let stderr = child.stderr.take().expect("stderr piped");
    let (tx, rx) = mpsc::sync_channel::<bool>(1);
    let captured = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let captured_drain = Arc::clone(&captured);
    let drain = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        let mut announced = false;
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    if !announced {
                        let _ = tx.send(false);
                    }
                    break;
                }
                Ok(_) => {
                    if let Ok(mut buf) = captured_drain.lock() {
                        if buf.len() < 200 {
                            buf.push(line.trim_end().to_string());
                        }
                    }
                    if !announced && line.contains("listening on") {
                        announced = true;
                        let _ = tx.send(true);
                    }
                }
                Err(_) => {
                    if !announced {
                        let _ = tx.send(false);
                    }
                    break;
                }
            }
        }
    });
    let deadline_ms = std::env::var("ARGO_TEST_LISTENER_DEADLINE_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30_000);
    match rx.recv_timeout(Duration::from_millis(deadline_ms)) {
        Ok(true) => GatewayHandle {
            port,
            home,
            child,
            _tempdir: tempdir,
            _drain: drain,
        },
        Ok(false) => {
            let _ = child.kill();
            let _ = child.wait();
            let log = captured.lock().map(|b| b.join("\n")).unwrap_or_default();
            panic!("gateway crashed before listener bound on port {port}\nstderr:\n{log}");
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            let log = captured.lock().map(|b| b.join("\n")).unwrap_or_default();
            panic!(
                "gateway did not announce 'listening on' within {deadline_ms}ms on port {port}\nstderr:\n{log}"
            );
        }
    }
}

fn pick_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    port
}

pub async fn connect_store(home: &std::path::Path) -> Arc<SessionStore> {
    Arc::new(
        SessionStore::open(&home.join("sessions.db"))
            .await
            .expect("open sessions.db"),
    )
}

pub async fn provision_tenant(
    store: &Arc<SessionStore>,
    name_seed: &str,
) -> ArgoResult<TenantHandle> {
    let (tenant_id, workspace_id) = store
        .ensure_workspace(name_seed, &format!("/skycot-isolation-bench/{name_seed}"))
        .await?;
    Ok(TenantHandle {
        tenant_id,
        workspace_id,
        tenant_audit_str: tenant_id.0.to_string(),
    })
}

#[derive(Clone, Copy)]
pub struct SeedCounts {
    pub audit: usize,
    pub memory: usize,
    pub workers: usize,
    pub skills: usize,
}

pub async fn seed_tenant_rows(
    store: &Arc<SessionStore>,
    handle: &TenantHandle,
    name_seed: &str,
    counts: SeedCounts,
) -> ArgoResult<argo_types::SessionId> {
    let session_id = store
        .create_session(
            handle.tenant_id,
            handle.workspace_id,
            "seed-model",
            "seed-provider",
            Some(name_seed),
            None,
        )
        .await?;
    let sink = AuditSink::new_with_tenant(Arc::clone(store), handle.tenant_audit_str.clone());
    for i in 0..counts.audit {
        sink.write(
            Some(session_id),
            AuditKind::TurnStart,
            serde_json::json!({"seed": name_seed, "i": i}),
            None,
        )
        .await?;
    }
    sink.flush_pending().await?;
    let memory = MemoryStore::new(Arc::clone(store));
    for i in 0..counts.memory {
        memory
            .append(
                handle.workspace_id,
                MemoryFile::Memory,
                format!("seed:{name_seed}:row:{i}"),
                None,
            )
            .await?;
    }
    let identities = IdentityStore::new(store);
    for i in 0..counts.workers {
        let worker = Worker::draft(
            handle.tenant_id,
            "isolation-bench",
            Persona::named(format!("Seed-{name_seed}-{i}"), "test@bench"),
            Role::new(format!("Seed role {i}")),
        );
        identities.create_worker(&worker).await?;
    }
    let skills = SkillStore::new(Arc::clone(store));
    for i in 0..counts.skills {
        let raw = format!(
            "---\nname: bench-{name_seed}-{i}\nversion: \"0.1.0\"\ndescription: \"seed {i}\"\nprovenance:\n  source: builtin\n  hash: sha256-bench-{name_seed}-{i}\n---\nbody {i}\n"
        );
        skills.install(&raw, Some(session_id)).await?;
    }
    Ok(session_id)
}
