//! Top-level façade — `DocRequest` + `DocExecutor` + bundle path
//! resolver.
//!
//! The dispatch shim in `argo-cli` calls `DocExecutor::run` with the
//! per-action bundle dir + parsed [`DocRequest`]. The bundle dir
//! has already been built under
//! `<cwd>/.skycot/outputs/<job_id>/<action_id>/` (the same shape
//! every other executor uses), so the executor only needs to
//! resolve the request's `output_path` *relative* to that bundle.
//!
//! Path-escape guard:
//!   * absolute paths are rejected (`/etc/passwd`)
//!   * `..` components are rejected (`../escape`)
//!   * empty components are rejected
//!
//! The four backends are pure functions in their respective sibling
//! modules — `markdown::write_markdown`, `csv::write_csv_to`,
//! `pdf::render_pdf`, `slides::write_slides`. The executor here is
//! the dispatch + payload-parsing seam.

use crate::error::{DocExecutorError, DocResult};
use crate::{csv, markdown, pdf, slides};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Re-exports of the backend env knobs so the dispatch shim can
/// surface them at audit time without depending on the submodules.
pub use crate::pdf::ENV_CHROME_PATH;

/// Stable list of the four kinds the executor knows about. Surfaced
/// for the dispatch shim's "unknown kind" error message.
pub const DOC_KINDS: &[&str] = &["markdown", "csv", "pdf", "slides"];

/// Parsed action payload. The dispatch shim deserialises
/// `action.payload` straight into this enum (`#[serde(tag = "kind")]`)
/// so the JSON shape is `{ "kind": "pdf", "title": "...",
/// "body": "...", "output_path": "..." }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocRequest {
    /// Direct markdown write — body persisted verbatim.
    Markdown {
        /// Path relative to the per-action bundle (e.g. `report.md`).
        output_path: String,
        /// Markdown body to persist.
        body: String,
    },
    /// CSV writer — header row + per-row fields. RFC 4180 compliant.
    Csv {
        output_path: String,
        header: Vec<String>,
        rows: Vec<CsvRow>,
    },
    /// PDF — markdown rendered through pulldown-cmark + headless
    /// Chrome. Requires Chrome installed on the host (see
    /// [`ENV_CHROME_PATH`] for override).
    Pdf {
        output_path: String,
        title: String,
        body: String,
    },
    /// Slide deck — markdown with `---` separators rendered to
    /// reveal.js-style HTML.
    Slides {
        output_path: String,
        title: String,
        body: String,
    },
}

/// Single CSV row. Wrapped in a struct so serde can deserialise
/// both `[[a, b], [c, d]]` and `[{cols: [a, b]}, {cols: [c, d]}]`
/// shapes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum CsvRow {
    /// Bare-array form: `["v1", "v2"]`.
    Bare(Vec<String>),
    /// Tagged form: `{ "cols": ["v1", "v2"] }`. Useful when the
    /// payload generator wants explicit JSON shape.
    Tagged { cols: Vec<String> },
}

impl CsvRow {
    pub fn cols(self) -> Vec<String> {
        match self {
            CsvRow::Bare(v) => v,
            CsvRow::Tagged { cols } => cols,
        }
    }
}

/// Outcome envelope the dispatch shim serialises into the audit
/// ledger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocOutcome {
    pub kind: String,
    pub output_path: PathBuf,
    pub bytes_written: u64,
}

/// Resolved bundle-relative path. Construction enforces the
/// path-escape guard so callers can't smuggle a forbidden path past
/// it.
#[derive(Debug, Clone)]
pub struct OutputPath {
    absolute: PathBuf,
}

impl OutputPath {
    /// Resolve `relative` against `bundle`. Returns an error if
    /// `relative` is absolute, contains `..`, or has empty
    /// components.
    pub fn resolve(bundle: &Path, relative: &str) -> DocResult<Self> {
        let candidate = Path::new(relative);
        if candidate.is_absolute() {
            return Err(DocExecutorError::PathEscapesSandbox {
                path: relative.to_string(),
                reason: "absolute path not allowed".into(),
            });
        }
        for component in candidate.components() {
            use std::path::Component;
            match component {
                Component::ParentDir => {
                    return Err(DocExecutorError::PathEscapesSandbox {
                        path: relative.to_string(),
                        reason: "parent traversal '..' not allowed".into(),
                    })
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(DocExecutorError::PathEscapesSandbox {
                        path: relative.to_string(),
                        reason: "absolute root not allowed".into(),
                    })
                }
                Component::Normal(s) if s.is_empty() => {
                    return Err(DocExecutorError::PathEscapesSandbox {
                        path: relative.to_string(),
                        reason: "empty component not allowed".into(),
                    })
                }
                _ => {}
            }
        }
        if relative.is_empty() {
            return Err(DocExecutorError::PathEscapesSandbox {
                path: relative.to_string(),
                reason: "empty path not allowed".into(),
            });
        }
        Ok(Self {
            absolute: bundle.join(candidate),
        })
    }

    pub fn as_path(&self) -> &Path {
        &self.absolute
    }
}

/// Stateless dispatch entry point. Holds nothing; built once by
/// the call site and reused per dispatch.
#[derive(Debug, Default, Clone, Copy)]
pub struct DocExecutor;

impl DocExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Run the request against `bundle`. The bundle dir must
    /// already exist (the dispatch shim creates it before
    /// invoking).
    pub fn run(&self, request: &DocRequest, bundle: &Path) -> DocResult<DocOutcome> {
        match request {
            DocRequest::Markdown { output_path, body } => {
                let path = OutputPath::resolve(bundle, output_path)?;
                let n = markdown::write_markdown(path.as_path(), body)?;
                Ok(DocOutcome {
                    kind: "markdown".into(),
                    output_path: path.absolute,
                    bytes_written: n,
                })
            }
            DocRequest::Csv {
                output_path,
                header,
                rows,
            } => {
                let path = OutputPath::resolve(bundle, output_path)?;
                let cols: Vec<Vec<String>> =
                    rows.iter().cloned().map(CsvRow::cols).collect();
                let n = csv::write_csv_to(path.as_path(), header, &cols)?;
                Ok(DocOutcome {
                    kind: "csv".into(),
                    output_path: path.absolute,
                    bytes_written: n,
                })
            }
            DocRequest::Pdf {
                output_path,
                title,
                body,
            } => {
                let path = OutputPath::resolve(bundle, output_path)?;
                let n = pdf::render_pdf(path.as_path(), body, title)?;
                Ok(DocOutcome {
                    kind: "pdf".into(),
                    output_path: path.absolute,
                    bytes_written: n,
                })
            }
            DocRequest::Slides {
                output_path,
                title,
                body,
            } => {
                let path = OutputPath::resolve(bundle, output_path)?;
                let n = slides::write_slides(path.as_path(), body, title)?;
                Ok(DocOutcome {
                    kind: "slides".into(),
                    output_path: path.absolute,
                    bytes_written: n,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_path_resolve_accepts_simple_filename() {
        let bundle = Path::new("/tmp/bundle");
        let p = OutputPath::resolve(bundle, "report.md").unwrap();
        assert_eq!(p.as_path(), Path::new("/tmp/bundle/report.md"));
    }

    #[test]
    fn output_path_resolve_accepts_nested_subdir() {
        let bundle = Path::new("/tmp/bundle");
        let p = OutputPath::resolve(bundle, "reports/q3/summary.md").unwrap();
        assert_eq!(
            p.as_path(),
            Path::new("/tmp/bundle/reports/q3/summary.md")
        );
    }

    #[test]
    fn output_path_resolve_rejects_absolute() {
        let bundle = Path::new("/tmp/bundle");
        let err = OutputPath::resolve(bundle, "/etc/passwd").unwrap_err();
        assert!(matches!(err, DocExecutorError::PathEscapesSandbox { .. }));
    }

    #[test]
    fn output_path_resolve_rejects_parent_traversal() {
        let bundle = Path::new("/tmp/bundle");
        let err = OutputPath::resolve(bundle, "../escape.md").unwrap_err();
        assert!(matches!(err, DocExecutorError::PathEscapesSandbox { .. }));
    }

    #[test]
    fn output_path_resolve_rejects_empty() {
        let bundle = Path::new("/tmp/bundle");
        let err = OutputPath::resolve(bundle, "").unwrap_err();
        assert!(matches!(err, DocExecutorError::PathEscapesSandbox { .. }));
    }

    #[test]
    fn doc_executor_dispatches_markdown() {
        let bundle = tempfile::tempdir().unwrap();
        let exec = DocExecutor::new();
        let req = DocRequest::Markdown {
            output_path: "out.md".into(),
            body: "# Title".into(),
        };
        let outcome = exec.run(&req, bundle.path()).unwrap();
        assert_eq!(outcome.kind, "markdown");
        assert_eq!(outcome.bytes_written, 7);
        assert!(outcome.output_path.exists());
    }

    #[test]
    fn doc_executor_dispatches_csv() {
        let bundle = tempfile::tempdir().unwrap();
        let exec = DocExecutor::new();
        let req = DocRequest::Csv {
            output_path: "data.csv".into(),
            header: vec!["a".into(), "b".into()],
            rows: vec![CsvRow::Bare(vec!["1".into(), "2".into()])],
        };
        let outcome = exec.run(&req, bundle.path()).unwrap();
        assert_eq!(outcome.kind, "csv");
        let body = std::fs::read_to_string(&outcome.output_path).unwrap();
        assert_eq!(body, "a,b\r\n1,2\r\n");
    }

    #[test]
    fn doc_executor_dispatches_slides() {
        let bundle = tempfile::tempdir().unwrap();
        let exec = DocExecutor::new();
        let req = DocRequest::Slides {
            output_path: "deck.html".into(),
            title: "Deck".into(),
            body: "# A\n---\n# B".into(),
        };
        let outcome = exec.run(&req, bundle.path()).unwrap();
        assert_eq!(outcome.kind, "slides");
        let body = std::fs::read_to_string(&outcome.output_path).unwrap();
        assert!(body.contains("<h1>A</h1>"));
        assert!(body.contains("<h1>B</h1>"));
    }

    #[test]
    fn doc_request_round_trips_through_serde_json() {
        let req = DocRequest::Markdown {
            output_path: "x.md".into(),
            body: "y".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["kind"].as_str().unwrap(), "markdown");
        let back: DocRequest = serde_json::from_value(json).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn csv_row_accepts_both_tagged_and_bare_shapes() {
        let bare: CsvRow = serde_json::from_value(serde_json::json!(["a", "b"])).unwrap();
        assert_eq!(bare.cols(), vec!["a".to_string(), "b".to_string()]);
        let tagged: CsvRow =
            serde_json::from_value(serde_json::json!({"cols": ["c", "d"]})).unwrap();
        assert_eq!(tagged.cols(), vec!["c".to_string(), "d".to_string()]);
    }

    #[test]
    fn doc_kinds_lists_all_four_backends() {
        assert_eq!(DOC_KINDS, &["markdown", "csv", "pdf", "slides"]);
    }
}
