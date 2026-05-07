//! End-to-end smoke tests for the document-creation executor.
//! Mirrors `argo-executor-filesystem`'s `tests/`-side coverage:
//! exercises the full `DocExecutor::run` path against tempdir
//! bundles for each backend that doesn't need an external binary.
//!
//! PDF needs Chrome installed; this file only asserts the
//! `ChromeMissing` failure path so the test suite is hermetic. The
//! real Chrome→PDF round-trip is exercised via a separate
//! `cargo test --features chrome` run when an operator wants to
//! validate against their Chrome install (out of scope here).

use argo_executor_document::{CsvRow, DocExecutor, DocExecutorError, DocRequest, ENV_CHROME_PATH};

#[test]
fn smoke_markdown_round_trip_through_executor() {
    let bundle = tempfile::tempdir().unwrap();
    let exec = DocExecutor::new();
    let req = DocRequest::Markdown {
        output_path: "reports/q3.md".into(),
        body: "# Q3\n\n## Numbers\n\n- revenue: $1.2M\n- ARR: $4.8M".into(),
    };
    let outcome = exec.run(&req, bundle.path()).unwrap();
    assert_eq!(outcome.kind, "markdown");
    assert!(outcome.output_path.ends_with("reports/q3.md"));
    let body = std::fs::read_to_string(&outcome.output_path).unwrap();
    assert!(body.contains("# Q3"));
    assert!(body.contains("revenue"));
}

#[test]
fn smoke_csv_round_trip_with_escaping() {
    let bundle = tempfile::tempdir().unwrap();
    let exec = DocExecutor::new();
    let req = DocRequest::Csv {
        output_path: "data/contacts.csv".into(),
        header: vec!["name".into(), "note".into()],
        rows: vec![
            CsvRow::Bare(vec!["Acme, Inc.".into(), "needs follow-up".into()]),
            CsvRow::Tagged {
                cols: vec!["O'Briens".into(), "She said \"yes\"".into()],
            },
        ],
    };
    let outcome = exec.run(&req, bundle.path()).unwrap();
    let body = std::fs::read_to_string(&outcome.output_path).unwrap();
    assert!(body.contains("\"Acme, Inc.\""));
    assert!(body.contains("\"She said \"\"yes\"\"\""));
    assert!(body.starts_with("name,note\r\n"));
}

#[test]
fn smoke_slides_round_trip_renders_three_sections() {
    let bundle = tempfile::tempdir().unwrap();
    let exec = DocExecutor::new();
    let req = DocRequest::Slides {
        output_path: "decks/intro.html".into(),
        title: "Skycot intro".into(),
        body: "# Hello\n\nintro text\n---\n# Why\n\nbecause\n---\n# Demo\n\nlive!".into(),
    };
    let outcome = exec.run(&req, bundle.path()).unwrap();
    let body = std::fs::read_to_string(&outcome.output_path).unwrap();
    assert_eq!(body.matches("<section>").count(), 3);
    assert!(body.contains("<title>Skycot intro</title>"));
    assert!(body.contains("ArrowRight"));
}

#[test]
fn smoke_pdf_returns_chrome_missing_when_binary_absent() {
    let bundle = tempfile::tempdir().unwrap();
    std::env::set_var(ENV_CHROME_PATH, "/no/such/chrome");
    let exec = DocExecutor::new();
    let req = DocRequest::Pdf {
        output_path: "out.pdf".into(),
        title: "Doc".into(),
        body: "# Hello".into(),
    };
    let err = exec.run(&req, bundle.path()).unwrap_err();
    std::env::remove_var(ENV_CHROME_PATH);
    match err {
        DocExecutorError::ChromeMissing { path, .. } => {
            assert_eq!(path.to_str().unwrap(), "/no/such/chrome");
        }
        other => panic!("expected ChromeMissing, got {other}"),
    }
}

#[test]
fn smoke_path_escape_rejected_at_dispatch() {
    let bundle = tempfile::tempdir().unwrap();
    let exec = DocExecutor::new();
    let req = DocRequest::Markdown {
        output_path: "../escape.md".into(),
        body: "x".into(),
    };
    let err = exec.run(&req, bundle.path()).unwrap_err();
    assert!(matches!(err, DocExecutorError::PathEscapesSandbox { .. }));
}
