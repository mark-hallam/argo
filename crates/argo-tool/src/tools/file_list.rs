//! `file_list` — list the entries of a directory in the workspace.
//!
//! Closes the stress-test P1 gap (2026-06-20): the built-in tool set
//! shipped `file_read` (a single known path) but no way to ENUMERATE a
//! directory, so an "ingest this folder / inbox / export dir" teammate
//! had to guess filenames and stalled. This is the read-only directory
//! lister — same `path_guard` jail as `file_read`, same `FsRead`
//! capability (listing is a read; no parallel `FsList` cap is minted).
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

/// Upper bound on entries returned, to keep the model context bounded
/// on a large directory. Truncation is reported in the body.
const MAX_ENTRIES: usize = 2_000;

/// Directory lister. Returns one line per entry (`kind  name  size`),
/// jailed to the workspace root.
#[derive(Debug, Default)]
pub struct FileList;

#[derive(Debug, Deserialize)]
struct Args {
    path: String,
    /// Optional `*`-wildcard filter applied to entry NAMES (not paths),
    /// e.g. `*.csv`, `reef-*`, `*report*`.
    #[serde(default)]
    glob: Option<String>,
}

#[async_trait]
impl Tool for FileList {
    fn name(&self) -> &str {
        "file_list"
    }
    fn description(&self) -> &str {
        "List the entries of a directory in the workspace. Returns one line \
         per entry as `<dir|file>  <name>  <size>`. Use this to discover what \
         files exist before reading them with `file_read`. Pass an optional \
         `glob` (`*` wildcards, e.g. `*.csv`) to filter entry names."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string", "description": "Workspace-relative or absolute directory path." },
                "glob": { "type": "string", "description": "Optional `*`-wildcard filter on entry names, e.g. `*.csv`." },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed: Args = parse_args(args)?;
        Ok(CapabilitySet(vec![Capability::FsRead {
            path_glob: parsed.path,
        }]))
    }

    fn required_capabilities_in_ctx(
        &self,
        args: &serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<CapabilitySet> {
        let parsed: Args = parse_args(args)?;
        // Resolve a relative path against the jail root so the derived glob is
        // absolute and matches the absolute grant (mirrors `file_read`).
        let path_glob = crate::path_guard::resolve_against_root(&ctx.workspace_root, &parsed.path);
        Ok(CapabilitySet(vec![Capability::FsRead { path_glob }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args = parse_args(&args)?;
        let elapsed = || started.elapsed().as_millis() as u64;

        // #677 — runtime path confinement. Refuses paths that escape root.
        let resolved = match crate::path_guard::confine_path(
            &ctx.workspace_root,
            std::path::Path::new(&parsed.path),
        ) {
            Ok(p) => p,
            Err(msg) => return Ok(error_text(ctx.call_id, msg, elapsed())),
        };

        let mut read_dir = match tokio::fs::read_dir(&resolved).await {
            Ok(rd) => rd,
            Err(e) => {
                return Ok(error_text(
                    ctx.call_id,
                    format!("failed to list {}: {e}", resolved.display()),
                    elapsed(),
                ));
            }
        };

        let mut entries: Vec<(bool, String, u64)> = Vec::new(); // (is_dir, name, size)
        let mut truncated = false;
        loop {
            match read_dir.next_entry().await {
                Ok(Some(entry)) => {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if let Some(pat) = parsed.glob.as_deref() {
                        if !glob_match(pat, &name) {
                            continue;
                        }
                    }
                    let (is_dir, size) = match entry.metadata().await {
                        Ok(m) => (m.is_dir(), m.len()),
                        Err(_) => (false, 0),
                    };
                    if entries.len() >= MAX_ENTRIES {
                        truncated = true;
                        break;
                    }
                    entries.push((is_dir, name, size));
                }
                Ok(None) => break,
                Err(e) => {
                    return Ok(error_text(
                        ctx.call_id,
                        format!("error reading {}: {e}", resolved.display()),
                        elapsed(),
                    ));
                }
            }
        }

        // Dirs first, then files; alphabetical within each.
        entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        Ok(success_text(
            ctx.call_id,
            render(&parsed.path, &entries, truncated),
            elapsed(),
        ))
    }
}

fn parse_args(args: &serde_json::Value) -> ArgoResult<Args> {
    serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
        tool: "file_list".into(),
        msg: format!("invalid arguments: {e}"),
    })
}

fn render(path: &str, entries: &[(bool, String, u64)], truncated: bool) -> String {
    if entries.is_empty() {
        return format!("{path}: (empty)");
    }
    let mut out = format!("{} entr{} in {path}:\n", entries.len(), if entries.len() == 1 { "y" } else { "ies" });
    for (is_dir, name, size) in entries {
        if *is_dir {
            out.push_str(&format!("dir   {name}\n"));
        } else {
            out.push_str(&format!("file  {name}  ({size} bytes)\n"));
        }
    }
    if truncated {
        out.push_str(&format!("… truncated at {MAX_ENTRIES} entries\n"));
    }
    out.truncate(out.trim_end().len());
    out
}

/// Minimal dependency-free `*`-wildcard matcher (no `?` / `[]`).
/// Matches the whole NAME against the pattern: `*.csv`, `reef-*`,
/// `*report*`.
fn glob_match(pattern: &str, name: &str) -> bool {
    fn helper(p: &[u8], n: &[u8]) -> bool {
        match p.first() {
            None => n.is_empty(),
            Some(b'*') => helper(&p[1..], n) || (!n.is_empty() && helper(p, &n[1..])),
            Some(&pc) => matches!(n.first(), Some(&nc) if nc == pc) && helper(&p[1..], &n[1..]),
        }
    }
    helper(pattern.as_bytes(), name.as_bytes())
}

#[cfg(test)]
#[path = "file_list_tests.rs"]
mod tests;
