//! `argo canonical-qa lint <corpus-dir>` — Track P brief P-04 lint.
//!
//! Walks a corpus directory and validates each entry against four
//! rules:
//!
//! 1. TOML parses + entry has ≥`MIN_PROMPTS_PER_ENTRY` prompts +
//!    non-empty id/answer (enforced by [`argo_skill::CanonicalQaIndex::load`]).
//! 2. Every `declared_claim_class` is a known class in the tenant's
//!    truth-boundary policy directory (cross-checks P-01). Unknown
//!    classes are a *warning* — P-01 documents Allow as the implicit
//!    default for any class not listed in a policy, so this surfaces
//!    typos and stale class names without breaking the lint when a
//!    new corpus entry intentionally declares an Allow-by-default
//!    class (e.g. `competitive-positioning`).
//! 3. `last_reviewed` is within
//!    `CANONICAL_QA_MAX_REVIEW_AGE_DAYS` (warning, not error — the
//!    corpus still loads; the operator should refresh it).
//! 4. No two entries share an exact prompt string (warning — a
//!    duplicate prompt means whichever entry sorts first wins
//!    silently).

use argo_capability::load_policies_from_dir;
use argo_skill::{
    CanonicalQaIndex, CANONICAL_QA_MAX_REVIEW_AGE_DAYS, CANONICAL_QA_MIN_PROMPTS_PER_ENTRY,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use clap::Subcommand;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Subcommands under `argo canonical-qa`.
#[derive(Subcommand, Debug)]
pub enum CanonicalQaCmd {
    /// Lint a canonical-qa corpus directory.
    Lint {
        /// Corpus directory containing one or more `*.toml` files.
        corpus_dir: PathBuf,
        /// Truth-boundary policy directory the corpus's
        /// `declared_claim_classes` must be a subset of. Defaults to
        /// `policies/truth-boundary` relative to the current working
        /// directory.
        #[arg(long, default_value = "policies/truth-boundary")]
        truth_boundary_policies: PathBuf,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
}

/// Dispatch for [`CanonicalQaCmd`].
pub async fn run(cmd: CanonicalQaCmd) -> ArgoResult<()> {
    match cmd {
        CanonicalQaCmd::Lint {
            corpus_dir,
            truth_boundary_policies,
            json,
        } => lint(&corpus_dir, &truth_boundary_policies, json),
    }
}

/// Per-entry finding. `level = "error"` means non-zero exit;
/// `level = "warning"` means surfaced but exit-zero (matches the
/// shell-convention the lint follows).
#[derive(Debug, Clone, serde::Serialize)]
struct Finding {
    entry_id: String,
    level: &'static str,
    message: String,
}

fn lint(corpus_dir: &Path, policies_dir: &Path, json: bool) -> ArgoResult<()> {
    let idx = CanonicalQaIndex::load(corpus_dir, TenantId::new())?;
    let known_classes = collect_known_classes(policies_dir)?;
    let mut findings = Vec::<Finding>::new();
    let today = Utc::now().date_naive();
    let mut prompt_seen: HashMap<String, String> = HashMap::new();
    for entry in idx.entries() {
        for class in &entry.declared_claim_classes {
            if !known_classes.contains(class) {
                findings.push(Finding {
                    entry_id: entry.id.clone(),
                    level: "warning",
                    message: format!(
                        "declared_claim_class {class:?} not present in any policy under {} (treated as Allow-by-default per P-01)",
                        policies_dir.display()
                    ),
                });
            }
        }
        let age = today.signed_duration_since(entry.last_reviewed).num_days();
        if age > CANONICAL_QA_MAX_REVIEW_AGE_DAYS {
            findings.push(Finding {
                entry_id: entry.id.clone(),
                level: "warning",
                message: format!(
                    "last_reviewed {} is {age} days old (max {CANONICAL_QA_MAX_REVIEW_AGE_DAYS})",
                    entry.last_reviewed
                ),
            });
        }
        for prompt in &entry.prompts {
            let key = prompt.to_lowercase();
            if let Some(other_id) = prompt_seen.get(&key) {
                if other_id != &entry.id {
                    findings.push(Finding {
                        entry_id: entry.id.clone(),
                        level: "warning",
                        message: format!(
                            "duplicate prompt {:?} already declared by entry {other_id:?}",
                            prompt
                        ),
                    });
                }
            } else {
                prompt_seen.insert(key, entry.id.clone());
            }
        }
    }
    emit(corpus_dir, idx.len(), &findings, json)?;
    let errors = findings.iter().filter(|f| f.level == "error").count();
    if errors > 0 {
        return Err(ArgoError::Schema(format!(
            "canonical-qa lint failed: {errors} error(s) in {}",
            corpus_dir.display()
        )));
    }
    Ok(())
}

fn collect_known_classes(policies_dir: &Path) -> ArgoResult<HashSet<String>> {
    if !policies_dir.exists() {
        return Err(ArgoError::Io(format!(
            "truth-boundary policies directory does not exist: {}",
            policies_dir.display()
        )));
    }
    let policies = load_policies_from_dir(policies_dir)?;
    let mut classes = HashSet::new();
    for policy in policies {
        for class in policy.claim_classes {
            classes.insert(class.name);
        }
    }
    Ok(classes)
}

fn emit(
    corpus_dir: &Path,
    entry_count: usize,
    findings: &[Finding],
    json: bool,
) -> ArgoResult<()> {
    let errors = findings.iter().filter(|f| f.level == "error").count();
    let warnings = findings.iter().filter(|f| f.level == "warning").count();
    if json {
        let payload = json!({
            "corpus_dir": corpus_dir.display().to_string(),
            "entry_count": entry_count,
            "min_prompts_per_entry": CANONICAL_QA_MIN_PROMPTS_PER_ENTRY,
            "errors": errors,
            "warnings": warnings,
            "findings": findings,
        });
        println!("{payload}");
    } else {
        println!(
            "canonical-qa lint {}: {entry_count} entries, {errors} error(s), {warnings} warning(s)",
            corpus_dir.display()
        );
        for f in findings {
            println!("  [{}] {}: {}", f.level, f.entry_id, f.message);
        }
    }
    Ok(())
}
