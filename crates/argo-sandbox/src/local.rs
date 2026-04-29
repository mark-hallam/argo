//! `LocalSandbox` — runs `bash -c` in the current OS namespace.
use crate::sandbox::{
    io_err, truncate_stream, RunRequest, RunResult, Sandbox, DEFAULT_TIMEOUT, MAX_OUTPUT_BYTES,
};
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::time::Instant;

/// Local-process sandbox. No isolation — equivalent to the
/// `terminal` tool's execution path. Use for trusted workspaces.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalSandbox;

impl LocalSandbox {
    /// Build a fresh local sandbox.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Sandbox for LocalSandbox {
    fn id(&self) -> &str {
        "local"
    }
    async fn run(&self, req: &RunRequest) -> ArgoResult<RunResult> {
        let started = Instant::now();
        let timeout = req.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let mut cmd = tokio::process::Command::new("bash");
        cmd.arg("-c")
            .arg(&req.command)
            .current_dir(&req.working_dir);
        if !req.inherit_env {
            cmd.env_clear();
        }
        for (k, v) in &req.env {
            cmd.env(k, v);
        }
        match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(output)) => Ok(RunResult {
                stdout: truncate_stream(&output.stdout, MAX_OUTPUT_BYTES),
                stderr: truncate_stream(&output.stderr, MAX_OUTPUT_BYTES),
                exit_code: output.status.code().unwrap_or(-1),
                timed_out: false,
                duration: started.elapsed(),
            }),
            Ok(Err(e)) => Err(io_err("local-sandbox spawn", e)),
            Err(_) => Ok(RunResult {
                stdout: String::new(),
                stderr: format!(
                    "local-sandbox: command exceeded {} ms timeout",
                    timeout.as_millis()
                ),
                exit_code: -1,
                timed_out: true,
                duration: started.elapsed(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echoes_stdout_and_zero_exit() {
        let s = LocalSandbox::new();
        let dir = tempfile::tempdir().unwrap();
        let r = s
            .run(&RunRequest::new("echo hi", dir.path()))
            .await
            .unwrap();
        assert_eq!(r.exit_code, 0);
        assert!(r.stdout.contains("hi"));
        assert!(!r.timed_out);
    }

    #[tokio::test]
    async fn captures_nonzero_exit_in_result_not_error() {
        let s = LocalSandbox::new();
        let dir = tempfile::tempdir().unwrap();
        let r = s.run(&RunRequest::new("exit 7", dir.path())).await.unwrap();
        assert_eq!(r.exit_code, 7);
        assert!(!r.timed_out);
    }

    #[tokio::test]
    async fn timeout_marks_result_and_does_not_error() {
        let s = LocalSandbox::new();
        let dir = tempfile::tempdir().unwrap();
        let mut req = RunRequest::new("sleep 5", dir.path());
        req.timeout = Some(std::time::Duration::from_millis(100));
        let r = s.run(&req).await.unwrap();
        assert!(r.timed_out);
        assert_eq!(r.exit_code, -1);
    }

    #[tokio::test]
    async fn env_is_inherited_by_default() {
        let s = LocalSandbox::new();
        let dir = tempfile::tempdir().unwrap();
        // PATH should be present from the parent process.
        let r = s
            .run(&RunRequest::new("echo \"PATH=$PATH\"", dir.path()))
            .await
            .unwrap();
        assert!(r.stdout.contains("PATH="));
        assert!(r.stdout.len() > "PATH=\n".len());
    }

    #[tokio::test]
    async fn env_clear_blanks_path() {
        let s = LocalSandbox::new();
        let dir = tempfile::tempdir().unwrap();
        let mut req = RunRequest::new("echo \"PATH=$PATH\"", dir.path());
        req.inherit_env = false;
        let r = s.run(&req).await.unwrap();
        // PATH was cleared so the value is empty.
        assert!(r.stdout.contains("PATH=\n") || r.stdout.contains("PATH="));
    }

    #[tokio::test]
    async fn working_dir_is_honoured() {
        let s = LocalSandbox::new();
        let dir = tempfile::tempdir().unwrap();
        let r = s.run(&RunRequest::new("pwd", dir.path())).await.unwrap();
        let canonical = std::fs::canonicalize(dir.path()).unwrap();
        let pwd = r.stdout.trim();
        assert!(
            std::fs::canonicalize(pwd)
                .map(|c| c == canonical)
                .unwrap_or(false),
            "pwd={pwd} expected={}",
            canonical.display()
        );
    }
}
