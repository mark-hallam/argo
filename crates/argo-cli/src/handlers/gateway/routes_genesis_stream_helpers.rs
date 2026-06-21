//! Pure helpers split out of routes_genesis_stream.rs (LOC ceiling). No behaviour change.

use super::routes_genesis_stream::{PROVIDER_KEY_ENV, PROVIDER_KEY_ALIASES};
use argo_genesis::name_to_persona_slug;
use std::path::{Path, PathBuf};

pub(super) fn provider_key_present() -> bool {
    let non_empty = |key: &str| {
        std::env::var(key)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    };
    non_empty(PROVIDER_KEY_ENV) || PROVIDER_KEY_ALIASES.iter().any(|k| non_empty(k))
}

/// Resolve the persona slug for the run: the explicit `persona` body
/// field wins; else a `Persona: <Name>` slot embedded in the role text
/// (the intake conversation's persona slot rides inside the SPA's
/// composed `role_description` — day-1 fix 4: ignoring it minted a
/// teammate named after the charter's first word, "Read"); else the
/// first-word fallback.
pub(super) fn persona_for(explicit: Option<&str>, role_description: &str) -> String {
    let raw = explicit
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| argo_genesis::persona_name_from_role_text(role_description))
        .unwrap_or_else(|| derive_persona(role_description));
    name_to_persona_slug(&raw)
}

pub(super) fn derive_persona(role_description: &str) -> String {
    // First "wordy" token (>=3 chars), capped at 16; "operator" on noise.
    let candidate = role_description
        .split(|c: char| !c.is_alphanumeric())
        .find(|s| s.len() >= 3)
        .unwrap_or("operator");
    let trimmed: String = candidate.chars().take(16).collect();
    if trimmed.is_empty() {
        "operator".to_string()
    } else {
        trimmed
    }
}

/// Canonical tenant→slug map for the `.genesis/<slug>/` tree. Delegates to
/// the shared [`super::genesis_tenant::sanitize_tenant`] so the slug this
/// writer writes under matches the slug the hire gate + runtime fence read
/// under (P5 (b) tenant-match enforcement — no parallel definition).
pub(super) fn sanitize_tenant(tenant_id: &str) -> String {
    super::genesis_tenant::sanitize_tenant(tenant_id)
}

pub(super) fn resolve_vertical_dir(tenant_slug: &str, vertical_id: &str) -> Result<PathBuf, String> {
    let root = argo_vertical::default_dir();
    let tenant_dir = root.join(".genesis").join(tenant_slug);
    std::fs::create_dir_all(&tenant_dir)
        .map_err(|e| format!("create tenant dir {}: {e}", tenant_dir.display()))?;
    Ok(tenant_dir.join(vertical_id))
}

pub(super) fn count_eval_seeds(vertical_dir: &Path) -> usize {
    let path = vertical_dir.join("eval-seed.toml");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return 0;
    };
    contents.matches("[[seed]]").count()
}
