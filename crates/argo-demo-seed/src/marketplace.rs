//! Demo-mode marketplace skill catalog — populates the `skill` table
//! the gateway's `/api/v1/skills` route reads from.
//!
//! Sibling to [`crate::skills`] (which writes the `skill_ir` graphs).
//! Demo brief `fix-skills-marketplace-perpetual-loading` requires
//! ≥6 catalog rows under the demo tenant so the /skills marketplace
//! page paints real cards (the canonical "competitor pricing scrape"
//! row anchors the Scene-4 narration).
//!
//! Idempotent: each install is gated on
//! [`argo_skill::SkillStore::find_active_by_name`] so a re-run on a
//! gateway with the seed already applied is a no-op.

use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

use crate::SeedResult;

/// A single catalog entry inserted by the demo marketplace seed.
struct MarketplaceRow {
    name: &'static str,
    description: &'static str,
}

/// The canonical demo marketplace catalog. The leading entry
/// `pricing.competitor_scrape` is the Scene-4 headline skill — the
/// search beat in the v1.0 demo narration drills into it. The rest
/// are workspace-shaped sibling skills that fill the FEATURED panel
/// with a non-trivial list. All install with the
/// `published:verified:argo-seed` provenance so they grade as
/// `PublishedVerified` and the SPA renders the trust-grade chip
/// consistently.
const DEMO_MARKETPLACE_SKILLS: &[MarketplaceRow] = &[
    MarketplaceRow {
        name: "pricing.competitor_scrape",
        description:
            "Scrape competitor product pricing across listed channels and emit a normalized \
             comparison row with confidence + freshness flags.",
    },
    MarketplaceRow {
        name: "approval.route",
        description:
            "Route approvals to the right human based on risk, dollar amount, and seniority rules.",
    },
    MarketplaceRow {
        name: "invoice.extract",
        description:
            "Pull structured line items from PDF or scanned invoices with vendor reconciliation.",
    },
    MarketplaceRow {
        name: "candidate.screen",
        description:
            "First-pass candidate screen against a JD with skill-match scoring and red-flag flags.",
    },
    MarketplaceRow {
        name: "deal.qualify",
        description:
            "BANT-style outbound qualification, calendar-link insertion, CRM update on accept.",
    },
    MarketplaceRow {
        name: "support.triage",
        description:
            "Classify tickets, deflect with KB articles, escalate the rest with full context bundle.",
    },
    MarketplaceRow {
        name: "lead.enrich",
        description:
            "Append firmographic + technographic context to inbound leads from public sources.",
    },
];

/// Insert the demo marketplace catalog under `tenant_id`. Skips any
/// row whose `name` already has an active version (idempotent — safe
/// to call on every gateway boot). Returns the number of NEW rows
/// inserted.
pub async fn seed_demo_marketplace_skills(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
) -> SeedResult<usize> {
    let skills = SkillStore::new(Arc::clone(store));
    let tenant_str = tenant_id.to_string();
    let mut inserted = 0;
    for row in DEMO_MARKETPLACE_SKILLS {
        if skills.find_active_by_name(row.name).await?.is_some() {
            continue;
        }
        let raw = skill_md(row.name, row.description);
        skills.install_for_tenant(&raw, &tenant_str, None).await?;
        inserted += 1;
    }
    Ok(inserted)
}

/// Render a minimal valid SKILL.md document. The body is a one-liner
/// description marker — the gateway catalog endpoint reads the
/// frontmatter for the description, never the body, so a placeholder
/// is fine. Marked `published:verified:argo-seed` so the trust-grade
/// derivation lands on `PublishedVerified`.
fn skill_md(name: &str, description: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: {description:?}\n\
         provenance:\n  source: \"published:verified:argo-seed\"\n  hash: sha256-demo-seed\n\
         ---\nDemo-seed marketplace catalog row for {name}.\n"
    )
}

/// Total number of catalog rows the demo marketplace seed installs.
/// Exposed for tests + the [`crate::SeedReport`] field.
#[must_use]
pub fn demo_marketplace_catalog_size() -> usize {
    DEMO_MARKETPLACE_SKILLS.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionStore;
    use argo_types::TenantId;
    use sqlx::Row;

    async fn open_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        Arc::new(store)
    }

    #[tokio::test]
    async fn seeds_full_catalog_for_tenant() {
        let store = open_store().await;
        let tenant = TenantId::new();
        let inserted = seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        assert_eq!(inserted, demo_marketplace_catalog_size());
        assert!(inserted >= 6, "brief requires >= 6 marketplace skills");
    }

    #[tokio::test]
    async fn second_run_is_noop() {
        let store = open_store().await;
        let tenant = TenantId::new();
        let first = seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        let second = seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        assert_eq!(first, demo_marketplace_catalog_size());
        assert_eq!(second, 0);
    }

    #[tokio::test]
    async fn includes_pricing_competitor_scrape() {
        // Scene-4 narration drills into this specific row.
        let store = open_store().await;
        let tenant = TenantId::new();
        seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        let count: i64 = sqlx::query("SELECT COUNT(*) AS c FROM skill WHERE name = ?")
            .bind("pricing.competitor_scrape")
            .fetch_one(store.pool())
            .await
            .unwrap()
            .get("c");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn rows_are_tenant_scoped() {
        let store = open_store().await;
        let tenant = TenantId::new();
        seed_demo_marketplace_skills(&store, tenant).await.unwrap();
        let count: i64 = sqlx::query("SELECT COUNT(*) AS c FROM skill WHERE tenant_id = ?")
            .bind(tenant.to_string())
            .fetch_one(store.pool())
            .await
            .unwrap()
            .get("c");
        assert_eq!(count, demo_marketplace_catalog_size() as i64);
    }
}
