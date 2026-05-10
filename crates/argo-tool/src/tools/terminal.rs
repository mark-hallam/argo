//! `terminal` — run a shell command in the workspace via `argo-sandbox`.
//!
//! Argo runs commands through `bash -c` for portability; full sandbox
//! isolation is §15 / Phase 5. Phase 3 enforces:
//!
//! - `Capability::Spawn { kind: Subprocess }` (or `Container` for the
//!   sandbox backend) — checked at dispatch.
//! - Per-call wall-clock timeout, defaulted to 60s.
//! - Output capped at 64 KB per stream so a runaway command can't OOM
//!   the agent loop.
//!
//! ## Backends
//!
//! `backend = "local"` (default) routes through
//! [`argo_sandbox::LocalSandbox`] — `bash -c` in the current OS
//! namespace. Suitable for trusted workspaces. Requires
//! `Capability::Spawn { kind: Subprocess }`.
//!
//! `backend = "sandbox"` opt-in container-grade isolation. Today the
//! dispatch still routes through `LocalSandbox` (the structural seam
//! is in place; the real Docker / E2B microVM backends land in a
//! follow-up brief). The dispatch logs a `terminal_sandbox_requested`
//! event so audit can confirm intent and operators see when a vertical
//! requests sandboxed exec while the isolation backend is still
//! pending. Requires `Capability::Spawn { kind: Container }`.
//!
//! ```
//! use argo_tool::{Terminal, Tool};
//! let t = Terminal::default();
//! assert_eq!(t.name(), "terminal");
//! // {"command": "echo hi"}                       — Spawn::Subprocess
//! // {"command": "npm install", "backend": "sandbox"} — Spawn::Container
//! ```
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_sandbox::{LocalSandbox, RunRequest, RunResult, Sandbox};
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, SpawnKind, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Local,
    Sandbox,
}

impl Backend {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            "sandbox" => Ok(Self::Sandbox),
            other => Err(format!(
                "unknown backend '{other}': supported = local | sandbox"
            )),
        }
    }

    fn from_args(args: &serde_json::Value) -> Result<Self, String> {
        match args.get("backend") {
            None | Some(serde_json::Value::Null) => Ok(Self::Local),
            Some(serde_json::Value::String(s)) => Self::parse(s),
            Some(other) => Err(format!(
                "invalid backend type: expected string, got {other}"
            )),
        }
    }

    fn capability(self) -> Capability {
        match self {
            Self::Local => Capability::Spawn {
                spawn_kind: SpawnKind::Subprocess,
            },
            Self::Sandbox => Capability::Spawn {
                spawn_kind: SpawnKind::Container,
            },
        }
    }
}

/// Run a shell command via `bash -c` through the
/// [`argo_sandbox::Sandbox`] seam.
///
/// Output is capped at 64 KB per stream and the wall-clock defaults
/// to 60s. The `backend` argument (`"local"` default | `"sandbox"`)
/// picks between the local-process and (today still local-backed)
/// container seam, with capability requirements differentiated
/// accordingly.
#[derive(Debug, Clone)]
pub struct Terminal {
    local: Arc<dyn Sandbox>,
    sandboxed: Arc<dyn Sandbox>,
}

impl Default for Terminal {
    fn default() -> Self {
        // Today both backends share `LocalSandbox`; the seam is in
        // place so a future Docker / E2B / Firecracker backend slots
        // into `sandboxed` without touching the tool surface.
        let local: Arc<dyn Sandbox> = Arc::new(LocalSandbox::new());
        Self {
            local: local.clone(),
            sandboxed: local,
        }
    }
}

impl Terminal {
    /// Inject custom sandbox instances. Used by tests (and by future
    /// runtime wiring once the Docker / E2B backend lands) to swap the
    /// `"sandbox"` backend without touching the tool surface.
    #[must_use]
    pub fn with_sandboxes(local: Arc<dyn Sandbox>, sandboxed: Arc<dyn Sandbox>) -> Self {
        Self { local, sandboxed }
    }

    fn pick(&self, backend: Backend) -> &Arc<dyn Sandbox> {
        match backend {
            Backend::Local => &self.local,
            Backend::Sandbox => &self.sandboxed,
        }
    }
}

// `backend` is read directly off the raw JSON in
// [`Backend::from_args`] (and again at capability time) so it does
// not appear in this struct, but the JSON schema still advertises it
// to the model.
#[derive(Debug, Deserialize)]
struct Args {
    command: String,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[async_trait]
impl Tool for Terminal {
    fn name(&self) -> &str {
        "terminal"
    }
    fn description(&self) -> &str {
        "Run a shell command in the workspace via `bash -c`. Returns \
         stdout, stderr, and exit code. Output capped at 64 KB per \
         stream; default timeout 60s. `backend` selects `local` \
         (default; Spawn::Subprocess) or `sandbox` (Spawn::Container — \
         routes through the argo-sandbox seam for isolation-grade exec)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": { "type": "string" },
                "working_dir": { "type": "string", "description": "Workspace-relative." },
                "timeout_ms": { "type": "integer", "minimum": 100 },
                "backend": {
                    "type": "string",
                    "enum": ["local", "sandbox"],
                    "default": "local",
                    "description": "`local` runs directly under bash (Spawn::Subprocess). `sandbox` routes through the argo-sandbox seam (Spawn::Container)."
                },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Serial
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let backend = Backend::from_args(args).map_err(|msg| ArgoError::Tool {
            tool: "terminal".into(),
            msg,
        })?;
        Ok(CapabilitySet(vec![backend.capability()]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let backend = match Backend::from_args(&args) {
            Ok(b) => b,
            Err(msg) => return Ok(err(ctx, started, msg)),
        };
        let parsed: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return Ok(err(ctx, started, format!("invalid arguments: {e}"))),
        };
        let cwd = parsed
            .working_dir
            .map(|d| ctx.workspace_root.join(d))
            .unwrap_or_else(|| ctx.workspace_root.clone());
        let timeout = Duration::from_millis(parsed.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));

        if backend == Backend::Sandbox {
            // Audit hook: explicit log so operators see when a vertical
            // requests sandboxed exec while the Docker / E2B backend is
            // still pending. The `resolved_backend_id` field reveals
            // whether the seam has been swapped (e.g. `"docker"`) or
            // is still falling back to local execution.
            tracing::info!(
                target: "argo.tool.terminal",
                event = "terminal_sandbox_requested",
                tool = "terminal",
                backend = "sandbox",
                resolved_backend_id = self.pick(backend).id(),
                command_preview = truncate(&parsed.command, 200).as_str(),
                "terminal sandbox-mode requested"
            );
        }

        let mut req = RunRequest::new(parsed.command.clone(), cwd);
        req.timeout = Some(timeout);
        match self.pick(backend).run(&req).await {
            Ok(result) => Ok(into_call_result(ctx, started, &parsed.command, timeout, &result)),
            Err(e) => Ok(err(ctx, started, format!("terminal exec failed: {e}"))),
        }
    }
}

fn err(ctx: &ToolContext, started: Instant, msg: String) -> ToolCallResult {
    error_text(ctx.call_id, msg, started.elapsed().as_millis() as u64)
}

fn into_call_result(
    ctx: &ToolContext,
    started: Instant,
    command: &str,
    timeout: Duration,
    result: &RunResult,
) -> ToolCallResult {
    if result.timed_out {
        return error_text(
            ctx.call_id,
            format!(
                "terminal exec exceeded {} ms timeout (`{}`)",
                timeout.as_millis(),
                truncate(command, 200)
            ),
            started.elapsed().as_millis() as u64,
        );
    }
    success_text(
        ctx.call_id,
        format_output(&result.stdout, &result.stderr, result.exit_code),
        started.elapsed().as_millis() as u64,
    )
}

fn format_output(stdout: &str, stderr: &str, code: i32) -> String {
    format!("exit_code: {code}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
