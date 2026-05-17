//! YAML parser, validator, and directory loader for declarative
//! policy files (Stream `claude-policy-as-code`). Sibling of
//! [`super`] so `policy.rs` stays under the per-file LOC budget;
//! re-exported via `pub use` so callers see the same public API as
//! before the split.

use super::{PolicyError, PolicyFile};
use std::path::Path;

/// Parse a YAML body into a [`PolicyFile`]. Used by both the CLI
/// loader and the gate's runtime loader. `name` is the file
/// basename (e.g. `finance-strict` for `finance-strict.yaml`).
pub fn parse_policy_yaml(name: &str, body: &str) -> Result<PolicyFile, PolicyError> {
    let mut file: PolicyFile = serde_yaml::from_str(body).map_err(|e| PolicyError::Parse {
        name: name.to_string(),
        message: e.to_string(),
    })?;
    file.name = name.to_string();
    validate_policy_file(&file)?;
    Ok(file)
}

/// Validate a parsed policy file. Catches problems the type
/// system can't (empty applies_to, wildcard-only rules, etc.).
pub fn validate_policy_file(file: &PolicyFile) -> Result<(), PolicyError> {
    if file.applies_to.is_empty() {
        return Err(PolicyError::Validation {
            name: file.name.clone(),
            detail: "applies_to is empty (must select at least one role or tenant)".into(),
        });
    }
    if file.rules.is_empty() {
        return Err(PolicyError::Validation {
            name: file.name.clone(),
            detail: "rules is empty (must declare at least one rule)".into(),
        });
    }
    for (i, rule) in file.rules.iter().enumerate() {
        if rule.action_kind.is_empty() {
            return Err(PolicyError::Validation {
                name: file.name.clone(),
                detail: format!("rules[{i}].action_kind is empty"),
            });
        }
        if let Some(qh) = &rule.quiet_hours {
            qh.parse_times()?;
        }
        if let Some(cap) = rule.spend_cap_per_action_usd {
            if !cap.is_finite() || cap < 0.0 {
                return Err(PolicyError::Validation {
                    name: file.name.clone(),
                    detail: format!("rules[{i}].spend_cap_per_action_usd must be >= 0; got {cap}"),
                });
            }
        }
    }
    Ok(())
}

/// Load every `*.yaml` policy file under `<dir>`, validating each
/// one. Errors short-circuit so a misconfigured workspace never
/// silently degrades. Files in unspecified order — caller passes
/// the result to [`super::resolve_effective_policy`] which preserves
/// order during chain walk.
pub fn load_policy_dir(dir: &Path) -> Result<Vec<PolicyFile>, PolicyError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(dir).map_err(|e| PolicyError::Io {
        path: dir.display().to_string(),
        message: e.to_string(),
    })?;
    let mut paths: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("yaml"))
        .collect();
    paths.sort(); // stable order so resolver behaviour is repeatable
    let mut out = Vec::new();
    for p in paths {
        let body = std::fs::read_to_string(&p).map_err(|e| PolicyError::Io {
            path: p.display().to_string(),
            message: e.to_string(),
        })?;
        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("policy")
            .to_string();
        out.push(parse_policy_yaml(&name, &body)?);
    }
    Ok(out)
}
