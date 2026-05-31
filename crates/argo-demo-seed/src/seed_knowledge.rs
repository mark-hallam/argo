//! Workplace-knowledge (handbook) seeder for the canonical v1 demo.
//!
//! The `/settings/knowledge` surface (`routes_knowledge.rs` →
//! `GET /api/v1/knowledge`) reads the tenant-scoped wiki substrate in
//! `argo-workplace-knowledge`. Before this seeder the demo tenant had
//! zero `workplace_knowledge_doc` rows, so the page rendered an empty
//! state while the endpoint + CRUD were fully live — a NO-THEATER gap
//! (real endpoint, no data).
//!
//! This module inserts a small set of **real** handbook docs via the
//! canonical `argo_workplace_knowledge::doc::create` writer (the same
//! path the gateway's POST handler uses), so the rows the page lists
//! are genuine wiki head rows the agents' context-injection hot path
//! also reads. The docs are written to match the demo workspace's
//! actual operating context — the marketplace / recruiting / support
//! work the seeded teammates do — so a doc opened on camera reads as a
//! real company handbook, not lorem-ipsum decoration.
//!
//! Idempotent: each doc is keyed `UNIQUE(tenant_id, slug)`; a doc whose
//! slug already exists for the tenant is skipped (a UNIQUE collision is
//! treated as a benign no-op), so re-seeding never duplicates the
//! handbook.

use argo_store::SessionStore;
use argo_types::TenantId;
use argo_workplace_knowledge::doc::{self, KnowledgeCategory};
use std::sync::Arc;

use crate::SeedResult;

/// Actor stamped as `created_by` on the seeded docs so the handbook's
/// authorship column reads as an operator-authored page rather than a
/// machine import.
const KNOWLEDGE_ACTOR: &str = "argo-demo-seed";

/// One canonical demo handbook doc.
struct DemoDoc {
    slug: &'static str,
    title: &'static str,
    category: KnowledgeCategory,
    body_markdown: &'static str,
}

/// Four handbook docs — one per category the page groups by (brand,
/// business-rules, faq, internal) — grounded in the demo workspace's
/// real marketplace + recruiting + support operating context.
const DEMO_DOCS: &[DemoDoc] = &[
    DemoDoc {
        slug: "brand-voice",
        title: "Brand voice",
        category: KnowledgeCategory::Brand,
        body_markdown: "# Brand voice\n\nWe write like a helpful, calm colleague — never \
            pushy, never robotic.\n\n- **Warm, not chummy.** Plain words; \
            short sentences.\n- **Honest about limits.** If we can't do \
            something, we say so and offer the next best step.\n- **No \
            hype.** State what is true; let the work speak.\n\nThis voice \
            applies to every customer-facing message a teammate drafts — \
            listings, buyer replies, candidate emails, support answers.",
    },
    DemoDoc {
        slug: "refund-and-discount-policy",
        title: "Refund & discount policy",
        category: KnowledgeCategory::BusinessRules,
        body_markdown: "# Refund & discount policy\n\n- Refunds are issued within **30 days** \
            of purchase, no questions asked.\n- A teammate may approve a \
            buyer discount up to **10%** autonomously.\n- Any discount \
            **above 10%**, or any refund **above $500**, requires operator \
            approval before it is sent.\n- Always CC the account owner on \
            refunds over $500.\n\nThese ceilings are enforced as approval \
            gates — a teammate that hits one pauses and asks rather than \
            acting.",
    },
    DemoDoc {
        slug: "buyer-faq",
        title: "Buyer FAQ",
        category: KnowledgeCategory::Faq,
        body_markdown: "# Buyer FAQ\n\n**How fast do you ship?** Most items ship within one \
            business day; you'll get tracking by email.\n\n**Can I make an \
            offer?** Yes — use the \"Make an offer\" button. Offers within \
            10% of list are usually accepted automatically.\n\n**What's your \
            return window?** 30 days from delivery, in original \
            condition.\n\nTeammates answer buyer questions from this FAQ \
            first, and escalate anything it doesn't cover.",
    },
    DemoDoc {
        slug: "escalation-runbook",
        title: "Escalation runbook",
        category: KnowledgeCategory::Internal,
        body_markdown: "# Escalation runbook\n\nA teammate escalates to a human when:\n\n1. \
            A request falls outside its declared scope or action \
            vocabulary.\n2. A money or legal decision exceeds its approval \
            ceiling.\n3. A customer is upset, or asks for something the \
            handbook doesn't cover.\n4. It is **uncertain** — when in \
            doubt, ask, don't guess.\n\nEscalations land in the approvals \
            inbox with the full context attached so a human can decide \
            quickly.",
    },
];

/// Insert the canonical demo handbook docs under `tenant_id` so
/// `/settings/knowledge` (`GET /api/v1/knowledge`) lists real wiki
/// pages instead of an empty state.
///
/// Returns the number of docs inserted (`4` on first run, `0` on
/// re-seed). Idempotent per doc: a slug already present for the tenant
/// is skipped.
pub(crate) async fn seed_knowledge(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
) -> SeedResult<usize> {
    let pool = store.pool();
    let mut inserted = 0usize;
    for d in DEMO_DOCS {
        // Idempotency: skip a slug that already exists for the tenant
        // so re-seeding (e.g. to add a teammate) does not error on the
        // UNIQUE(tenant_id, slug) constraint.
        if doc::get(pool, &tenant_id, d.slug).await?.is_some() {
            continue;
        }
        doc::create(
            pool,
            &tenant_id,
            d.slug,
            d.title,
            d.category,
            d.body_markdown,
            KNOWLEDGE_ACTOR,
        )
        .await?;
        inserted += 1;
    }
    Ok(inserted)
}

#[cfg(test)]
#[path = "seed_knowledge_tests.rs"]
mod tests;
