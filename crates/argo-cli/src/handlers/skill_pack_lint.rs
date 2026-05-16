//! Handler for `argo skill pack lint` — formats the
//! [`argo_skill::pack_lint::LintReport`] produced by the pure
//! linter into text or JSON output and maps severity counts to
//! exit codes. Brief: `mission-skill-pack-lint-cli`.

use argo_skill::pack_lint::{lint_pack, LintReport, Severity};
use argo_types::{ArgoError, ArgoResult};
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process;

/// Run `argo skill pack lint <path> [--strict] [--json]`.
///
/// Exit codes (via [`std::process::exit`] after the formatted
/// output is flushed):
/// * `0` — no findings
/// * `1` — warnings only, no `--strict`
/// * `2` — at least one error, or `--strict` with warnings
pub fn run(path: &str, strict: bool, json: bool) -> ArgoResult<()> {
    let mut stdout = io::stdout().lock();
    let use_color = io::stdout().is_terminal();
    let exit = render(&mut stdout, Path::new(path), strict, json, use_color)?;
    let _ = stdout.flush();
    drop(stdout);
    process::exit(exit);
}

/// Same as [`run`] but writes to a caller-supplied `Write` and
/// returns the exit code instead of calling [`std::process::exit`].
/// Exposed so the handler tests can assert formatter behaviour
/// without terminating the test runner.
pub fn render<W: Write>(
    w: &mut W,
    path: &Path,
    strict: bool,
    json: bool,
    use_color: bool,
) -> ArgoResult<i32> {
    let report = lint_pack(path);
    if json {
        emit_json(w, &report)?;
    } else {
        emit_text(w, &report, use_color)?;
    }
    Ok(classify(&report, strict))
}

fn classify(report: &LintReport, strict: bool) -> i32 {
    if report.errors() > 0 {
        2
    } else if report.warnings() > 0 {
        if strict {
            2
        } else {
            1
        }
    } else {
        0
    }
}

fn emit_json<W: Write>(w: &mut W, report: &LintReport) -> ArgoResult<()> {
    let body = serde_json::json!({
        "pack": report.pack,
        "checks": report.findings.iter().map(|f| serde_json::json!({
            "id": f.id.as_slug(),
            "severity": match f.severity { Severity::Error => "error", Severity::Warning => "warning" },
            "message": f.message,
        })).collect::<Vec<_>>(),
        "summary": {
            "errors": report.errors(),
            "warnings": report.warnings(),
        }
    });
    writeln!(
        w,
        "{}",
        serde_json::to_string_pretty(&body)
            .map_err(|e| ArgoError::Provider(format!("lint json: {e}")))?
    )
    .map_err(|e| ArgoError::Io(format!("write: {e}")))?;
    Ok(())
}

fn emit_text<W: Write>(w: &mut W, report: &LintReport, color: bool) -> ArgoResult<()> {
    writeln!(w, "{}/", report.pack).map_err(io_err)?;
    let errors = report.errors();
    let warnings = report.warnings();
    if errors == 0 && warnings == 0 {
        writeln!(w, "  {} all checks passed", paint("OK", "32", color)).map_err(io_err)?;
        return Ok(());
    }
    for finding in &report.findings {
        let (tag, color_code) = match finding.severity {
            Severity::Error => ("ERROR", "31"),
            Severity::Warning => ("WARN ", "33"),
        };
        writeln!(
            w,
            "  {} [{}] {}",
            paint(tag, color_code, color),
            finding.id.as_slug(),
            finding.message,
        )
        .map_err(io_err)?;
    }
    writeln!(w, "  summary: {errors} error(s), {warnings} warning(s)").map_err(io_err)?;
    Ok(())
}

fn paint(text: &str, code: &str, color: bool) -> String {
    if color {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn io_err(e: io::Error) -> ArgoError {
    ArgoError::Io(format!("write: {e}"))
}

#[cfg(test)]
#[path = "skill_pack_lint_tests.rs"]
mod tests;
