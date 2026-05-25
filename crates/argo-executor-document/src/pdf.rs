//! PDF backend — markdown → HTML → PDF via headless Chrome.
//!
//! Mirrors the `docs/build-plan/build-pdf/build.sh` toolchain we
//! already use for `PROTOCOL_ARCHITECTURE.pdf`: pulldown-cmark
//! produces the HTML, headless Chrome prints the PDF.
//!
//! Dependency posture: Chrome is shelled out to as a child process,
//! the path resolved from `ARGO_DOC_CHROME` (override) or the
//! standard `/Applications/Google Chrome.app/...` macOS location.
//! Linux operators set `ARGO_DOC_CHROME=/usr/bin/google-chrome`.
//!
//! When Chrome is missing, [`render_pdf`] returns
//! [`DocExecutorError::ChromeMissing`] with a clear actionable
//! message. The dispatch shim surfaces this to the audit ledger so
//! the operator sees the gap rather than a silent fail.

use crate::error::{DocExecutorError, DocResult};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Env var the operator sets to nominate a non-default Chrome path.
pub const ENV_CHROME_PATH: &str = "ARGO_DOC_CHROME";

/// Default macOS path. Linux operators must set the env override.
const DEFAULT_CHROME_PATH: &str = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

/// Convert markdown to standalone HTML the headless renderer can
/// print. Bundles a print stylesheet so PDF output looks polished
/// without a CDN fetch.
pub fn markdown_to_html(markdown: &str, title: &str) -> String {
    let mut html_body = String::with_capacity(markdown.len() * 2);
    let parser = pulldown_cmark::Parser::new(markdown);
    pulldown_cmark::html::push_html(&mut html_body, parser);
    let safe_title = html_escape(title);
    format!(
        "<!doctype html>
<html><head><meta charset=\"utf-8\"><title>{safe_title}</title>
<style>
@page {{ size: A4; margin: 18mm; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; line-height: 1.5; color: #1a1a1a; }}
h1 {{ font-size: 22pt; margin-top: 0; }}
h2 {{ font-size: 16pt; margin-top: 18pt; }}
h3 {{ font-size: 13pt; }}
code {{ font-family: 'SF Mono', Menlo, Consolas, monospace; background: #f4f4f4; padding: 1pt 3pt; border-radius: 3pt; }}
pre {{ background: #f4f4f4; padding: 8pt; border-radius: 4pt; overflow-x: auto; }}
pre code {{ background: transparent; padding: 0; }}
table {{ border-collapse: collapse; margin: 12pt 0; }}
th, td {{ border: 1px solid #ddd; padding: 4pt 8pt; text-align: left; }}
th {{ background: #f4f4f4; }}
blockquote {{ border-left: 3pt solid #ccc; margin: 12pt 0; padding-left: 12pt; color: #555; }}
img {{ max-width: 100%; }}
</style></head><body>
{html_body}
</body></html>"
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Resolve which Chrome binary to invoke. `ARGO_DOC_CHROME`
/// override wins, then the macOS default.
pub fn resolve_chrome_path() -> PathBuf {
    if let Ok(override_path) = std::env::var(ENV_CHROME_PATH) {
        return PathBuf::from(override_path);
    }
    PathBuf::from(DEFAULT_CHROME_PATH)
}

/// Render markdown to a PDF at `target`. The intermediate HTML is
/// written to `<bundle>/.staging/render.html` so Chrome has a stable
/// file URL to print from, then removed on successful render so a
/// plaintext duplicate of the generated document is not left on
/// disk. A failed render preserves the file so operators can inspect
/// what Chrome was asked to print.
pub fn render_pdf(target: &Path, markdown: &str, title: &str) -> DocResult<u64> {
    render_pdf_with(target, markdown, title, invoke_chrome)
}

/// Testable seam behind [`render_pdf`]. The closure stands in for
/// the headless Chrome invocation so the staging-cleanup contract
/// can be exercised without a real Chrome on PATH.
fn render_pdf_with<F>(target: &Path, markdown: &str, title: &str, invoke: F) -> DocResult<u64>
where
    F: FnOnce(&Path, &Path) -> DocResult<()>,
{
    let html = markdown_to_html(markdown, title);
    let staging = match target.parent() {
        Some(p) => p.join(".staging"),
        None => PathBuf::from(".staging"),
    };
    std::fs::create_dir_all(&staging)
        .map_err(|e| DocExecutorError::io(format!("pdf: stage dir {}", staging.display()), e))?;
    let html_path = staging.join("render.html");
    std::fs::write(&html_path, html.as_bytes())
        .map_err(|e| DocExecutorError::io(format!("pdf: write {}", html_path.display()), e))?;
    // Chrome failure short-circuits via `?`, which preserves the
    // staging HTML for post-mortem inspection. The cleanup below
    // runs only when Chrome reported success.
    invoke(&html_path, target)?;
    let bytes = std::fs::metadata(target)
        .map(|m| m.len())
        .map_err(|e| DocExecutorError::io(format!("pdf: stat output {}", target.display()), e))?;
    cleanup_staging_html(&html_path, &staging)?;
    Ok(bytes)
}

/// Remove the staging HTML after a successful render. Surfaces any
/// cleanup failure as an executor error rather than silently leaving
/// the plaintext on disk — the privacy invariant (no duplicate of
/// the generated document) is the whole reason the cleanup exists,
/// so an audit trail when it fails beats a quiet leak.
fn cleanup_staging_html(html_path: &Path, staging: &Path) -> DocResult<()> {
    std::fs::remove_file(html_path)
        .map_err(|e| DocExecutorError::io(format!("pdf: cleanup {}", html_path.display()), e))?;
    // Drop the now-empty `.staging` dir as a best-effort tidy-up.
    // We do not bubble errors here: a concurrent writer may have
    // placed another file in the dir, and the render.html removal
    // is the load-bearing invariant.
    let _ = std::fs::remove_dir(staging);
    Ok(())
}

fn invoke_chrome(html_path: &Path, pdf_path: &Path) -> DocResult<()> {
    let chrome = resolve_chrome_path();
    if !chrome.exists() {
        return Err(DocExecutorError::ChromeMissing {
            path: chrome,
            env: ENV_CHROME_PATH,
            hint: "macOS: brew install --cask google-chrome; \
                   linux: install google-chrome-stable",
        });
    }
    let html_uri = format!("file://{}", html_path.display());
    let print_arg = format!("--print-to-pdf={}", pdf_path.display());
    let output = Command::new(&chrome)
        .arg("--headless=new")
        .arg("--no-sandbox")
        .arg("--disable-gpu")
        .arg("--no-pdf-header-footer")
        .arg(&print_arg)
        .arg(&html_uri)
        .output()
        .map_err(|e| DocExecutorError::io("pdf: spawn chrome", e))?;
    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(DocExecutorError::ChromeFailed { code, stderr });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_to_html_renders_heading() {
        let html = markdown_to_html("# Hello\n\nworld", "Doc");
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<title>Doc</title>"));
    }

    #[test]
    fn markdown_to_html_escapes_title_characters() {
        let html = markdown_to_html("body", "<script>");
        assert!(html.contains("<title>&lt;script&gt;</title>"));
    }

    #[test]
    fn render_pdf_returns_chrome_missing_error_when_path_invalid() {
        let _g = EnvGuard::set(ENV_CHROME_PATH, "/nonexistent/chrome-binary");
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.pdf");
        let err = render_pdf(&target, "# H", "T").unwrap_err();
        assert!(
            matches!(err, DocExecutorError::ChromeMissing { .. }),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn render_pdf_removes_staging_html_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.pdf");
        let staging_html = dir.path().join(".staging").join("render.html");
        let bytes = render_pdf_with(&target, "# Body", "T", |html_path, pdf_path| {
            assert!(
                html_path.exists(),
                "fake-chrome should see the staging html before render"
            );
            std::fs::write(pdf_path, b"%PDF-stub").unwrap();
            Ok(())
        })
        .unwrap();
        assert!(bytes > 0);
        assert!(
            !staging_html.exists(),
            "staging render.html must be removed after a successful render"
        );
        assert!(
            !staging_html.parent().unwrap().exists(),
            "now-empty .staging dir should be tidied up too"
        );
    }

    #[test]
    fn render_pdf_preserves_staging_html_when_chrome_fails() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.pdf");
        let staging_html = dir.path().join(".staging").join("render.html");
        let err = render_pdf_with(&target, "# Body", "T", |_html, _pdf| {
            Err(DocExecutorError::ChromeFailed {
                code: 1,
                stderr: "synthetic".into(),
            })
        })
        .unwrap_err();
        assert!(matches!(err, DocExecutorError::ChromeFailed { .. }));
        assert!(
            staging_html.exists(),
            "failed renders must preserve the staging html for post-mortem"
        );
        let content = std::fs::read_to_string(&staging_html).unwrap();
        assert!(content.contains("<h1>Body</h1>"));
    }

    #[test]
    fn render_pdf_surfaces_cleanup_failure_as_io_error() {
        // Simulate cleanup-time failure by having the fake "chrome"
        // delete the staging file out from under us. The subsequent
        // remove_file inside render_pdf_with hits NotFound and the
        // function must surface that as DocExecutorError::Io rather
        // than silently returning Ok.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.pdf");
        let err = render_pdf_with(&target, "# Body", "T", |html_path, pdf_path| {
            std::fs::write(pdf_path, b"%PDF-stub").unwrap();
            std::fs::remove_file(html_path).unwrap();
            Ok(())
        })
        .unwrap_err();
        match err {
            DocExecutorError::Io { context, .. } => {
                assert!(
                    context.starts_with("pdf: cleanup"),
                    "unexpected io context: {context}"
                );
            }
            other => panic!("expected Io cleanup error, got {other}"),
        }
    }

    #[test]
    fn resolve_chrome_path_honors_env_override() {
        let _g = EnvGuard::set(ENV_CHROME_PATH, "/custom/chrome");
        assert_eq!(resolve_chrome_path(), PathBuf::from("/custom/chrome"));
    }

    /// Restore an env var on drop so tests don't leak state into
    /// each other.
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
