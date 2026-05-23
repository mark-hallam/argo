//! Workplace-knowledge context injection — Wave A brief 5
//! (`mission-knowledge-context-injection`).
//!
//! Renders the per-tenant `workplace_knowledge_doc` rows into the
//! `[WORKPLACE KNOWLEDGE — ...]` block that
//! [`build_system_prompt_with_ab`] splices into every turn-start
//! system message. A process-local cache keyed on
//! `(tenant_id, MAX(version))` keeps the hot path to a single
//! COUNT-style query when the wiki has not changed.
//!
//! Soft-fails on any underlying [`argo_workplace_knowledge::doc`]
//! error: the caller sees `Ok(None)` and the block is omitted, so a
//! transient store hiccup never blocks a turn.
//!
//! Loaded via `#[path = "turn_helpers_workplace.rs"] mod
//! turn_helpers_workplace;` from `turn.rs` to keep that file under
//! the 500-LOC ceiling (HARD RULE 6).

use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use argo_workplace_knowledge::doc::{self, WorkplaceKnowledgeDoc};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

/// Process-local cache entry. Correctness is identical with size = 0;
/// this is a pure performance optimization (see brief Gotchas).
#[derive(Clone)]
struct CacheEntry {
    max_version: i64,
    rendered_block: String,
}

/// Per-process cache. Keyed by tenant-id string. The OnceLock wrapper
/// keeps the static initialization side-effect-free; the inner
/// [`tokio::sync::RwLock`] permits concurrent reads on the hot path.
static WORKPLACE_KNOWLEDGE_CACHE: OnceLock<RwLock<HashMap<String, CacheEntry>>> = OnceLock::new();

fn cache() -> &'static RwLock<HashMap<String, CacheEntry>> {
    WORKPLACE_KNOWLEDGE_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Returns the rendered `[WORKPLACE KNOWLEDGE — ...]` block for the
/// given tenant, or `Ok(None)` when the tenant has no workplace docs
/// (the section is omitted entirely from the system prompt).
///
/// Hot path: one COUNT-style probe via
/// [`doc::max_version_for_tenant`]; the full
/// [`doc::list`] re-fetch only runs on cache miss or when a doc has
/// been written since the last call.
pub(crate) async fn render_workplace_knowledge_block(
    store: &Arc<SessionStore>,
    tenant_id: &TenantId,
) -> ArgoResult<Option<String>> {
    let current_max = doc::max_version_for_tenant(store.pool(), tenant_id).await?;
    if current_max == 0 {
        return Ok(None);
    }

    let tenant_key = tenant_id.to_string();

    {
        let cache_ref = cache().read().await;
        if let Some(entry) = cache_ref.get(&tenant_key) {
            if entry.max_version == current_max {
                return Ok(Some(entry.rendered_block.clone()));
            }
        }
    }

    let docs = doc::list(store.pool(), tenant_id).await?;
    let rendered = render_block(&docs);

    {
        let mut cache_ref = cache().write().await;
        cache_ref.insert(
            tenant_key,
            CacheEntry {
                max_version: current_max,
                rendered_block: rendered.clone(),
            },
        );
    }

    Ok(Some(rendered))
}

/// Render the doc list into the system-prompt block, ordered
/// alphabetically by `category` then `slug` (stable sort →
/// cacheable). Pure function for unit-testing.
pub(crate) fn render_block(docs: &[WorkplaceKnowledgeDoc]) -> String {
    let mut sorted: Vec<&WorkplaceKnowledgeDoc> = docs.iter().collect();
    sorted.sort_by(|a, b| {
        a.category
            .as_db_str()
            .cmp(b.category.as_db_str())
            .then_with(|| a.slug.cmp(&b.slug))
    });

    let mut out = String::from("[WORKPLACE KNOWLEDGE — read-only, do not contradict]\n\n");
    for doc in &sorted {
        out.push_str(&format!(
            "## {} (v{}, last updated {})\n{}\n\n---\n\n",
            doc.title,
            doc.version,
            iso_date_from_micros(doc.updated_at_micros),
            doc.body_markdown.trim_end()
        ));
    }
    out.push_str("[END WORKPLACE KNOWLEDGE]\n");
    out
}

fn iso_date_from_micros(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Test-only escape hatch — drops the process-local cache so unit
/// tests can simulate cold-start conditions without depending on
/// run order.
#[cfg(test)]
pub(crate) async fn clear_cache_for_tests() {
    cache().write().await.clear();
}

#[cfg(test)]
#[path = "turn_helpers_workplace_tests.rs"]
mod tests;
