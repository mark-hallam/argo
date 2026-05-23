//! Workplace-knowledge seed docs shipped on `/onboarding/provision`.
//!
//! Content is embedded via `include_str!` so it ships in the binary;
//! operators edit the docs via the gateway after provision. Insertion
//! is per-slug idempotent: if a doc with the same `(tenant, slug)`
//! already exists, it is left untouched — re-running provision on a
//! workspace whose operator has edited the seeds will not overwrite
//! their edits.

use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use argo_workplace_knowledge::doc::{self, KnowledgeCategory};

/// One starter wiki page shipped with every fresh workspace.
pub(super) struct SeedDoc {
    pub slug: &'static str,
    pub title: &'static str,
    pub category: KnowledgeCategory,
    pub body_markdown: &'static str,
}

/// Canonical list of the 5 starter docs, in display order. Order is
/// load-bearing for tests that assert byte-for-byte equality against
/// the on-disk files.
pub(super) fn all_seeds() -> [SeedDoc; 5] {
    [
        SeedDoc {
            slug: "brand-voice",
            title: "Brand voice",
            category: KnowledgeCategory::Brand,
            body_markdown: include_str!("brand_voice.md"),
        },
        SeedDoc {
            slug: "business-rules",
            title: "Business rules",
            category: KnowledgeCategory::BusinessRules,
            body_markdown: include_str!("business_rules.md"),
        },
        SeedDoc {
            slug: "escalation-policy",
            title: "Escalation policy",
            category: KnowledgeCategory::Compliance,
            body_markdown: include_str!("escalation_policy.md"),
        },
        SeedDoc {
            slug: "customer-tone",
            title: "Customer tone",
            category: KnowledgeCategory::Brand,
            body_markdown: include_str!("customer_tone.md"),
        },
        SeedDoc {
            slug: "never-do-this",
            title: "Never do this",
            category: KnowledgeCategory::BusinessRules,
            body_markdown: include_str!("never_do_this.md"),
        },
    ]
}

/// Idempotent seed: walks the 5 starter docs and inserts each only if
/// no doc with that slug exists for the tenant. Returns the slugs that
/// were newly inserted so the caller can emit one audit row per
/// insertion. Re-provisioning a workspace whose seeds are all present
/// is a no-op (returns an empty vec).
pub(super) async fn seed_workplace_knowledge(
    store: &SessionStore,
    tenant_id: &TenantId,
    created_by: &str,
) -> ArgoResult<Vec<&'static str>> {
    let mut inserted = Vec::new();
    for seed in all_seeds() {
        if doc::get(store.pool(), tenant_id, seed.slug).await?.is_some() {
            continue;
        }
        doc::create(
            store.pool(),
            tenant_id,
            seed.slug,
            seed.title,
            seed.category,
            seed.body_markdown,
            created_by,
        )
        .await?;
        inserted.push(seed.slug);
    }
    Ok(inserted)
}
