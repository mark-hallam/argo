//! `file_read` — read a file (or a line range) from the workspace.
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

/// Default LOC cap when both `start_line` and `end_line` are omitted —
/// keeps the model context bounded without requiring callers to chunk.
const DEFAULT_LINE_LIMIT: usize = 2_000;

/// File reader. Returns the text body, or a slice of it.
#[derive(Debug, Default)]
pub struct FileRead;

#[derive(Debug, Deserialize)]
struct Args {
    path: String,
    #[serde(default)]
    start_line: Option<u32>,
    #[serde(default)]
    end_line: Option<u32>,
}

#[async_trait]
impl Tool for FileRead {
    fn name(&self) -> &str {
        "file_read"
    }
    fn description(&self) -> &str {
        "Read a file from the workspace. Returns the file's text content. \
         Provide `start_line` / `end_line` (1-based, inclusive) to read a \
         range; otherwise the first 2000 lines are returned."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string", "description": "Workspace-relative or absolute path." },
                "start_line": { "type": "integer", "minimum": 1 },
                "end_line": { "type": "integer", "minimum": 1 },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed: Args =
            serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
                tool: "file_read".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        Ok(CapabilitySet(vec![Capability::FsRead {
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
                tool: "file_read".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        // cu1c — resolve a relative path against the jail root so the
        // derived glob is absolute and matches the absolute grant.
        let path_glob = crate::path_guard::resolve_against_root(&ctx.workspace_root, &parsed.path);
        Ok(CapabilitySet(vec![Capability::FsRead { path_glob }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "file_read".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        // #677 — runtime path confinement. Refuses paths that
        // would escape the workspace root.
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
        match tokio::fs::read_to_string(&resolved).await {
            Ok(text) => {
                let body = slice_lines(&text, parsed.start_line, parsed.end_line);
                Ok(success_text(
                    ctx.call_id,
                    body,
                    started.elapsed().as_millis() as u64,
                ))
            }
            Err(e) => Ok(error_text(
                ctx.call_id,
                format!("failed to read {}: {e}", resolved.display()),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

fn slice_lines(text: &str, start: Option<u32>, end: Option<u32>) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    let lo = start.map(|s| (s as usize).saturating_sub(1)).unwrap_or(0);
    let hi_default = (lo + DEFAULT_LINE_LIMIT).min(total);
    let hi = end.map(|e| (e as usize).min(total)).unwrap_or(hi_default);
    let lo = lo.min(total);
    let hi = hi.max(lo);
    lines[lo..hi].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_capability::CapabilityEngine;
    use argo_types::Principal;
    use std::sync::Arc;

    #[tokio::test]
    async fn reads_full_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        tokio::fs::write(&path, "hello\nworld").await.unwrap();
        let reg_tool: Arc<dyn Tool> = Arc::new(FileRead);
        let engine = CapabilityEngine::with_defaults();
        engine
            .bind(
                Principal::Local {
                    os_user: "u".into(),
                },
                "owner".into(),
            )
            .unwrap();
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = reg_tool
            .dispatch(serde_json::json!({"path": "hello.txt"}), &ctx)
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = match &result.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert_eq!(text, "hello\nworld");
    }

    #[tokio::test]
    async fn missing_file_yields_is_error_true() {
        let dir = tempfile::tempdir().unwrap();
        let tool = FileRead;
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = tool
            .dispatch(serde_json::json!({"path": "nope.txt"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[test]
    fn slice_lines_respects_range() {
        let txt = "1\n2\n3\n4\n5";
        assert_eq!(slice_lines(txt, None, None), "1\n2\n3\n4\n5");
        assert_eq!(slice_lines(txt, Some(2), Some(4)), "2\n3\n4");
        assert_eq!(slice_lines(txt, Some(4), None), "4\n5");
        assert_eq!(slice_lines(txt, Some(1), Some(1)), "1");
        // Out-of-range end clamps to total.
        assert_eq!(slice_lines(txt, Some(4), Some(99)), "4\n5");
    }

    #[test]
    fn required_capability_narrows_to_path() {
        let tool = FileRead;
        let caps = tool
            .required_capabilities(&serde_json::json!({"path": "foo/bar"}))
            .unwrap();
        assert!(caps.covers(&Capability::FsRead {
            path_glob: "foo/bar".into()
        }));
    }

    #[test]
    fn required_capability_rejects_bad_args() {
        let tool = FileRead;
        let err = tool
            .required_capabilities(&serde_json::json!({"missing": "path"}))
            .unwrap_err();
        assert!(matches!(err, argo_types::ArgoError::Tool { .. }));
    }

    // cu1c — relative paths resolve against the jail root for the
    // capability glob; the absolute grant then matches in-fence
    // relatives and misses climb-outs.

    fn ctx_at(root: &str) -> ToolContext {
        ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::path::PathBuf::from(root),
        )
    }

    fn read_glob(caps: &CapabilitySet) -> String {
        match &caps.0[0] {
            Capability::FsRead { path_glob } => path_glob.clone(),
            other => panic!("expected FsRead, got {other:?}"),
        }
    }

    #[test]
    fn in_ctx_resolves_relative_against_root() {
        let ctx = ctx_at("/tmp/w");
        let caps = FileRead
            .required_capabilities_in_ctx(&serde_json::json!({"path": "a.txt"}), &ctx)
            .unwrap();
        // OS-native separators (backslash on Windows) — normalize for the
        // logical-layout assertion. No-op on unix.
        assert_eq!(read_glob(&caps).replace('\\', "/"), "/tmp/w/a.txt");
        // The absolute grant <root>/** covers the resolved relative path.
        let grant = CapabilitySet(vec![Capability::FsRead {
            path_glob: "/tmp/w/**".into(),
        }]);
        assert!(grant.covers(&caps.0[0]));
    }

    #[test]
    fn in_ctx_climb_out_is_denied_by_grant() {
        let ctx = ctx_at("/tmp/w");
        let caps = FileRead
            .required_capabilities_in_ctx(&serde_json::json!({"path": "../escape"}), &ctx)
            .unwrap();
        assert!(!read_glob(&caps).starts_with("/tmp/w/"));
        let grant = CapabilitySet(vec![Capability::FsRead {
            path_glob: "/tmp/w/**".into(),
        }]);
        assert!(!grant.covers(&caps.0[0]));
    }

    #[test]
    fn in_ctx_absolute_path_unchanged() {
        let ctx = ctx_at("/tmp/w");
        // Must be absolute on the *host* so the passthrough branch triggers:
        // "/etc/passwd" is not absolute on Windows (no drive prefix).
        #[cfg(windows)]
        let abs = r"C:\Windows\System32\config\SAM";
        #[cfg(not(windows))]
        let abs = "/etc/passwd";
        let caps = FileRead
            .required_capabilities_in_ctx(&serde_json::json!({ "path": abs }), &ctx)
            .unwrap();
        assert_eq!(read_glob(&caps), abs);
        let grant = CapabilitySet(vec![Capability::FsRead {
            path_glob: "/tmp/w/**".into(),
        }]);
        assert!(!grant.covers(&caps.0[0]));
    }

    // #677 — runtime path confinement.

    #[tokio::test]
    async fn refuses_parent_traversal_path() {
        let dir = tempfile::tempdir().unwrap();
        let tool: Arc<dyn Tool> = Arc::new(FileRead);
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = tool
            .dispatch(serde_json::json!({"path": "../escape.txt"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        let body = match &result.content[0] {
            argo_types::ContentBlock::Text { text } => text,
            _ => panic!("expected text"),
        };
        assert!(body.contains("`..`"), "got: {body}");
    }

    #[tokio::test]
    async fn refuses_absolute_path_outside_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let tool: Arc<dyn Tool> = Arc::new(FileRead);
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.path().to_path_buf(),
        );
        let result = tool
            .dispatch(serde_json::json!({"path": "/etc/passwd"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        let body = match &result.content[0] {
            argo_types::ContentBlock::Text { text } => text,
            _ => panic!("expected text"),
        };
        assert!(body.contains("outside the workspace root"), "got: {body}");
    }
}
