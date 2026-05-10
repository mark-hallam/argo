//! `code_execute` — run ad-hoc code through `argo_sandbox::Sandbox`.
//!
//! Hermes parity (mirrors `code_execution_tool.py`): an agent supplies
//! a `language` (`python` | `node` | `bash`), a `source` script body,
//! and optionally a `files` map (filename → contents) of side-cars.
//! The tool stages everything in a fresh tempdir, invokes the matching
//! interpreter through the sandbox backend, and returns
//! `{ stdout, stderr, exit, timed_out, created_files }` as a JSON
//! body. Stdout / stderr are capped at 64 KiB per stream and the
//! response carries `stdout_truncated` / `stderr_truncated` markers.
//! Files created by the script (anything in the staged dir that
//! wasn't part of the input set, modulo Python `__pycache__`) are
//! reported as `created_files: [{ path, bytes }]`.
//!
//! Capability gating happens twice along the dispatch path:
//!
//! - `Capability::ToolDispatch { tool_name_glob: "code_execute" }` —
//!   verified by [`crate::ToolRegistry::dispatch`] like every tool.
//! - `Capability::Spawn { spawn_kind: SpawnKind::Container }` —
//!   declared by [`Tool::required_capabilities`]; the dispatcher
//!   verifies BOTH before invoking. Sandbox isolation is gated even
//!   when the configured backend is the local fallback so vertical
//!   manifests stay portable across stricter container runtimes.
//!
//! E2B / Firecracker were named in the operator brief but the
//! in-tree sandbox today only ships `local` + `docker` (see
//! `argo_sandbox`). The tool routes through whichever backend is
//! supplied; future backends slot in without changing this file.
//!
//! ```
//! use argo_tool::{CodeExecute, Tool};
//! let t = CodeExecute::default();
//! assert_eq!(t.name(), "code_execute");
//! ```
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_sandbox::{LocalSandbox, RunRequest, RunResult, Sandbox};
use argo_types::{ArgoResult, Capability, CapabilitySet, SpawnKind, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const TRUNCATION_MARKER: &str = "[truncated ";

/// Sandboxed code-execution tool.
#[derive(Debug, Clone)]
pub struct CodeExecute {
    sandbox: Arc<dyn Sandbox>,
}

impl Default for CodeExecute {
    fn default() -> Self {
        Self {
            sandbox: Arc::new(LocalSandbox::new()),
        }
    }
}

impl CodeExecute {
    /// Build a tool that dispatches through the supplied backend.
    /// Default uses [`LocalSandbox`]; operators can swap in
    /// [`argo_sandbox::DockerSandbox`] (or future Firecracker / E2B
    /// backends) without touching the tool surface.
    #[must_use]
    pub fn with_sandbox(sandbox: Arc<dyn Sandbox>) -> Self {
        Self { sandbox }
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    language: String,
    source: String,
    #[serde(default)]
    files: Option<BTreeMap<String, String>>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
enum Lang {
    Python,
    Node,
    Bash,
}

impl Lang {
    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "python" => Ok(Self::Python),
            "node" => Ok(Self::Node),
            "bash" => Ok(Self::Bash),
            other => Err(format!(
                "unknown language '{other}': supported = python | node | bash"
            )),
        }
    }

    fn entrypoint(self) -> &'static str {
        match self {
            Self::Python => "main.py",
            Self::Node => "main.js",
            Self::Bash => "main.sh",
        }
    }

    fn command(self) -> &'static str {
        match self {
            // -B suppresses .pyc / __pycache__ so the created-files
            // snapshot only contains files the script itself wrote.
            Self::Python => "python3 -B main.py",
            Self::Node => "node main.js",
            Self::Bash => "bash main.sh",
        }
    }
}

#[async_trait]
impl Tool for CodeExecute {
    fn name(&self) -> &str {
        "code_execute"
    }

    fn description(&self) -> &str {
        "Run ad-hoc code in a sandboxed workspace. Languages: \
         `python` (python3 -B), `node`, `bash`. Provide `source` \
         (the script body) and optionally a `files` map (filename → \
         contents) of side-cars. Returns stdout, stderr, exit code, \
         truncation markers, and the list of files created during \
         the run."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["language", "source"],
            "properties": {
                "language": {
                    "type": "string",
                    "enum": ["python", "node", "bash"],
                },
                "source": { "type": "string" },
                "files": {
                    "type": "object",
                    "description": "Optional filename → contents side-cars staged alongside the entrypoint.",
                    "additionalProperties": { "type": "string" },
                },
                "timeout_ms": { "type": "integer", "minimum": 100 },
            },
            "additionalProperties": false,
        })
    }

    fn concurrency(&self) -> Concurrency {
        Concurrency::Serial
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        Ok(CapabilitySet(vec![Capability::Spawn {
            spawn_kind: SpawnKind::Container,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return Ok(err(ctx, started, format!("invalid arguments: {e}"))),
        };
        let lang = match Lang::parse(&parsed.language) {
            Ok(l) => l,
            Err(msg) => return Ok(err(ctx, started, msg)),
        };
        let staged = match stage_workspace(&parsed.source, parsed.files.as_ref(), lang).await {
            Ok(s) => s,
            Err(msg) => return Ok(err(ctx, started, msg)),
        };
        let mut req = RunRequest::new(lang.command(), staged.dir.path());
        req.timeout = Some(Duration::from_millis(
            parsed.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS),
        ));
        let run = match self.sandbox.run(&req).await {
            Ok(r) => r,
            Err(e) => return Ok(err(ctx, started, format!("sandbox dispatch failed: {e}"))),
        };
        let created = match snapshot_created_files(staged.dir.path(), &staged.staged_rel) {
            Ok(c) => c,
            Err(e) => return Ok(err(
                ctx,
                started,
                format!("failed to snapshot created files: {e}"),
            )),
        };
        Ok(success_text(
            ctx.call_id,
            format_result(&run, &created),
            started.elapsed().as_millis() as u64,
        ))
    }
}

fn err(ctx: &ToolContext, started: Instant, msg: String) -> ToolCallResult {
    error_text(ctx.call_id, msg, started.elapsed().as_millis() as u64)
}

struct Staged {
    dir: tempfile::TempDir,
    staged_rel: HashSet<PathBuf>,
}

async fn stage_workspace(
    source: &str,
    files: Option<&BTreeMap<String, String>>,
    lang: Lang,
) -> Result<Staged, String> {
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let mut staged_rel: HashSet<PathBuf> = HashSet::new();
    let entry = PathBuf::from(lang.entrypoint());
    let entry_abs = dir.path().join(&entry);
    tokio::fs::write(&entry_abs, source.as_bytes())
        .await
        .map_err(|e| format!("write {}: {e}", entry.display()))?;
    staged_rel.insert(entry);
    if let Some(files) = files {
        for (name, contents) in files {
            let rel = PathBuf::from(name);
            if rel.is_absolute()
                || rel
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(format!(
                    "files key '{name}': must be a relative filename without parent components"
                ));
            }
            let target = dir.path().join(&rel);
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("create_dir_all {}: {e}", parent.display()))?;
            }
            tokio::fs::write(&target, contents.as_bytes())
                .await
                .map_err(|e| format!("write {name}: {e}"))?;
            staged_rel.insert(rel);
        }
    }
    Ok(Staged { dir, staged_rel })
}

fn snapshot_created_files(
    root: &Path,
    staged_rel: &HashSet<PathBuf>,
) -> std::io::Result<Vec<(String, u64)>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let path = entry.path();
            if ft.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some("__pycache__") {
                    continue;
                }
                stack.push(path);
            } else if ft.is_file() {
                let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                if staged_rel.contains(&rel) {
                    continue;
                }
                let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((rel.to_string_lossy().into_owned(), bytes));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn format_result(run: &RunResult, created: &[(String, u64)]) -> String {
    let body = serde_json::json!({
        "stdout": run.stdout,
        "stderr": run.stderr,
        "exit": run.exit_code,
        "timed_out": run.timed_out,
        "stdout_truncated": run.stdout.contains(TRUNCATION_MARKER),
        "stderr_truncated": run.stderr.contains(TRUNCATION_MARKER),
        "duration_ms": run.duration.as_millis() as u64,
        "created_files": created
            .iter()
            .map(|(p, b)| serde_json::json!({ "path": p, "bytes": b }))
            .collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::{ContentBlock, Principal};

    fn ctx() -> ToolContext {
        ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::env::temp_dir(),
        )
    }

    fn body(r: &ToolCallResult) -> String {
        match &r.content[0] {
            ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        }
    }

    fn interpreter_available(name: &str) -> bool {
        std::process::Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn schema_lists_three_languages() {
        let t = CodeExecute::default();
        let s = t.parameters();
        let langs = &s["properties"]["language"]["enum"];
        assert!(langs.as_array().unwrap().iter().any(|v| v == "python"));
        assert!(langs.as_array().unwrap().iter().any(|v| v == "node"));
        assert!(langs.as_array().unwrap().iter().any(|v| v == "bash"));
    }

    #[test]
    fn requires_spawn_container_capability() {
        let caps = CodeExecute::default()
            .required_capabilities(&serde_json::json!({"language":"bash","source":""}))
            .unwrap();
        assert!(caps.covers(&Capability::Spawn {
            spawn_kind: SpawnKind::Container,
        }));
        assert!(!caps.covers(&Capability::Spawn {
            spawn_kind: SpawnKind::Subprocess,
        }));
    }

    #[tokio::test]
    async fn unknown_language_returns_clean_error() {
        let r = CodeExecute::default()
            .dispatch(
                serde_json::json!({"language":"ruby","source":"puts 1"}),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(body(&r).contains("unknown language"));
    }

    #[tokio::test]
    async fn parent_traversal_in_files_is_rejected() {
        let r = CodeExecute::default()
            .dispatch(
                serde_json::json!({
                    "language": "bash",
                    "source": "echo hi",
                    "files": { "../escape.txt": "x" },
                }),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(body(&r).contains("must be a relative filename"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn python_hello_world_runs() {
        if !interpreter_available("python3") {
            return;
        }
        let r = CodeExecute::default()
            .dispatch(
                serde_json::json!({"language":"python","source":"print('hi-py')"}),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(!r.is_error, "{}", body(&r));
        let b = body(&r);
        assert!(b.contains("hi-py"), "{b}");
        assert!(b.contains("\"exit\": 0"), "{b}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn node_hello_world_runs() {
        if !interpreter_available("node") {
            return;
        }
        let r = CodeExecute::default()
            .dispatch(
                serde_json::json!({"language":"node","source":"console.log('hi-js')"}),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(!r.is_error, "{}", body(&r));
        let b = body(&r);
        assert!(b.contains("hi-js"), "{b}");
        assert!(b.contains("\"exit\": 0"), "{b}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bash_echo_runs() {
        let r = CodeExecute::default()
            .dispatch(
                serde_json::json!({"language":"bash","source":"echo hi-sh"}),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(!r.is_error, "{}", body(&r));
        let b = body(&r);
        assert!(b.contains("hi-sh"), "{b}");
        assert!(b.contains("\"exit\": 0"), "{b}");
    }
}
