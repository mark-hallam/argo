//! `GET /api/v1/verticals` — canonical vertical-template catalogue.
//!
//! Previously this endpoint returned a tenant-scoped view derived
//! from `workflow_definition_index`. That made the wizard catalogue
//! empty on cold boot (the operator hadn't installed any verticals
//! yet) and forced `/onboarding/pick-vertical` onto its hardcoded
//! `FALLBACK_VERTICALS` list with the `(offline · N verticals)` hint.
//!
//! The canonical contract is now the same as the legacy `/verticals`
//! route (`routes_verticals.rs::list_response`): every loaded pack
//! the gateway can see, in the canonical `{verticals: [...]}` envelope.
//! Rows carry the full `VerticalRow` shape the SPA already consumes
//! via `web/lib/api/verticals.ts` — `id`, `name`, `version`,
//! `description`, `headline`, `tags`, `workflow_count`, `action_count`,
//! `role_label`, `kpi_count`, `skill_count`.
//!
//! The tenant-scoped "what verticals does this workspace have
//! installed?" view is gone from this endpoint. Today the only
//! caller that depended on the tenant view was the demo-mint
//! onboarding-derivation probe (`web/app/api/auth/demo-mint/route.ts`);
//! its `verticalCount > 0` check now reads the catalogue (always > 0
//! when packs exist), so `needs_onboarding` effectively becomes
//! `workerCount === 0`. That's the load-bearing signal anyway — the
//! demo-seed CLI provisions workers and verticals in lockstep.
use argo_store::SessionStore;
use argo_vertical::{load_dir, LoadedVertical};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

pub(super) async fn list_response(
    _req: &str,
    _store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    list_response_from_dir(&dir)
}

/// Synchronous core split out so tests can pin the verticals
/// directory without mutating the process-global `ARGO_VERTICALS_DIR`
/// env var (which would race across `cargo test`'s default parallel
/// runner).
fn list_response_from_dir(dir: &Path) -> (&'static str, String) {
    let (registry, _errors) = match load_dir(dir) {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json!({"error": format!("verticals load_dir: {e}")}).to_string(),
            );
        }
    };
    let verticals: Vec<_> = registry.list().map(|(id, v)| row(id, v)).collect();
    ("200 OK", json!({ "verticals": verticals }).to_string())
}

/// Project one loaded pack into the `VerticalRow` shape the SPA
/// consumes. Mirrors `routes_verticals.rs::list_row` so the two
/// endpoints stay byte-compatible per row — the only difference is
/// the envelope key (`loaded` for legacy, `verticals` for canonical).
fn row(id: &str, v: &LoadedVertical) -> serde_json::Value {
    let m = &v.manifest;
    json!({
        "id": id,
        "name": m.vertical.name,
        "version": m.vertical.version,
        "description": m.vertical.description,
        "headline": headline(v),
        "tags": m.vertical.tags,
        "workflow_count": m.workflows.as_ref().map(|w| w.files.len()).unwrap_or(0),
        "action_count": m.actions.as_ref().map(|a| a.actions.len()).unwrap_or(0),
        "role_label": m.vertical.name,
        "kpi_count": m.all_kpis().len(),
        "skill_count": skill_count(v),
    })
}

/// First-sentence summary pulled from `vertical.description`. Match
/// the legacy `/verticals` route's helper so card sub-lines render
/// identically across both endpoints.
fn headline(v: &LoadedVertical) -> String {
    let desc = v.manifest.vertical.description.trim();
    if desc.is_empty() {
        return String::new();
    }
    let end = desc
        .char_indices()
        .find(|(_, c)| matches!(c, '.' | '?' | '!'))
        .map(|(i, c)| i + c.len_utf8());
    match end {
        Some(idx) => desc[..idx].to_string(),
        None => desc.to_string(),
    }
}

/// Count `*.skill.md` files in `<root>/skills/`. Matches the helper
/// in `routes_verticals.rs` so the number renders identically on
/// each card across both endpoints.
fn skill_count(v: &LoadedVertical) -> u64 {
    let Ok(read) = std::fs::read_dir(v.root.join("skills")) else {
        return 0;
    };
    let mut count: u64 = 0;
    for entry in read.flatten() {
        if entry.file_name().to_string_lossy().ends_with(".skill.md") {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
#[path = "routes_verticals_list_get_tests.rs"]
mod tests;
