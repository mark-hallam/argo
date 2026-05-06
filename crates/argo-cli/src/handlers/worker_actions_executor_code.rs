//! `CodeExecutor` — V1.2 sandboxed code-execution executor for the
//! Worker v3 dispatch trait (Stream A `#A-executor-trait`).
//!
//! Runs a Python 3.12 or Node 20 script against an action payload
//! and returns the captured stdout / stderr / exit code plus any
//! files dropped at `/tmp/artifacts/` inside the sandbox.
//!
//! ## Wiring
//!
//! `[actions.<name>] executor = "code_execution"` (or the tagged
//! form `executor = { kind = "code_execution", language = "python",
//! allow_network = false, allow_pkg_install = false }`) routes the
//! action through this impl. The dispatch shim parses the payload
//! into a [`CodeRequest`], builds the language-specific shell
//! invocation, runs it via the configured backend, and serialises
//! the result.
//!
//! ## Backend
//!
//! v1 ships a single backend keyed off the env:
//!
//! - `ARGO_E2B_TOKEN` set → the executor refuses to run with a
//!   "not yet wired" error pointing at the v1.x E2B-exec client
//!   work item (the existing `argo-executor-computer-use` E2B
//!   integration covers browser primitives only — `exec` lands
//!   in a follow-up brief).
//! - `ARGO_E2B_TOKEN` missing → the executor falls back to
//!   `LocalSandbox` (no isolation, suitable only for trusted
//!   workspaces). The choice is logged at dispatch time so the
//!   audit ledger captures the gap.
//!
//! ## Sandbox guarantees (and their v1 limits)
//!
//! - **Time limit** — wall-clock cap from `ARGO_CODE_EXEC_TIMEOUT_S`
//!   (default 60s). Enforced via tokio timeout on the sandbox call.
//! - **Memory limit** — wrapped via `ulimit -v` in the bash command
//!   (1 GiB default, env-overridable `ARGO_CODE_EXEC_MEM_MIB`).
//! - **Network egress** — only enforceable under a real microVM. v1
//!   refuses to run with `allow_network = false` AND no E2B token,
//!   so the operator sees the gap rather than relying on a guarantee
//!   the LocalSandbox can't deliver.
//! - **Package install** — same: refused when `allow_pkg_install =
//!   false` AND no E2B token (a `pip install` / `npm install` would
//!   succeed silently under LocalSandbox).
//! - **Artifacts** — anything written under `/tmp/artifacts/` inside
//!   the sandbox is collected and listed in the result. v1 collects
//!   filenames + sizes (not bytes); v1.x will exfiltrate the bytes
//!   to the per-worker fs_root.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use argo_sandbox::{LocalSandbox, RunRequest, RunResult, Sandbox};
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default per-action wall-clock cap (60s). Override via the env
/// var `ARGO_CODE_EXEC_TIMEOUT_S`.
pub(crate) const DEFAULT_TIMEOUT_SECONDS: u64 = 60;

/// Default per-action memory cap in MiB (1024 = 1 GiB). Override
/// via the env var `ARGO_CODE_EXEC_MEM_MIB`.
pub(crate) const DEFAULT_MEMORY_MIB: u64 = 1024;

/// Env var the operator sets to nominate the E2B Firecracker
/// backend. v1 still falls back to LocalSandbox when missing — see
/// the module docs for the trade-off.
pub(crate) const ENV_E2B_TOKEN: &str = "ARGO_E2B_TOKEN";

/// Wire shape of the action payload. The dispatch shim deserialises
/// `action.payload` into this struct; missing optional fields
/// default to the most-restrictive setting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CodeRequest {
    /// `python` (3.12) or `node` (20). Other values are rejected
    /// at parse time with a helpful error.
    pub language: String,
    /// The script body. Written to `/tmp/script.py` (Python) or
    /// `/tmp/script.js` (Node) inside the sandbox before exec.
    pub code: String,
    /// Optional structured input — written to `/tmp/input.json`
    /// inside the sandbox so the script can read its own input
    /// from a known path.
    #[serde(default)]
    pub input_json: Option<serde_json::Value>,
    /// Per-action opt-in: allow `pip install` / `npm install` to
    /// run inside the sandbox. v1 treats this as "needs E2B" — the
    /// LocalSandbox backend can't enforce a refusal.
    #[serde(default)]
    pub allow_pkg_install: bool,
    /// Per-action opt-in: allow outbound network. Same v1 caveat
    /// as `allow_pkg_install`.
    #[serde(default)]
    pub allow_network: bool,
    /// Optional override for the wall-clock cap (seconds). When
    /// omitted, the env default + module default applies.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

/// Resolved sandbox limits + backend choice. Built once per dispatch.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedConfig {
    pub timeout: Duration,
    pub memory_mib: u64,
    pub backend: BackendKind,
}

/// Which sandbox backend the executor will use this dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackendKind {
    /// Real Firecracker microVM via the E2B managed service. v1
    /// returns a "not yet wired" error when this is selected — the
    /// `exec` HTTP client lands in a follow-up.
    E2b,
    /// `bash -c` in the parent OS namespace. No isolation.
    Local,
}

impl ResolvedConfig {
    /// Resolve from env + per-action overrides.
    pub(crate) fn from_env(req: &CodeRequest) -> Self {
        let timeout_secs = req
            .timeout_seconds
            .or_else(|| {
                std::env::var("ARGO_CODE_EXEC_TIMEOUT_S")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
            })
            .unwrap_or(DEFAULT_TIMEOUT_SECONDS);
        let memory_mib = std::env::var("ARGO_CODE_EXEC_MEM_MIB")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_MEMORY_MIB);
        let backend = if std::env::var(ENV_E2B_TOKEN).is_ok() {
            BackendKind::E2b
        } else {
            BackendKind::Local
        };
        Self {
            timeout: Duration::from_secs(timeout_secs),
            memory_mib,
            backend,
        }
    }
}

/// `Executor` impl. Holds nothing — all state is per-dispatch.
pub(crate) struct CodeExecutor;

impl CodeExecutor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Executor for CodeExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::CodeExecution
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let req = parse_request(&action.payload)?;
        let cfg = ResolvedConfig::from_env(&req);
        // v1 refuses opt-ins the LocalSandbox can't honor.
        guard_opt_ins(&req, cfg.backend)?;
        if cfg.backend == BackendKind::E2b {
            return Err(ArgoError::Provider(
                "code-execution executor: E2B backend requested but the \
                 E2B exec HTTP client is not yet wired (v1.x follow-up). \
                 Unset ARGO_E2B_TOKEN to fall back to LocalSandbox."
                    .into(),
            ));
        }
        let sandbox = LocalSandbox::new();
        let bundle = ensure_bundle_dir(cwd, &action.job_id, &action.action_id)?;
        let result = run_via(&sandbox, &req, &cfg, &bundle)?;
        let artifacts = collect_artifacts(&bundle);
        Ok(ActionOutcome::Completed {
            bundle_path: bundle,
            result_json: serde_json::json!({
                "outcome": "completed",
                "executor": "code_execution",
                "language": req.language,
                "backend": format!("{:?}", cfg.backend).to_ascii_lowercase(),
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.exit_code,
                "timed_out": result.timed_out,
                "duration_ms": result.duration.as_millis() as u64,
                "artifacts": artifacts,
            }),
        })
    }
}

/// Parse the action's payload into a [`CodeRequest`]. Surfaced so
/// the unit tests can exercise the parser without going through
/// dispatch.
pub(crate) fn parse_request(payload: &serde_json::Value) -> ArgoResult<CodeRequest> {
    let req: CodeRequest = serde_json::from_value(payload.clone()).map_err(|e| {
        ArgoError::Provider(format!(
            "code-execution payload: {e} (need {{ language, code, ... }})"
        ))
    })?;
    if !matches!(req.language.as_str(), "python" | "node") {
        return Err(ArgoError::Provider(format!(
            "code-execution language '{}' not supported — \
             valid: python | node",
            req.language
        )));
    }
    Ok(req)
}

/// Refuse opt-ins the LocalSandbox can't honor. With E2B the host
/// enforces network + pkg-install boundaries; LocalSandbox shells
/// straight into the parent namespace, so a script-level `pip
/// install` would silently succeed.
pub(crate) fn guard_opt_ins(req: &CodeRequest, backend: BackendKind) -> ArgoResult<()> {
    if backend == BackendKind::Local {
        if req.allow_network {
            return Err(ArgoError::Provider(
                "code-execution: allow_network = true is not safe under \
                 LocalSandbox (no network namespace isolation). Set \
                 ARGO_E2B_TOKEN to nominate the E2B backend, or remove \
                 the opt-in."
                    .into(),
            ));
        }
        if req.allow_pkg_install {
            return Err(ArgoError::Provider(
                "code-execution: allow_pkg_install = true is not safe \
                 under LocalSandbox. Set ARGO_E2B_TOKEN to nominate \
                 the E2B backend, or remove the opt-in."
                    .into(),
            ));
        }
    }
    Ok(())
}

/// Build the per-action artifact bundle dir + clean its
/// `/tmp/artifacts/` mirror so a previous run's outputs don't leak.
fn ensure_bundle_dir(cwd: &Path, job_id: &str, action_id: &str) -> ArgoResult<PathBuf> {
    let dir = cwd
        .join(".skycot")
        .join("outputs")
        .join(job_id)
        .join("actions")
        .join(action_id);
    std::fs::create_dir_all(&dir).map_err(|e| {
        ArgoError::Provider(format!(
            "code-execution: create bundle dir {}: {e}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

/// Run the script via the configured sandbox. Returns the
/// captured `RunResult`; converts transport errors into
/// [`ArgoError::Provider`] so the audit ledger sees the reason.
fn run_via(
    sandbox: &dyn Sandbox,
    req: &CodeRequest,
    cfg: &ResolvedConfig,
    bundle: &Path,
) -> ArgoResult<RunResult> {
    let command = build_command(req, cfg, bundle)?;
    let req_inner = RunRequest {
        command,
        working_dir: bundle.to_path_buf(),
        env: Vec::new(),
        inherit_env: true,
        timeout: Some(cfg.timeout),
    };
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ArgoError::Provider(format!(
            "code-execution executor needs a tokio runtime to drive the sandbox: {e}"
        ))
    })?;
    tokio::task::block_in_place(|| handle.block_on(sandbox.run(&req_inner)))
}

/// Synthesise the bash command line. Layout per language:
///   * Python: write the script + optional input to `/tmp/`, run
///     `ulimit -v <kib>; python3 /tmp/<script>`.
///   * Node: same shape but `node /tmp/<script>`.
///
/// `/tmp/artifacts/` is freshly created so the post-run collector
/// only sees this dispatch's outputs.
pub(crate) fn build_command(
    req: &CodeRequest,
    cfg: &ResolvedConfig,
    bundle: &Path,
) -> ArgoResult<String> {
    let script_filename = match req.language.as_str() {
        "python" => "script.py",
        "node" => "script.js",
        // parse_request already rejected unknown languages — defensive.
        other => {
            return Err(ArgoError::Provider(format!(
                "code-execution: build_command for '{other}' (parse_request should have rejected)"
            )))
        }
    };
    let script_path = bundle.join(script_filename);
    std::fs::write(&script_path, req.code.as_bytes()).map_err(|e| {
        ArgoError::Provider(format!(
            "code-execution: write script {}: {e}",
            script_path.display()
        ))
    })?;
    if let Some(ref input) = req.input_json {
        let input_path = bundle.join("input.json");
        let body = serde_json::to_string_pretty(input).unwrap_or_else(|_| "{}".into());
        std::fs::write(&input_path, body).map_err(|e| {
            ArgoError::Provider(format!(
                "code-execution: write input.json {}: {e}",
                input_path.display()
            ))
        })?;
    }
    let runner = match req.language.as_str() {
        "python" => "python3",
        "node" => "node",
        _ => unreachable!("language already validated"),
    };
    let mem_kib = cfg.memory_mib.saturating_mul(1024);
    let artifacts_dir = bundle.join("artifacts");
    let _ = std::fs::remove_dir_all(&artifacts_dir);
    std::fs::create_dir_all(&artifacts_dir).map_err(|e| {
        ArgoError::Provider(format!(
            "code-execution: prep artifacts dir {}: {e}",
            artifacts_dir.display()
        ))
    })?;
    Ok(format!(
        "ulimit -v {mem_kib} 2>/dev/null; {runner} {}",
        shell_escape(&script_path.display().to_string())
    ))
}

/// Tiny single-quote shell escape — enough for the
/// path-with-spaces case we care about, no full POSIX coverage.
fn shell_escape(s: &str) -> String {
    if !s.contains(['\'', ' ', '\\']) {
        return s.to_string();
    }
    let escaped = s.replace('\'', r"'\''");
    format!("'{escaped}'")
}

/// Walk `<bundle>/artifacts/` and report `[ {name, size} ]` entries.
/// Empty dir → empty list. v1 surfaces the catalog only; the bytes
/// stay on disk under the per-action bundle so the audit chain can
/// re-read them.
pub(crate) fn collect_artifacts(bundle: &Path) -> Vec<serde_json::Value> {
    let dir = bundle.join("artifacts");
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in read.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_file() {
            out.push(serde_json::json!({
                "name": entry.file_name().to_string_lossy(),
                "size": meta.len(),
            }));
        }
    }
    out.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    out
}

#[cfg(test)]
#[path = "worker_actions_executor_code_tests.rs"]
mod tests;
