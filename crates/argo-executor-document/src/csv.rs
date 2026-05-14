//! CSV backend — thin shim around the canonical RFC 4180 writer in
//! `argo-executor-document-csv`. Owns shape validation (row arity vs
//! header width) and the file-write convenience wrapper; quoting and
//! line-terminator rules live in the shared helper.

use crate::error::{DocExecutorError, DocResult};
use argo_executor_document_csv::render_rfc4180;
use std::path::Path;

/// Render the CSV bytes for the given header + rows. Returns the
/// final UTF-8 string; the executor writes it via [`write_csv_to`].
///
/// Rows with arity != header length raise
/// [`DocExecutorError::CsvShapeMismatch`] so a poisoned manifest
/// can't ship malformed CSV downstream.
pub fn render_csv(header: &[String], rows: &[Vec<String>]) -> DocResult<String> {
    for (i, row) in rows.iter().enumerate() {
        if row.len() != header.len() {
            return Err(DocExecutorError::CsvShapeMismatch {
                row_index: i,
                expected: header.len(),
                got: row.len(),
            });
        }
    }
    let bytes = render_rfc4180(header, rows);
    // render_rfc4180 only emits the caller's UTF-8 strings + ASCII
    // framing, so the result is always valid UTF-8.
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Convenience wrapper: render + write in one call.
pub fn write_csv_to(target: &Path, header: &[String], rows: &[Vec<String>]) -> DocResult<u64> {
    let body = render_csv(header, rows)?;
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DocExecutorError::io(format!("csv: create parent {}", parent.display()), e)
            })?;
        }
    }
    std::fs::write(target, body.as_bytes())
        .map_err(|e| DocExecutorError::io(format!("csv: write {}", target.display()), e))?;
    Ok(body.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_csv_quotes_field_with_comma() {
        let rendered =
            render_csv(&["a".into(), "b".into()], &[vec!["x,y".into(), "z".into()]]).unwrap();
        assert_eq!(rendered, "a,b\r\n\"x,y\",z\r\n");
    }

    #[test]
    fn render_csv_doubles_internal_quote() {
        let rendered = render_csv(
            &["q".into()],
            &[vec![r#"She said "hi""#.into()]],
        )
        .unwrap();
        assert_eq!(rendered, "q\r\n\"She said \"\"hi\"\"\"\r\n");
    }

    #[test]
    fn render_csv_quotes_field_with_newline() {
        let rendered = render_csv(
            &["body".into()],
            &[vec!["line1\nline2".into()]],
        )
        .unwrap();
        assert_eq!(rendered, "body\r\n\"line1\nline2\"\r\n");
    }

    #[test]
    fn render_csv_rejects_row_with_wrong_arity() {
        let err = render_csv(
            &["a".into(), "b".into(), "c".into()],
            &[vec!["x".into(), "y".into()]],
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                DocExecutorError::CsvShapeMismatch {
                    row_index: 0,
                    expected: 3,
                    got: 2
                }
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn render_csv_handles_empty_rows() {
        let rendered = render_csv(&["a".into(), "b".into()], &[]).unwrap();
        assert_eq!(rendered, "a,b\r\n");
    }

    #[test]
    fn write_csv_to_writes_bytes_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested/out.csv");
        let n = write_csv_to(
            &target,
            &["k".into()],
            &[vec!["v1".into()], vec!["v2".into()]],
        )
        .unwrap();
        let body = std::fs::read_to_string(&target).unwrap();
        assert_eq!(body, "k\r\nv1\r\nv2\r\n");
        assert_eq!(n, body.len() as u64);
    }
}
