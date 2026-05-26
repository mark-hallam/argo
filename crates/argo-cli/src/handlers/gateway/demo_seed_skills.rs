//! Demo-mode marketplace skill seed — wires
//! [`argo_demo_seed::seed_demo_marketplace_skills`] into gateway boot
//! when `ARGO_DEMO_MODE=1`.
//!
//! Brief: `fix-skills-marketplace-perpetual-loading`. The /skills SPA
//! page would otherwise paint "0 skills · 0 shown" with a perpetual
//! loading spinner because the demo's `skill` table is empty on a
//! fresh workspace; this seed installs ≥6 marketplace catalog rows
//! under the gateway's cwd-bound workspace tenant so Scene 4 of the
//! v1.0 demo MP4 can record (the camera drills into the
//! `pricing.competitor_scrape` row).
//!
//! Idempotent: each install in the underlying seed is gated on a
//! per-name `find_active_by_name` check, so calling this on every
//! gateway boot is safe.

use argo_store::SessionStore;
use std::sync::Arc;

/// Seed the demo marketplace catalog if `ARGO_DEMO_MODE=1` is set.
/// Logs a one-line stderr summary and emits a tracing event so the
/// operator sees the count in the gateway boot transcript. Never
/// fails the boot — a seed error logs a warning and returns.
pub(super) async fn seed_marketplace_if_demo_mode(store: &Arc<SessionStore>) {
    if !super::demo_mode::is_enabled() {
        return;
    }
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                target: "gateway::demo_seed_skills",
                error = %e,
                "demo marketplace seed skipped: cwd lookup failed",
            );
            return;
        }
    };
    let workspace_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let tenant_id = match store.ensure_workspace(workspace_name, &root).await {
        Ok((tenant, _)) => tenant,
        Err(e) => {
            tracing::warn!(
                target: "gateway::demo_seed_skills",
                error = %e,
                "demo marketplace seed skipped: ensure_workspace failed",
            );
            return;
        }
    };
    match argo_demo_seed::seed_demo_marketplace_skills(store, tenant_id).await {
        Ok(inserted) => {
            let catalog_size = argo_demo_seed::demo_marketplace_catalog_size();
            eprintln!(
                "argo gateway: ARGO_DEMO_MODE=1 marketplace seed — {inserted} new / {catalog_size} total skill catalog rows for tenant {tenant_id}",
            );
            tracing::info!(
                target: "gateway::demo_seed_skills",
                inserted,
                catalog_size,
                tenant_id = %tenant_id,
                "demo marketplace seed complete",
            );
        }
        Err(e) => {
            tracing::warn!(
                target: "gateway::demo_seed_skills",
                error = %e,
                "demo marketplace seed failed; /skills will paint empty",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionStore;

    async fn open_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        Arc::new(store)
    }

    #[tokio::test]
    async fn noop_when_demo_mode_disabled() {
        // `is_enabled()` caches on first read; we can't reliably toggle
        // it inside the test process, so this case relies on the
        // default-unset value. The assertion is that the call returns
        // cleanly and never panics — the underlying seed is not invoked
        // because the env var was never set.
        std::env::remove_var("ARGO_DEMO_MODE");
        let store = open_store().await;
        seed_marketplace_if_demo_mode(&store).await;
    }
}
