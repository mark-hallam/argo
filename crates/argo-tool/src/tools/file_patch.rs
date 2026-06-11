//! `file_patch` — replace a unique string in a file with a new string.
//!
//! Conservative-by-design: the call fails if `old_str` does not appear
//! exactly once. That's the standard interface for AI-assisted file
//! editing — it forces the agent to disambiguate before making changes.
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

/// In-place string-replace patcher; refuses to apply when the match
/// count doesn't match `expected_replacements` (default 1).
#[derive(Debug, Default)]
pub struct FilePatch;

#[derive(Debug, Deserialize)]
struct Args {
    path: String,
    old_str: String,
    new_str: String,
    #[serde(default)]
    expected_replacements: Option<u32>,
}

#[async_trait]
impl Tool for FilePatch {
    fn name(&self) -> &str {
        "file_patch"
    }
    fn description(&self) -> &str {
        "Replace `old_str` with `new_str` in a file. Fails if `old_str` \
         does not appear exactly once (or `expected_replacements` times \
         when set)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["path", "old_str", "new_str"],
            "properties": {
                "path": { "type": "string" },
                "old_str": { "type": "string" },
                "new_str": { "type": "string" },
                "expected_replacements": { "type": "integer", "minimum": 1 },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::ExclusiveBy("file_patch")
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed: Args =
            serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
                tool: "file_patch".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        Ok(CapabilitySet(vec![
            Capability::FsRead {
                path_glob: parsed.path.clone(),
            },
            Capability::FsWrite {
                path_glob: parsed.path,
            },
        ]))
    }

    fn required_capabilities_in_ctx(
        &self,
        args: &serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<CapabilitySet> {
        let parsed: Args =
            serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
                tool: "file_patch".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        // cu1c — resolve a relative path against the jail root so the
        // derived globs are absolute and match the absolute grant.
        let path_glob = crate::path_guard::resolve_against_root(&ctx.workspace_root, &parsed.path);
        Ok(CapabilitySet(vec![
            Capability::FsRead {
                path_glob: path_glob.clone(),
            },
            Capability::FsWrite { path_glob },
        ]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "file_patch".into(),
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
        let original = match tokio::fs::read_to_string(&resolved).await {
            Ok(t) => t,
            Err(e) => {
                return Ok(error_text(
                    ctx.call_id,
                    format!("read {} for patch: {e}", resolved.display()),
                    started.elapsed().as_millis() as u64,
                ))
            }
        };
        let count = original.matches(&parsed.old_str).count();
        let want = parsed.expected_replacements.unwrap_or(1) as usize;
        if count != want {
            return Ok(error_text(
                ctx.call_id,
                format!(
                    "patch refused: expected {want} match{} of `old_str`, found {count}",
                    if want == 1 { "" } else { "es" }
                ),
                started.elapsed().as_millis() as u64,
            ));
        }
        let patched = original.replace(&parsed.old_str, &parsed.new_str);
        match tokio::fs::write(&resolved, patched.as_bytes()).await {
            Ok(()) => Ok(success_text(
                ctx.call_id,
                format!(
                    "patched {} ({count} replacement{} applied)",
                    resolved.display(),
                    if count == 1 { "" } else { "s" }
                ),
                started.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(error_text(
                ctx.call_id,
                format!("write {} after patch: {e}", resolved.display()),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::Principal;

    fn ctx_for(dir: &std::path::Path) -> ToolContext {
        ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.to_path_buf(),
        )
    }

    #[tokio::test]
    async fn patches_unique_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        tokio::fs::write(&path, "hello world\n").await.unwrap();
        let r = FilePatch
            .dispatch(
                serde_json::json!({
                    "path": "a.txt",
                    "old_str": "hello",
                    "new_str": "goodbye"
                }),
                &ctx_for(dir.path()),
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        assert_eq!(
            tokio::fs::read_to_string(&path).await.unwrap(),
            "goodbye world\n"
        );
    }

    #[tokio::test]
    async fn rejects_zero_matches() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.txt"), "hello\n")
            .await
            .unwrap();
        let r = FilePatch
            .dispatch(
                serde_json::json!({
                    "path": "a.txt",
                    "old_str": "missing",
                    "new_str": "x"
                }),
                &ctx_for(dir.path()),
            )
            .await
            .unwrap();
        assert!(r.is_error);
    }

    #[tokio::test]
    async fn rejects_ambiguous_matches() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.txt"), "x x x\n")
            .await
            .unwrap();
        let r = FilePatch
            .dispatch(
                serde_json::json!({
                    "path": "a.txt",
                    "old_str": "x",
                    "new_str": "y"
                }),
                &ctx_for(dir.path()),
            )
            .await
            .unwrap();
        assert!(r.is_error);
    }

    // cu1c — relative paths resolve against the jail root for BOTH the
    // FsRead and FsWrite capability globs file_patch derives.

    #[test]
    fn in_ctx_resolves_relative_against_root() {
        let ctx = ctx_for(std::path::Path::new("/tmp/w"));
        let caps = FilePatch
            .required_capabilities_in_ctx(
                &serde_json::json!({"path": "a.txt", "old_str": "x", "new_str": "y"}),
                &ctx,
            )
            .unwrap();
        let grant = CapabilitySet(vec![
            Capability::FsRead {
                path_glob: "/tmp/w/**".into(),
            },
            Capability::FsWrite {
                path_glob: "/tmp/w/**".into(),
            },
        ]);
        // Both derived caps (read + write) resolve to /tmp/w/a.txt and
        // are covered by the absolute grant.
        assert_eq!(caps.0.len(), 2);
        for cap in &caps.0 {
            match cap {
                Capability::FsRead { path_glob } | Capability::FsWrite { path_glob } => {
                    assert_eq!(path_glob, "/tmp/w/a.txt");
                }
                other => panic!("unexpected cap {other:?}"),
            }
            assert!(grant.covers(cap));
        }
    }

    #[test]
    fn in_ctx_climb_out_is_denied_by_grant() {
        let ctx = ctx_for(std::path::Path::new("/tmp/w"));
        let caps = FilePatch
            .required_capabilities_in_ctx(
                &serde_json::json!({"path": "../escape", "old_str": "x", "new_str": "y"}),
                &ctx,
            )
            .unwrap();
        let grant = CapabilitySet(vec![
            Capability::FsRead {
                path_glob: "/tmp/w/**".into(),
            },
            Capability::FsWrite {
                path_glob: "/tmp/w/**".into(),
            },
        ]);
        for cap in &caps.0 {
            assert!(!grant.covers(cap), "climb-out cap must be denied: {cap:?}");
        }
    }

    #[tokio::test]
    async fn expected_replacements_allows_n_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        tokio::fs::write(&path, "x x x").await.unwrap();
        let r = FilePatch
            .dispatch(
                serde_json::json!({
                    "path": "a.txt",
                    "old_str": "x",
                    "new_str": "y",
                    "expected_replacements": 3,
                }),
                &ctx_for(dir.path()),
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "y y y");
    }
}
