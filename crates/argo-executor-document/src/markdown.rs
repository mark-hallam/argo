//! Markdown backend — direct write-through. The simplest of the
//! four ops: the action payload's `body` already IS the artefact
//! body. We just persist it to the resolved path.

use crate::error::{DocExecutorError, DocResult};
use std::path::Path;

/// Write `body` to `target`. Creates the parent dir if missing.
/// Returns the byte count actually written so the executor can
/// surface it in the result envelope.
pub fn write_markdown(target: &Path, body: &str) -> DocResult<u64> {
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DocExecutorError::io(
                    format!("markdown: create parent {}", parent.display()),
                    e,
                )
            })?;
        }
    }
    std::fs::write(target, body.as_bytes()).map_err(|e| {
        DocExecutorError::io(format!("markdown: write {}", target.display()), e)
    })?;
    Ok(body.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_markdown_persists_body_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        let body = "# Title\n\nbody text with `code` and a [link](http://x)";
        let n = write_markdown(&target, body).unwrap();
        assert_eq!(n, body.len() as u64);
        let read = std::fs::read_to_string(&target).unwrap();
        assert_eq!(read, body);
    }

    #[test]
    fn write_markdown_creates_nested_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a/b/c/deep.md");
        write_markdown(&target, "ok").unwrap();
        assert!(target.exists());
    }
}
