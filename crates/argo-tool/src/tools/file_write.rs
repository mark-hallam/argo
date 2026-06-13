//! `file_write` — write (or overwrite) a file in the workspace.
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

/// File writer — overwrites or creates.
#[derive(Debug, Default)]
pub struct FileWrite;

#[derive(Debug, Deserialize)]
struct Args {
    path: String,
    content: String,
    #[serde(default)]
    create_dirs: bool,
}

#[async_trait]
impl Tool for FileWrite {
    fn name(&self) -> &str {
        "file_write"
    }
    fn description(&self) -> &str {
        "Write a file in the workspace, creating it if missing or \
         overwriting if it exists. Set `create_dirs` to `true` to create \
         missing parent directories."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["path", "content"],
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" },
                "create_dirs": { "type": "boolean", "default": false },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        // Two writers to the same path conflict; the agent loop's parallel
        // dispatcher serialises by path-key.
        Concurrency::ExclusiveBy("file_write")
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed: Args =
            serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
                tool: "file_write".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        Ok(CapabilitySet(vec![Capability::FsWrite {
            path_glob: parsed.path,
        }]))
    }

    fn required_capabilities_in_ctx(
        &self,
        args: &serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<CapabilitySet> {
        let parsed: Args =
            serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
                tool: "file_write".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        // cu1c — resolve a relative path against the jail root so the
        // derived glob is absolute and matches the absolute grant.
        let path_glob = crate::path_guard::resolve_against_root(&ctx.workspace_root, &parsed.path);
        Ok(CapabilitySet(vec![Capability::FsWrite { path_glob }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "file_write".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        // #677 — runtime path confinement.
        let resolved = match crate::path_guard::confine_path(
            &ctx.workspace_root,
            std::path::Path::new(&parsed.path),
        ) {
            Ok(p) => p,
            Err(msg) => {
                return Ok(error_text(
                    ctx.call_id,
                    msg,
                    started.elapsed().as_millis() as u64,
                ));
            }
        };
        if parsed.create_dirs {
            if let Some(parent) = resolved.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return Ok(error_text(
                        ctx.call_id,
                        format!("create_dir_all {}: {e}", parent.display()),
                        started.elapsed().as_millis() as u64,
                    ));
                }
            }
        }
        match tokio::fs::write(&resolved, parsed.content.as_bytes()).await {
            Ok(()) => Ok(success_text(
                ctx.call_id,
                format!(
                    "wrote {} bytes to {}",
                    parsed.content.len(),
                    resolved.display()
                ),
                started.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(error_text(
                ctx.call_id,
                format!("failed to write {}: {e}", resolved.display()),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::Principal;

    #[tokio::test]
    async fn writes_overwrites_and_returns_byte_count() {
        let dir = tempfile::tempdir().unwrap();
        let tool = FileWrite;
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = tool
            .dispatch(
                serde_json::json!({"path": "out.txt", "content": "hello"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let written = tokio::fs::read_to_string(dir.path().join("out.txt"))
            .await
            .unwrap();
        assert_eq!(written, "hello");
    }

    #[tokio::test]
    async fn create_dirs_makes_parent() {
        let dir = tempfile::tempdir().unwrap();
        let tool = FileWrite;
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = tool
            .dispatch(
                serde_json::json!({
                    "path": "nested/deep/file.txt",
                    "content": "x",
                    "create_dirs": true
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(dir.path().join("nested/deep/file.txt").exists());
    }

    #[tokio::test]
    async fn missing_parent_without_create_dirs_yields_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = FileWrite;
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = tool
            .dispatch(
                serde_json::json!({
                    "path": "nope/file.txt",
                    "content": "x"
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.is_error);
    }

    // #677 — runtime path confinement.

    #[tokio::test]
    async fn refuses_parent_traversal_path_for_write() {
        let dir = tempfile::tempdir().unwrap();
        let tool = FileWrite;
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = tool
            .dispatch(
                serde_json::json!({"path": "../escape.txt", "content": "x"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.is_error);
        // The escape file MUST NOT have been created.
        let escape = dir.path().parent().unwrap().join("escape.txt");
        assert!(
            !escape.exists(),
            "guard must reject before any fs::write call"
        );
    }

    // cu1c — relative paths resolve against the jail root for the
    // FsWrite capability glob.

    fn ctx_at(root: &str) -> ToolContext {
        ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::path::PathBuf::from(root),
        )
    }

    fn write_glob(caps: &CapabilitySet) -> String {
        match &caps.0[0] {
            Capability::FsWrite { path_glob } => path_glob.clone(),
            other => panic!("expected FsWrite, got {other:?}"),
        }
    }

    #[test]
    fn in_ctx_resolves_relative_against_root() {
        let ctx = ctx_at("/tmp/w");
        let caps = FileWrite
            .required_capabilities_in_ctx(
                &serde_json::json!({"path": "out.txt", "content": "x"}),
                &ctx,
            )
            .unwrap();
        // OS-native separators (backslash on Windows) — normalize for the
        // logical-layout assertion. No-op on unix.
        assert_eq!(write_glob(&caps).replace('\\', "/"), "/tmp/w/out.txt");
        let grant = CapabilitySet(vec![Capability::FsWrite {
            path_glob: "/tmp/w/**".into(),
        }]);
        assert!(grant.covers(&caps.0[0]));
    }

    #[test]
    fn in_ctx_climb_out_is_denied_by_grant() {
        let ctx = ctx_at("/tmp/w");
        let caps = FileWrite
            .required_capabilities_in_ctx(
                &serde_json::json!({"path": "../escape.txt", "content": "x"}),
                &ctx,
            )
            .unwrap();
        let grant = CapabilitySet(vec![Capability::FsWrite {
            path_glob: "/tmp/w/**".into(),
        }]);
        assert!(!grant.covers(&caps.0[0]));
    }

    #[test]
    fn in_ctx_absolute_path_unchanged() {
        let ctx = ctx_at("/tmp/w");
        // Must be absolute on the *host* so the passthrough branch triggers:
        // "/etc/evil" is not absolute on Windows (no drive prefix).
        #[cfg(windows)]
        let abs = r"C:\Windows\Temp\evil";
        #[cfg(not(windows))]
        let abs = "/etc/evil";
        let caps = FileWrite
            .required_capabilities_in_ctx(
                &serde_json::json!({ "path": abs, "content": "x" }),
                &ctx,
            )
            .unwrap();
        assert_eq!(write_glob(&caps), abs);
    }

    #[tokio::test]
    async fn refuses_absolute_path_outside_workspace_for_write() {
        let dir = tempfile::tempdir().unwrap();
        let tool = FileWrite;
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = tool
            .dispatch(
                serde_json::json!({"path": "/tmp/argo-escape-test.txt", "content": "x"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.is_error);
    }
}
