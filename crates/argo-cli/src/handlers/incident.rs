//! Stream `claude-incident-postmortem-template` — operator walker
//! for blameless postmortems. `argo incident new` either reads a
//! YAML fixture (CI-friendly) or prompts the operator section by
//! section over stdin and writes the rendered markdown to
//! `docs/skycot/incidents/<YYYY-MM-DD>-<slug>.md`.
//!
//! The template lives at `docs/skycot/incidents/_template.md` and
//! is rendered with literal `{{field}}` substitutions — no template
//! engine. Sections that allow multi-line input terminate on a line
//! containing only `.` when prompted interactively.

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Bundled template body, embedded at compile time so tests don't
/// need to walk back to `docs/`.
pub const TEMPLATE_BODY: &str = include_str!(
    "../../../../docs/skycot/incidents/_template.md"
);

/// Maximum slug length, in characters. Matches the brief.
pub const MAX_SLUG_LEN: usize = 60;

/// Required fields for a rendered postmortem. Mirrors the
/// `{{field}}` placeholders in `_template.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct IncidentInput {
    pub summary: String,
    pub severity: String,
    pub timeline: String,
    pub root_cause: String,
    pub contributing_factors: String,
    pub detection: String,
    pub resolution: String,
    pub action_items: String,
    pub lessons_learned: String,
}

/// Today's date as `YYYY-MM-DD` in UTC. Pulled out so tests can
/// inject a fixed date.
pub fn today_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Slugify a summary into a filename-safe stem. Lowercase, ASCII
/// alphanumerics joined by single dashes, capped at `MAX_SLUG_LEN`.
pub fn slugify(summary: &str) -> String {
    let mut out = String::with_capacity(summary.len());
    let mut last_dash = true;
    for ch in summary.chars() {
        if ch.is_ascii_alphanumeric() {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.chars().count() > MAX_SLUG_LEN {
        out = out.chars().take(MAX_SLUG_LEN).collect();
        while out.ends_with('-') {
            out.pop();
        }
    }
    if out.is_empty() {
        out.push_str("incident");
    }
    out
}

/// Render the template with the input fields. Pure function — used
/// by both the file-writer and tests.
pub fn render(template: &str, input: &IncidentInput) -> String {
    template
        .replace("{{summary}}", &input.summary)
        .replace("{{severity}}", &input.severity)
        .replace("{{timeline}}", &input.timeline)
        .replace("{{root_cause}}", &input.root_cause)
        .replace("{{contributing_factors}}", &input.contributing_factors)
        .replace("{{detection}}", &input.detection)
        .replace("{{resolution}}", &input.resolution)
        .replace("{{action_items}}", &input.action_items)
        .replace("{{lessons_learned}}", &input.lessons_learned)
}

/// Validate that no required field is empty after trimming. Returns
/// `Schema(...)` on the first missing field so the operator gets a
/// clear error rather than a silently-broken postmortem.
pub fn validate(input: &IncidentInput) -> ArgoResult<()> {
    let pairs = [
        ("summary", input.summary.as_str()),
        ("severity", input.severity.as_str()),
        ("timeline", input.timeline.as_str()),
        ("root_cause", input.root_cause.as_str()),
        ("contributing_factors", input.contributing_factors.as_str()),
        ("detection", input.detection.as_str()),
        ("resolution", input.resolution.as_str()),
        ("action_items", input.action_items.as_str()),
        ("lessons_learned", input.lessons_learned.as_str()),
    ];
    for (name, value) in pairs {
        if value.trim().is_empty() {
            return Err(ArgoError::Schema(format!(
                "incident field `{name}` is required and may not be empty"
            )));
        }
    }
    if !is_known_severity(&input.severity) {
        return Err(ArgoError::Schema(format!(
            "severity `{}` is not one of SEV1/SEV2/SEV3/SEV4",
            input.severity.trim()
        )));
    }
    Ok(())
}

fn is_known_severity(value: &str) -> bool {
    matches!(value.trim(), "SEV1" | "SEV2" | "SEV3" | "SEV4")
}

/// Compute the destination path for a rendered postmortem, given an
/// output directory and the date stamp. Filename is
/// `YYYY-MM-DD-<slug>.md`.
pub fn output_path(out_dir: &Path, date: &str, summary: &str) -> PathBuf {
    out_dir.join(format!("{date}-{}.md", slugify(summary)))
}

/// Core entrypoint: validate, render, and write. Refuses to
/// overwrite an existing file unless `force` is set; on refusal it
/// returns a `Schema` error mentioning `--force` so the CLI surface
/// can guide the operator without re-implementing the message.
pub fn run_new(
    input: IncidentInput,
    out_dir: &Path,
    force: bool,
) -> ArgoResult<PathBuf> {
    validate(&input)?;
    let date = today_utc();
    let path = output_path(out_dir, &date, &input.summary);
    if path.exists() && !force {
        return Err(ArgoError::Schema(format!(
            "refusing to overwrite existing postmortem at {}; pass --force to replace",
            path.display()
        )));
    }
    fs::create_dir_all(out_dir).map_err(ArgoError::from)?;
    let body = render(TEMPLATE_BODY, &input);
    fs::write(&path, body).map_err(ArgoError::from)?;
    Ok(path)
}

/// Load an `IncidentInput` from a YAML fixture on disk. Used by
/// `--from <path>`.
pub fn load_fixture(path: &Path) -> ArgoResult<IncidentInput> {
    let raw = fs::read_to_string(path).map_err(ArgoError::from)?;
    serde_yaml::from_str(&raw).map_err(|e| {
        ArgoError::Schema(format!("invalid incident YAML at {}: {e}", path.display()))
    })
}

/// Section prompt definitions for the interactive walker. The order
/// matches the template; the bool says whether the section accepts
/// multiline input (terminated by a single `.` line).
const PROMPTS: &[(&str, &str, bool)] = &[
    ("summary", "Headline summary (one line)", false),
    ("severity", "Severity (SEV1 / SEV2 / SEV3 / SEV4)", false),
    (
        "timeline",
        "Timeline — one bullet per event, end with a single '.' line",
        true,
    ),
    ("root_cause", "Root cause — end with a single '.' line", true),
    (
        "contributing_factors",
        "Contributing factors — end with a single '.' line",
        true,
    ),
    ("detection", "Detection — end with a single '.' line", true),
    ("resolution", "Resolution — end with a single '.' line", true),
    (
        "action_items",
        "Action items — end with a single '.' line",
        true,
    ),
    (
        "lessons_learned",
        "Lessons learned — end with a single '.' line",
        true,
    ),
];

/// Interactively prompt for every required section. Reads from
/// `reader`, writes prompts to `writer`. Multi-line sections end on
/// a single line containing only `.`.
pub fn prompt_input<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
) -> ArgoResult<IncidentInput> {
    let mut values = std::collections::HashMap::new();
    for (key, label, multiline) in PROMPTS {
        if *multiline {
            writeln!(writer, "{label}:").map_err(ArgoError::from)?;
        } else {
            write!(writer, "{label}: ").map_err(ArgoError::from)?;
        }
        writer.flush().map_err(ArgoError::from)?;
        let value = if *multiline {
            read_multiline(&mut reader)?
        } else {
            read_line(&mut reader)?
        };
        values.insert(*key, value);
    }
    Ok(IncidentInput {
        summary: take(&mut values, "summary"),
        severity: take(&mut values, "severity"),
        timeline: take(&mut values, "timeline"),
        root_cause: take(&mut values, "root_cause"),
        contributing_factors: take(&mut values, "contributing_factors"),
        detection: take(&mut values, "detection"),
        resolution: take(&mut values, "resolution"),
        action_items: take(&mut values, "action_items"),
        lessons_learned: take(&mut values, "lessons_learned"),
    })
}

fn take(map: &mut std::collections::HashMap<&'static str, String>, key: &'static str) -> String {
    map.remove(key).unwrap_or_default()
}

fn read_line<R: BufRead>(reader: &mut R) -> ArgoResult<String> {
    let mut buf = String::new();
    reader.read_line(&mut buf).map_err(ArgoError::from)?;
    Ok(buf.trim_end_matches(['\r', '\n']).to_string())
}

fn read_multiline<R: BufRead>(reader: &mut R) -> ArgoResult<String> {
    let mut out = String::new();
    loop {
        let mut buf = String::new();
        let n = reader.read_line(&mut buf).map_err(ArgoError::from)?;
        if n == 0 {
            break;
        }
        let line = buf.trim_end_matches(['\r', '\n']);
        if line == "." {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    Ok(out.trim_end_matches('\n').to_string())
}

/// CLI entrypoint wired from `main.rs`. Resolves the input source
/// (YAML fixture or interactive stdin), then delegates to
/// [`run_new`].
pub fn run(from: Option<PathBuf>, out_dir: PathBuf, force: bool) -> ArgoResult<()> {
    let input = match from {
        Some(path) => load_fixture(&path)?,
        None => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            prompt_input(stdin.lock(), stdout.lock())?
        }
    };
    let path = run_new(input, &out_dir, force)?;
    println!("wrote postmortem: {}", path.display());
    Ok(())
}
