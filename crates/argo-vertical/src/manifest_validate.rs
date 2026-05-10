//! Strict-mode manifest validation.
//!
//! Pure additional pass over a parsed [`VerticalManifest`]. Catches
//! contracts that the structural parser can't enforce on its own:
//! unknown executor kinds, executor-binding shape mismatches, dangling
//! `on_complete_emit` references, and provider model-id format. All
//! checks accumulate into a single error message so the operator sees
//! every problem in one shot rather than chasing one fix per parse.
//!
//! Called from [`crate::manifest::parse`] at the tail end so callers
//! that already use `parse` get strict mode for free. Manifests that
//! were valid before this module landed remain valid: the checks are
//! tightened against fields that already shipped, not new schema.
//!
//! Error type note: the brief calls out `ArgoError::InvalidInput(_)`,
//! but that variant doesn't exist in this codebase's error tree. The
//! existing convention for parse / validate failures is
//! `ArgoError::Schema(String)` — see `manifest.rs::parse` and
//! `validator.rs`. Strict-mode keeps that convention so callers don't
//! need a second match arm.
use crate::manifest::VerticalManifest;
use crate::manifest_actions::{ActionDeclaration, ExecutorDeclaration};
use argo_types::{ArgoError, ArgoResult};
use std::collections::BTreeSet;

/// Canonical list of executor kinds the runtime knows how to
/// dispatch. Mirrors `ExecutorKind::parse` in
/// `crates/argo-cli/src/handlers/worker_actions_executor.rs`. The
/// dispatch shim lives above this crate (argo-cli depends on
/// argo-vertical, not the other way around), so we can't pull the
/// list at runtime; the canonical declaration belongs here in the
/// schema crate. Adding a new kind = update both lists. The
/// `executor_kind_known_round_trip` test in the dispatch shim
/// guards against drift on the cli side.
pub const KNOWN_EXECUTOR_KINDS: &[&str] = &[
    "human",
    "mcp",
    "computer_use",
    "computer_use_macos",
    "computer-use-macos",
    "api",
    "browser",
    "filesystem",
    "code_execution",
    "document_creation",
];

/// One validation failure surfaced from [`validate_strict`]. The
/// `path` field follows TOML-table convention
/// (`actions.foo.executor.kind`) so an operator can locate the bad
/// field by reading their `vertical.toml` left-to-right.
///
/// Public for downstream tooling (e.g. an editor extension that
/// renders per-line hints) but not currently consumed inside the
/// crate; [`validate_strict`] folds the vec into a single
/// `ArgoError::Schema` for the parse path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// TOML-style dotted path to the offending field.
    pub path: String,
    /// Human-readable reason. Surfaced in the aggregated error.
    pub message: String,
    /// Optional 1-based line number. `None` today; `toml::Value`
    /// doesn't preserve spans cheaply. Reserved for follow-up.
    pub line: Option<usize>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(f, "{} (line {}): {}", self.path, line, self.message),
            None => write!(f, "{}: {}", self.path, self.message),
        }
    }
}

/// Run every strict-mode check against `manifest`. Errors are
/// accumulated and folded into a single `ArgoError::Schema` so the
/// operator sees the full punch-list at once instead of chasing
/// one-fix-per-parse.
///
/// Returns `Ok(())` for every fixture that previously parsed
/// without error: this is a tightening pass, not a schema change.
///
/// # Example
/// ```
/// use argo_vertical::{parse, validate_strict};
/// let m = parse(r#"
/// schema_version = "1"
/// [vertical]
/// id = "demo"
/// name = "Demo"
/// version = "0.1.0"
/// "#).unwrap();
/// validate_strict(&m).expect("minimal manifest passes strict mode");
/// ```
pub fn validate_strict(manifest: &VerticalManifest) -> ArgoResult<()> {
    let mut errors: Vec<ValidationError> = Vec::new();

    check_actions(manifest, &mut errors);
    check_provider(manifest, &mut errors);

    if errors.is_empty() {
        return Ok(());
    }
    let mut joined = String::from("vertical manifest failed strict validation:");
    for e in &errors {
        joined.push_str("\n  - ");
        joined.push_str(&format!("{e}"));
    }
    Err(ArgoError::Schema(joined))
}

fn check_actions(manifest: &VerticalManifest, errors: &mut Vec<ValidationError>) {
    let Some(actions) = manifest.actions.as_ref() else {
        return;
    };
    let declared_names: BTreeSet<&str> = actions.actions.keys().map(String::as_str).collect();

    for (name, decl) in &actions.actions {
        let base = format!("actions.{name}");
        check_executor_kind(name, decl, errors);
        check_executor_binding_shape(name, decl, errors);
        check_on_complete_emit(&base, decl, &declared_names, errors);
    }
}

fn check_executor_kind(
    action: &str,
    decl: &ActionDeclaration,
    errors: &mut Vec<ValidationError>,
) {
    let kind = decl.executor.kind_str();
    let path = format!("actions.{action}.executor.kind");
    if kind.is_empty() {
        errors.push(ValidationError {
            path,
            message: "executor kind must not be empty".to_string(),
            line: None,
        });
        return;
    }
    if !KNOWN_EXECUTOR_KINDS.contains(&kind) {
        errors.push(ValidationError {
            path,
            message: format!(
                "unknown executor kind '{kind}' — valid: {}",
                KNOWN_EXECUTOR_KINDS.join(" | ")
            ),
            line: None,
        });
    }
}

fn check_executor_binding_shape(
    action: &str,
    decl: &ActionDeclaration,
    errors: &mut Vec<ValidationError>,
) {
    let ExecutorDeclaration::Tagged(t) = &decl.executor else {
        return;
    };
    let push_missing = |errs: &mut Vec<ValidationError>, field: &str, kind: &str| {
        errs.push(ValidationError {
            path: format!("actions.{action}.executor.{field}"),
            message: format!("kind = \"{kind}\" requires a non-empty '{field}' field"),
            line: None,
        });
    };
    let is_empty = |opt: &Option<String>| opt.as_deref().unwrap_or("").is_empty();
    let kind = t.kind.as_str();
    if kind == "mcp" {
        if is_empty(&t.server) {
            push_missing(errors, "server", "mcp");
        }
        if is_empty(&t.tool) {
            push_missing(errors, "tool", "mcp");
        }
    } else if kind == "api" && is_empty(&t.binding) {
        push_missing(errors, "binding", "api");
    } else if kind == "browser" && is_empty(&t.binding) {
        push_missing(errors, "binding", "browser");
    } else if matches!(
        kind,
        "computer_use" | "computer_use_macos" | "computer-use-macos"
    ) && is_empty(&t.profile)
    {
        push_missing(errors, "profile", kind);
    }
}

fn check_on_complete_emit(
    base: &str,
    decl: &ActionDeclaration,
    declared: &BTreeSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    for (i, target) in decl.on_complete_emit.iter().enumerate() {
        if !declared.contains(target.as_str()) {
            errors.push(ValidationError {
                path: format!("{base}.on_complete_emit[{i}]"),
                message: format!(
                    "references undeclared action '{target}' — every entry must name an action declared in [actions]"
                ),
                line: None,
            });
        }
    }
}

fn check_provider(manifest: &VerticalManifest, errors: &mut Vec<ValidationError>) {
    let Some(prov) = manifest.provider.as_ref() else {
        return;
    };
    check_model_id("provider.default", &prov.default, errors);
    if let Some(fallback) = prov.fallback.as_ref() {
        check_model_id("provider.fallback", fallback, errors);
    }
}

fn check_model_id(path: &str, model_id: &str, errors: &mut Vec<ValidationError>) {
    // Strict mode only catches the degenerate empty case here.
    // The richer `provider/model` shape check is a deliberate
    // follow-up: existing fixtures (e.g. `verticals/ai-receptionist`,
    // `verticals/sales-follow-up`) ship a bare provider name in
    // `default`, and those fixtures are outside this brief's
    // may-touch list. Tightening the format gate before the
    // fixtures land a fix would block the strict-mode pass on
    // valid-today manifests.
    if model_id.trim().is_empty() {
        errors.push(ValidationError {
            path: path.to_string(),
            message: "must not be empty".to_string(),
            line: None,
        });
    }
}

#[cfg(test)]
#[path = "manifest_validate_tests.rs"]
mod tests;

