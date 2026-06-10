//! Tenant-scoped resolution of Genesis-generated verticals (DEC-025 #9 /
//! Genesis P5 part (b) — tenant-match enforcement, fail-closed).
//!
//! ## The hole this closes
//!
//! Genesis writes a generated teammate to
//! `<verticals_root>/.genesis/<tenant_slug>/<vertical_id>/`. The hire-gate
//! resolver used to scan **every** `.genesis/<tenant_slug>/` segment for the
//! id and return the first match, ignoring tenant. Under single-tenant-per-cwd
//! that is latent, but the moment the gateway goes multi-tenant it is a
//! cross-tenant resolve: tenant B could hire (and the runtime fence could
//! bind a manifest for) a vertical tenant A generated, simply by id.
//!
//! P5 (b) scopes resolution to the caller's own tenant segment ONLY. A
//! vertical that exists for a *different* tenant but not the caller's is
//! treated as not-found — the hire fails closed at the gate, never silently
//! resolving another tenant's teammate.
//!
//! [`sanitize_tenant`] is the single canonical tenant→slug map (the SSE-write
//! path, the hire-gate read path, and the runtime fence all agree on it), so
//! the slug a vertical is written under matches the slug it is read under.

use std::path::{Path, PathBuf};

/// Canonical tenant-id → on-disk slug mapping for the `.genesis/<slug>/`
/// tree. Lowercases, keeps `[a-z0-9-]`, maps everything else to `-`, trims
/// leading/trailing `-`, and falls back to `anonymous` for an empty result.
///
/// This MUST stay byte-identical to the slug the SSE writer
/// (`routes_genesis_stream::resolve_vertical_dir`) writes under — both call
/// this function, so they cannot drift.
///
/// Converged onto the canonical [`argo_vertical::sanitize_tenant`] so the
/// gateway, the run path, and the agent-runner fence all share ONE slug map
/// (rather than three byte-identical copies that could silently diverge).
#[must_use]
pub(super) fn sanitize_tenant(tenant_id: &str) -> String {
    argo_vertical::sanitize_tenant(tenant_id)
}

/// Resolve a Genesis vertical's on-disk directory **scoped to `tenant_id`**:
/// `<verticals_root>/.genesis/<sanitize_tenant(tenant_id)>/<id>/`.
///
/// Returns `Some(dir)` ONLY when the vertical exists under the caller's own
/// tenant segment. A vertical that exists for a different tenant — but not the
/// caller's — resolves to `None` (fail-closed cross-tenant denial). Never
/// scans other tenants' segments.
#[must_use]
pub(super) fn genesis_vertical_dir_for_tenant(
    verticals_root: &Path,
    tenant_id: &str,
    id: &str,
) -> Option<PathBuf> {
    let slug = sanitize_tenant(tenant_id);
    let dir = verticals_root.join(".genesis").join(slug).join(id);
    dir.join("vertical.toml").is_file().then_some(dir)
}

/// `true` when `id` resolves to a Genesis vertical under the caller's OWN
/// tenant segment. Tenant-scoped converse of
/// [`genesis_vertical_dir_for_tenant`].
#[must_use]
pub(super) fn genesis_vertical_exists_for_tenant(
    verticals_root: &Path,
    tenant_id: &str,
    id: &str,
) -> bool {
    genesis_vertical_dir_for_tenant(verticals_root, tenant_id, id).is_some()
}

/// Detect a CROSS-TENANT-ONLY match: the vertical exists for some *other*
/// tenant but NOT for `tenant_id`. Used to distinguish "genuinely unknown id"
/// (a plain 400) from "this id belongs to another tenant" (a security event
/// worth auditing). Scans `.genesis/` only to answer the existence question;
/// it never returns the foreign path.
#[must_use]
pub(super) fn genesis_vertical_exists_for_other_tenant(
    verticals_root: &Path,
    tenant_id: &str,
    id: &str,
) -> bool {
    if genesis_vertical_exists_for_tenant(verticals_root, tenant_id, id) {
        return false; // resolvable for us → not a cross-tenant-only match
    }
    let Ok(entries) = std::fs::read_dir(verticals_root.join(".genesis")) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.path()
            .join(id)
            .join("vertical.toml")
            .is_file()
    })
}

#[cfg(test)]
#[path = "genesis_tenant_tests.rs"]
mod tests;
