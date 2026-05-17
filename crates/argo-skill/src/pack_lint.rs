//! Offline lint for vertical-template pack directories
//! (`mission-skill-pack-lint-cli`).
//!
//! Pack authors run this before publishing to surface structural
//! problems that would otherwise only show up at runtime — bad
//! `pack.toml`, unparseable Skill IRs, missing memory schema
//! types, oversized files. The CLI front-end is
//! `argo skill pack lint <path>`.
//!
//! [`lint_pack`] is pure: it reads from disk but performs no
//! network calls and never mutates the pack. The returned
//! [`LintReport`] is the input to the formatter in
//! `argo-cli::handlers::skill_pack_lint`.
//!
//! The check IDs in [`CheckId`] track the brief's 12-row table
//! (`docs/build-plan/streams/mission-skill-pack-lint-cli.md`).
//! Reality-checks (no `worker.toml` in any shipping pack today,
//! eval fixtures stored as a single `golden.json`) are noted on
//! the individual check helpers in `pack_lint_checks` — they
//! fire on what exists, skip what doesn't, never invent errors.
#![allow(clippy::module_name_repetitions)]

use serde::Serialize;
use std::path::Path;

use super::pack_lint_checks;

/// Stable identifiers for each lint check. Stable across versions
/// so JSON consumers can pin behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckId {
    /// `pack.toml` exists, parses, and declares `name`, `version`,
    /// and at least one `[[verticals]]` entry.
    PackToml,
    /// `worker.toml` parses if present. (No shipping pack uses one
    /// today, so missing files are silent.)
    WorkerToml,
    /// `memory/schema.json` parses and every declared field has a
    /// non-empty type string.
    MemorySchema,
    /// Every `skills/*.skill.json` parses as a [`crate::ir::SkillGraph`].
    SkillIrParse,
    /// Tool bindings inside Skill IRs name a recognisable MCP
    /// server / tool. Warning-only — runtime resolves these.
    SkillIrToolRef,
    /// Capability declarations parse to a valid
    /// [`argo_types::CapabilitySet`].
    Capabilities,
    /// Eval fixtures under `evals/` parse to a JSON array.
    EvalParse,
    /// Eval fixtures' `expected_outcome` is a non-empty string and
    /// matches one of the IR-declared outcome names where the IR
    /// declares them.
    EvalShape,
    /// `README.md` exists at pack root.
    ReadmePresent,
    /// No file under the pack exceeds the 500-LOC architecture
    /// invariant.
    FileLocLimit,
    /// Pack name + skill names follow `kebab-case` (lowercase,
    /// hyphens, no spaces).
    KebabCase,
    /// Pack version is a valid semver.
    Semver,
}

impl CheckId {
    /// Stable kebab-case slug — what JSON consumers see and what
    /// the brief's table uses as the row anchor.
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::PackToml => "pack-toml",
            Self::WorkerToml => "worker-toml",
            Self::MemorySchema => "memory-schema",
            Self::SkillIrParse => "skill-ir-parse",
            Self::SkillIrToolRef => "skill-ir-tool-ref",
            Self::Capabilities => "capabilities",
            Self::EvalParse => "eval-parse",
            Self::EvalShape => "eval-shape",
            Self::ReadmePresent => "readme-present",
            Self::FileLocLimit => "file-loc-limit",
            Self::KebabCase => "kebab-case",
            Self::Semver => "semver",
        }
    }
}

/// One finding produced by a check. `Ok` findings are dropped
/// from the report — only errors/warnings flow through.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackLintSeverity {
    /// Hard fail — pack is broken.
    Error,
    /// Soft fail — pack works but the author should look.
    Warning,
}

/// One actionable lint finding.
#[derive(Debug, Clone, Serialize)]
pub struct LintFinding {
    /// Which check raised this finding.
    pub id: CheckId,
    /// `error` or `warning`.
    pub severity: PackLintSeverity,
    /// Operator-facing message; should name the offending file
    /// when applicable.
    pub message: String,
}

/// Aggregate lint outcome — what the CLI formatter renders.
#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    /// Pack root the report describes (display path).
    pub pack: String,
    /// All non-Ok findings, in check-id order.
    pub findings: Vec<LintFinding>,
}

impl LintReport {
    /// Count findings at [`PackLintSeverity::Error`].
    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == PackLintSeverity::Error)
            .count()
    }

    /// Count findings at [`PackLintSeverity::Warning`].
    pub fn warnings(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == PackLintSeverity::Warning)
            .count()
    }
}

/// Lint a vertical-template pack directory.
///
/// The function reads from disk but performs no network calls
/// and does not mutate the pack. All 12 checks always run — a
/// missing file is itself a finding for some checks
/// (e.g. `pack.toml`) and silently skipped for others
/// (e.g. `worker.toml`, which no shipping pack uses today).
///
/// # Example
/// ```no_run
/// use argo_skill::pack_lint::lint_pack;
/// use std::path::Path;
/// let report = lint_pack(Path::new("packs/marketing-intern"));
/// println!("{} errors / {} warnings", report.errors(), report.warnings());
/// ```
pub fn lint_pack(path: &Path) -> LintReport {
    let mut findings: Vec<LintFinding> = Vec::new();
    let pack_display = path.display().to_string();

    let manifest = pack_lint_checks::check_pack_toml(path, &mut findings);
    pack_lint_checks::check_worker_toml(path, &mut findings);
    pack_lint_checks::check_memory_schema(path, &mut findings);
    let outcome_names = pack_lint_checks::check_skill_irs(path, &mut findings);
    pack_lint_checks::check_eval_fixtures(path, &outcome_names, &mut findings);
    pack_lint_checks::check_readme(path, &mut findings);
    pack_lint_checks::check_file_loc(path, &mut findings);

    if let Some(m) = manifest.as_ref() {
        pack_lint_checks::check_kebab_case_pack(&m.pack.name, &mut findings);
        pack_lint_checks::check_semver_pack(&m.pack.version, &mut findings);
    }

    LintReport {
        pack: pack_display,
        findings,
    }
}

/// Append a finding to the per-check sink. Used by every
/// `pack_lint_checks::check_*`; kept in this module so it sees
/// the private `LintFinding` constructor.
pub(super) fn push(
    findings: &mut Vec<LintFinding>,
    id: CheckId,
    severity: PackLintSeverity,
    message: &str,
) {
    findings.push(LintFinding {
        id,
        severity,
        message: message.to_string(),
    });
}

#[cfg(test)]
#[path = "pack_lint_tests.rs"]
mod tests;
