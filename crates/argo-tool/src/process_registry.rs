//! `ProcessRegistry` — long-lived background OS processes.
//!
//! When `terminal { background: true }` is dispatched, the child is
//! spawned through this registry instead of being awaited inline. The
//! call returns a stable `process_id` immediately, and the new
//! [`crate::tools::Process`] tool exposes `list | status | tail | kill`
//! against the registry handle held on [`crate::ToolContext`].
//!
//! Mirrors Hermes' `process_registry.py`: ring-buffered stdout/stderr
//! tails (~64 KB each), reap-on-poll status, and a SIGTERM →
//! SIGKILL-after-grace kill path on Unix.
//!
//! **Tenant scope.** Every public method takes a `TenantId`; entries
//! are filtered to the caller's tenant. Cross-tenant `status`/`tail`
//! returns `None`; cross-tenant `kill` returns the same
//! `no such process` error as a genuinely-unknown id (no existence
//! leak). This is the structural defense the F2 audit finding asked
//! for — once `mission-agent-runner-wire-process-registry-per-session`
//! lands and a single registry is shared across tenants, the entries
//! still can't be seen or touched across tenant boundaries.
//!
//! ```no_run
//! # use std::sync::Arc;
//! # use argo_tool::process_registry::{ProcessRegistry, SpawnRequest};
//! # use argo_types::TenantId;
//! # async fn run() -> argo_types::ArgoResult<()> {
//! let registry = Arc::new(ProcessRegistry::new());
//! let tenant = TenantId::new();
//! let pid = registry
//!     .spawn(
//!         SpawnRequest {
//!             command: "tail -f /var/log/system.log".into(),
//!             working_dir: std::path::PathBuf::from("/tmp"),
//!         },
//!         tenant,
//!     )
//!     .await?;
//! let live = registry.list(tenant).await;
//! assert_eq!(live.len(), 1);
//! assert_eq!(live[0].id, pid);
//! registry
//!     .kill(&pid, tenant, std::time::Duration::from_secs(2))
//!     .await?;
//! # Ok(()) }
//! ```
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

#[path = "process_registry_tail.rs"]
mod tail;
pub use tail::TailBuffer;
use tail::{send_sigterm, spawn_pump};

/// Cap per stream — generous enough for normal tail output, hard
/// enough to stop a `yes` loop from ballooning the registry.
const TAIL_CAPACITY: usize = 64 * 1024;

/// Soft cap on the bytes returned by [`ProcessRegistry::tail`] (sum
/// of stdout + stderr). Mirrors the Hermes default.
const TAIL_RETURN_BYTES: usize = 32 * 1024;

/// Stable identifier for a registered background process.
///
/// We re-use UUIDv7 (already enabled workspace-wide) so listings are
/// time-ordered. The model only sees the string form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub String);

impl ProcessId {
    /// Generate a fresh id.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7().to_string())
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Request shape accepted by [`ProcessRegistry::spawn`].
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    /// Shell command-line, run via `bash -c` on unix / `cmd /C` on Windows
    /// (see [`shell_command`]). Command syntax is the caller's responsibility
    /// per platform.
    pub command: String,
    /// Working directory the child inherits.
    pub working_dir: PathBuf,
}

/// Lightweight summary returned by [`ProcessRegistry::list`] and
/// [`ProcessRegistry::status`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessInfo {
    /// Registry id.
    pub id: ProcessId,
    /// OS pid (if still alive at last poll).
    pub pid: Option<u32>,
    /// Original command line.
    pub command: String,
    /// `running`, `exited:<code>`, or `killed`.
    pub state: String,
    /// Wall-clock seconds since spawn.
    pub age_seconds: u64,
}

/// Owned by the registry; not exposed to callers.
struct ProcessHandle {
    info: ProcessInfo,
    started_at: Instant,
    child: Mutex<Option<tokio::process::Child>>,
    stdout: Arc<Mutex<TailBuffer>>,
    stderr: Arc<Mutex<TailBuffer>>,
    /// Tenant that spawned the process. Reads/writes from any other
    /// tenant are filtered out (see module docstring).
    tenant_id: TenantId,
}

/// Build the platform shell invocation for a background command line.
///
/// Unix runs it through `bash -c <cmd>` (unchanged). Windows runs it through
/// `cmd /C <cmd>`: `bash` is not guaranteed on a native-Windows host (it ships
/// only with Git/WSL), while `cmd.exe` always is, so hardcoding `bash` made the
/// spawn primitive itself unusable there. The command-line *syntax* remains the
/// caller's responsibility per platform — a bash-syntax command will not run
/// under `cmd` — this only selects a shell binary that exists.
///
/// `pub(crate)` so the streaming terminal bypass
/// ([`crate::tools::terminal_streaming::run_streaming`]) shares one shell-selection
/// seam instead of hardcoding `bash` a second time (it previously did, breaking
/// streaming `terminal` on native Windows; see `win-terminal-streaming-bash-portability`).
pub(crate) fn shell_command(command: &str) -> tokio::process::Command {
    #[cfg(windows)]
    {
        let mut c = tokio::process::Command::new("cmd");
        c.arg("/C").arg(command);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = tokio::process::Command::new("bash");
        c.arg("-c").arg(command);
        c
    }
}

/// Concurrent registry of background processes.
#[derive(Debug, Default)]
pub struct ProcessRegistry {
    inner: RwLock<HashMap<ProcessId, Arc<ProcessHandle>>>,
}

impl std::fmt::Debug for ProcessHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessHandle")
            .field("info", &self.info)
            .field("tenant_id", &self.tenant_id)
            .finish()
    }
}

impl ProcessRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new background process. The child runs through the platform
    /// shell ([`shell_command`]: `bash -c` on unix, `cmd /C` on Windows),
    /// mirroring [`crate::tools::Terminal`]. The entry is tagged
    /// with `tenant_id`; only callers with the same tenant can
    /// `status`/`tail`/`kill`/`list` it. Returns the new process id.
    pub async fn spawn(&self, req: SpawnRequest, tenant_id: TenantId) -> ArgoResult<ProcessId> {
        let mut child = shell_command(&req.command)
            .current_dir(&req.working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ArgoError::Tool {
                tool: "process_registry".into(),
                msg: format!("spawn failed: {e}"),
            })?;
        let pid = child.id();
        let stdout = Arc::new(Mutex::new(TailBuffer::new(TAIL_CAPACITY)));
        let stderr = Arc::new(Mutex::new(TailBuffer::new(TAIL_CAPACITY)));
        if let Some(out) = child.stdout.take() {
            spawn_pump(out, stdout.clone());
        }
        if let Some(err) = child.stderr.take() {
            spawn_pump(err, stderr.clone());
        }
        let id = ProcessId::new();
        let handle = Arc::new(ProcessHandle {
            info: ProcessInfo {
                id: id.clone(),
                pid,
                command: req.command,
                state: "running".into(),
                age_seconds: 0,
            },
            started_at: Instant::now(),
            child: Mutex::new(Some(child)),
            stdout,
            stderr,
            tenant_id,
        });
        self.inner.write().await.insert(id.clone(), handle);
        Ok(id)
    }

    /// Snapshot every live process **owned by `tenant_id`**. Reaps any
    /// that exited since the last call so callers see fresh `state`
    /// strings. Entries owned by other tenants are invisible.
    pub async fn list(&self, tenant_id: TenantId) -> Vec<ProcessInfo> {
        let ids: Vec<ProcessId> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|(_, h)| h.tenant_id == tenant_id)
            .map(|(id, _)| id.clone())
            .collect();
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(info) = self.status(&id, tenant_id).await {
                out.push(info);
            }
        }
        out.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        out
    }

    /// Status of a single process; `None` if unknown OR owned by a
    /// different tenant (no existence leak across tenants).
    pub async fn status(&self, id: &ProcessId, tenant_id: TenantId) -> Option<ProcessInfo> {
        let handle = self.lookup_for_tenant(id, tenant_id).await?;
        let mut info = handle.info.clone();
        info.age_seconds = handle.started_at.elapsed().as_secs();
        let mut child_slot = handle.child.lock().await;
        if let Some(child) = child_slot.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    info.state = match status.code() {
                        Some(code) => format!("exited:{code}"),
                        None => "killed".into(),
                    };
                    info.pid = None;
                    *child_slot = None;
                }
                Ok(None) => {}
                Err(_) => {
                    info.state = "unknown".into();
                }
            }
        }
        Some(info)
    }

    /// Render the most recent ~32 KB of stdout + stderr for a single
    /// process. Returns `None` if unknown OR owned by a different
    /// tenant.
    pub async fn tail(&self, id: &ProcessId, tenant_id: TenantId) -> Option<TailOutput> {
        let handle = self.lookup_for_tenant(id, tenant_id).await?;
        let stdout = handle.stdout.lock().await.render(TAIL_RETURN_BYTES);
        let stderr = handle.stderr.lock().await.render(TAIL_RETURN_BYTES);
        Some(TailOutput { stdout, stderr })
    }

    /// Send SIGTERM, wait up to `grace` for the child to exit, then
    /// fall back to SIGKILL via tokio. Removes the process from the
    /// registry on success.
    ///
    /// Cross-tenant `kill` returns the same `no such process` error
    /// as a genuinely-unknown id — callers from another tenant can't
    /// distinguish "doesn't exist" from "exists but you can't touch
    /// it." This is by design to prevent existence-probing.
    pub async fn kill(
        &self,
        id: &ProcessId,
        tenant_id: TenantId,
        grace: Duration,
    ) -> ArgoResult<()> {
        let handle =
            self.lookup_for_tenant(id, tenant_id)
                .await
                .ok_or_else(|| ArgoError::Tool {
                    tool: "process_registry".into(),
                    msg: format!("no such process: {id}"),
                })?;
        let mut child_slot = handle.child.lock().await;
        let Some(mut child) = child_slot.take() else {
            self.remove_if_tenant_matches(id, tenant_id).await;
            return Ok(());
        };
        if let Some(pid) = child.id() {
            send_sigterm(pid);
        }
        let waited = tokio::time::timeout(grace, child.wait()).await;
        if waited.is_err() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        drop(child_slot);
        self.remove_if_tenant_matches(id, tenant_id).await;
        Ok(())
    }

    /// Reap any processes belonging to `tenant_id` that have already
    /// exited. Other tenants' entries are untouched. Called by the
    /// `process` tool's `list` action so completed work doesn't sit
    /// in the registry forever.
    pub async fn reap_finished(&self, tenant_id: TenantId) {
        let ids: Vec<ProcessId> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|(_, h)| h.tenant_id == tenant_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            let exited = {
                let map = self.inner.read().await;
                let Some(handle) = map.get(&id).cloned() else {
                    continue;
                };
                if handle.tenant_id != tenant_id {
                    continue;
                }
                let mut slot = handle.child.lock().await;
                if let Some(child) = slot.as_mut() {
                    matches!(child.try_wait(), Ok(Some(_)))
                } else {
                    true
                }
            };
            if exited {
                self.remove_if_tenant_matches(&id, tenant_id).await;
            }
        }
    }

    /// Number of registry entries owned by `tenant_id` (live +
    /// not-yet-reaped). Entries from other tenants are not counted.
    pub async fn len(&self, tenant_id: TenantId) -> usize {
        self.inner
            .read()
            .await
            .values()
            .filter(|h| h.tenant_id == tenant_id)
            .count()
    }

    /// True if `tenant_id` has no entries in this registry. Other
    /// tenants' entries don't affect the answer.
    pub async fn is_empty(&self, tenant_id: TenantId) -> bool {
        self.len(tenant_id).await == 0
    }

    /// Lookup a handle by id, returning `None` if missing OR owned by
    /// a different tenant. This is the single fence used by every
    /// tenant-scoped accessor.
    async fn lookup_for_tenant(
        &self,
        id: &ProcessId,
        tenant_id: TenantId,
    ) -> Option<Arc<ProcessHandle>> {
        let handle = self.inner.read().await.get(id).cloned()?;
        if handle.tenant_id == tenant_id {
            Some(handle)
        } else {
            None
        }
    }

    /// Remove an entry only if it still belongs to `tenant_id`. Used
    /// after `kill`/`reap_finished` so a race with another tenant's
    /// spawn into the same slot can't drop their entry. ProcessId is
    /// a fresh UUIDv7 per spawn so collisions are astronomically
    /// unlikely, but the guard is cheap.
    async fn remove_if_tenant_matches(&self, id: &ProcessId, tenant_id: TenantId) {
        let mut map = self.inner.write().await;
        if let Some(handle) = map.get(id) {
            if handle.tenant_id == tenant_id {
                map.remove(id);
            }
        }
    }
}

/// Output of [`ProcessRegistry::tail`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TailOutput {
    /// Most-recent stdout window (lossy UTF-8).
    pub stdout: String,
    /// Most-recent stderr window (lossy UTF-8).
    pub stderr: String,
}

#[cfg(test)]
#[path = "process_registry_tests.rs"]
mod tests;
