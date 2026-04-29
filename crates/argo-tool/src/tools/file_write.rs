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
        let resolved = ctx.workspace_root.join(&parsed.path);
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
}
