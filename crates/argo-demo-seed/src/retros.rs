//! Demo retro rows for the canonical v1.0 seed.
//!
//! Closes UX audit `whole-site F08` — `/retro` rendered "0 retros across
//! the tenant — No retros yet" in the authed demo session because the
//! seed wrote no rows. Investor walkthroughs that drift past the 6-scene
//! runbook (Q&A, deep dives) need 3 real retros the operator can point at.
//!
//! Seeds 3 retros under the demo tenant, each bound to a canonical Scene-1
//! persona ([`crate::personas::DEMO_SEED_PERSONAS`]) so the SPA card
//! preview renders a real worker name + the retro's 5-section narrative.
//! Each retro references the `customer_research_followup` workflow seeded
//! by [`crate::workflow`] so the retro detail page surfaces a real
//! provenance chain.
//!
//! Schema is owned by `argo_store::retro::RetroStore`; this module only
//! writes through the store's public API.

use argo_identity::IdentityStore;
use argo_store::retro::{NewRetro, RetroStore};
use argo_store::{SessionStore, WorkerId};
use argo_types::TenantId;
use chrono::Utc;
use serde_json::{json, Value};

use crate::SeedResult;

/// Inclusive retro window depth — matches `argo_store::retro::DEFAULT_WINDOW_DAYS`.
const WINDOW_DAYS: i64 = 7;

/// Workflow name woven into each retro's `surface_metrics` narrative so
/// the retro detail page surfaces a real provenance chain back to the
/// `customer_research_followup` workflow seeded by [`crate::workflow`].
const WORKFLOW_NAME: &str = "customer_research_followup";

/// One demo retro entry.
struct RetroEntry {
    /// Persona name to look up in the IdentityStore. Matches
    /// `crate::personas::DEMO_SEED_PERSONAS` exactly.
    persona_name: &'static str,
    /// Frozen metrics aggregated for the 7-day window. Shape mirrors
    /// `argo_store::retro::metrics_template`.
    metrics: fn() -> Value,
    /// 5-section narrative body — the load-bearing shape the SPA card
    /// preview + retro detail page bind to.
    sections: fn() -> Value,
}

/// Insert the 3 canonical demo retros under `tenant_id`. Returns the
/// number of retro rows newly created. Idempotent — re-running against
/// a tenant that already has the canonical retros inserts duplicates,
/// matching the existing `seed_demo_workers` / `seed_demo_skills`
/// per-call semantics (the operator calls `argo demo seed` once per
/// fresh tenant).
///
/// Worker resolution is done via `IdentityStore::list_workers` rather
/// than the passed-in `worker_ids` slice so the lookup can be done by
/// persona name — that decouples this seed from the caller's id ordering.
/// If any of the three required personas (Mariah, Priya, Peta) are
/// missing from the tenant, the function returns `0` and writes nothing.
///
/// # Errors
///
/// Propagates any identity-store or retro-store backend error.
pub(crate) async fn seed_demo_retros(
    store: &SessionStore,
    tenant_id: TenantId,
) -> SeedResult<usize> {
    let identities = IdentityStore::new(store);
    identities.ensure_schema().await?;
    let workers = identities.list_workers(tenant_id, false).await?;

    let resolve = |name: &str| -> Option<WorkerId> {
        workers
            .iter()
            .find(|w| w.persona.name == name)
            .map(|w| w.id)
    };

    let entries: [RetroEntry; 3] = [
        RetroEntry {
            persona_name: "Mariah",
            metrics: mariah_metrics,
            sections: mariah_sections,
        },
        RetroEntry {
            persona_name: "Priya",
            metrics: priya_metrics,
            sections: priya_sections,
        },
        RetroEntry {
            persona_name: "Peta",
            metrics: peta_metrics,
            sections: peta_sections,
        },
    ];

    let store_pool = store.pool();
    let retros = RetroStore::new(store_pool);
    let tenant_blob = tenant_id.0.as_bytes().to_vec();
    let now = Utc::now().timestamp();
    let window_start = now - WINDOW_DAYS * 24 * 3600;
    let mut inserted = 0;

    for entry in &entries {
        let Some(worker_id) = resolve(entry.persona_name) else {
            tracing::warn!(
                persona = entry.persona_name,
                "demo retro skipped — persona not found under tenant"
            );
            continue;
        };
        let worker_blob = worker_id.0.as_bytes().to_vec();
        let new = NewRetro {
            tenant_id: tenant_blob.clone(),
            worker_id: worker_blob,
            window_start,
            window_end: now,
            metrics_json: (entry.metrics)(),
            created_at: now,
        };
        let id = retros.create(new).await.map_err(map_retro_err)?;
        let sections = (entry.sections)();
        retros
            .update_sections(&tenant_blob, id, &sections)
            .await
            .map_err(map_retro_err)?;
        inserted += 1;
    }
    Ok(inserted)
}

fn map_retro_err(e: argo_store::retro::RetroError) -> crate::SeedError {
    use argo_store::retro::RetroError;
    match e {
        RetroError::Backend(inner) => crate::SeedError::Argo(inner),
        RetroError::NotFound | RetroError::Finalized => crate::SeedError::Argo(
            argo_types::ArgoError::Provider(format!("demo retro seed: {e}")),
        ),
    }
}

// ── narrative bodies ──────────────────────────────────────────────────
//
// Each pair of functions below is a single retro. The metrics shape
// matches `argo_store::retro::metrics_template`; the sections shape
// matches the 5 slots `argo_store::retro::section_template` declares.
// The narratives reference the seeded `customer_research_followup`
// workflow + the Scene-1 cohort so the SPA card preview shows real
// provenance instead of empty strings.

fn mariah_metrics() -> Value {
    json!({
        "actions_total": 47,
        "actions_approved": 31,
        "actions_refused": 9,
        "escalations": 4,
        "learned_skills_added": 0,
        "error_count_by_category": { "approval.refused": 9 },
    })
}

fn mariah_sections() -> Value {
    json!({
        "surface_metrics": format!(
            "Sprint 28 retro — marketplace pricing experiment. 47 actions / 31 \
             approved / 9 refused / 4 escalations across the window. Mariah ran \
             the pricing experiment against the `{WORKFLOW_NAME}` workflow's \
             follow-up arm: +6.4% margin on the marketplace listing batch but 9 \
             buyer refusals on the higher-priced lots vs. 3 the week before."
        ),
        "what_went_well":
            "Pricing tier B (mid-tier listings) held conversion at 92% with the \
             new floor. Approval flow on the buyer-discount cap (+$50) kept ops \
             in the loop without slowing throughput. Mariah owned listing copy \
             end-to-end; no escalations on tone.",
        "surprises":
            "Refusal rate on tier A (premium lots) doubled vs. baseline. The 9 \
             buyers refusing were the same 4 customers from the prior week — \
             pricing felt sticky to them, not aspirational.",
        "what_to_change":
            "Tier A pricing reverts to baseline next sprint; tier B holds at the \
             experimental floor. Wire an approval guard for tier-A repricing \
             >+15% so the operator sees the change on the inbox before it ships.",
        "action_items":
            "1) Mariah reverts tier A by Mon. 2) Engineering wires the >+15% \
             approval guard against the `customer_research_followup` workflow's \
             repricing step. 3) Re-run the experiment in 2 sprints with the new \
             approval gate. Mixed verdict — kept 4 wins, stopped 2 items.",
    })
}

fn priya_metrics() -> Value {
    json!({
        "actions_total": 62,
        "actions_approved": 58,
        "actions_refused": 1,
        "escalations": 2,
        "learned_skills_added": 1,
        "error_count_by_category": {},
    })
}

fn priya_sections() -> Value {
    json!({
        "surface_metrics": format!(
            "Q1 review — candidate funnel improvements. 62 actions / 58 approved \
             / 1 refused / 2 escalations + 1 learned skill across the quarter. \
             Priya's interview-booking arm of `{WORKFLOW_NAME}` shipped a 3.1x \
             throughput gain (interviews booked / candidate hours) vs. Q4."
        ),
        "what_went_well":
            "Calibration cadence with hiring managers caught 4 mis-leveled \
             candidates before they reached the panel. Approval-to-book latency \
             dropped from 18h median to 4h — the new slot-suggestion skill \
             paid back its training cost inside week 2. Positive verdict.",
        "surprises":
            "Candidate no-show rate fell to 2% (was 11% in Q4). Tracked it back \
             to the day-of confirmation SMS Priya added — small change, large \
             effect. Worth packaging as a reusable skill for the marketplace.",
        "what_to_change":
            "Promote the day-of confirmation SMS to a tenant-default skill. Add \
             a structured `candidate_source` field to the funnel rows so we can \
             slice approval rate by source in the next review.",
        "action_items":
            "1) Priya promotes the day-of SMS skill to the marketplace by \
             Fri. 2) Engineering adds `candidate_source` to the funnel schema. \
             3) Re-run the funnel review at the end of Q2 with the new dimension.",
    })
}

fn peta_metrics() -> Value {
    json!({
        "actions_total": 28,
        "actions_approved": 11,
        "actions_refused": 6,
        "escalations": 9,
        "learned_skills_added": 0,
        "error_count_by_category": { "halt": 4, "escalation.required": 9 },
    })
}

fn peta_sections() -> Value {
    json!({
        "surface_metrics": format!(
            "Standup digest reliability post-mortem. 28 actions / 11 approved / \
             6 refused / 9 escalations + 4 halts across the window — well above \
             our error budget. Peta's standup-digest beat (the project-coord arm \
             of `{WORKFLOW_NAME}`) missed its delivery SLA 3 of 5 days last week."
        ),
        "what_went_well":
            "Failure mode was loud, not silent — the digest never went out half- \
             built. The halt-then-escalate path worked. Peta caught all 4 halts \
             inside 6 minutes and re-routed manually so engineering still got a \
             standup on those days.",
        "surprises":
            "Root cause wasn't the digest skill — it was the upstream Slack \
             token rotation that nobody documented. The 4 halts were all 401 \
             responses from Slack, not skill bugs. We were blaming the wrong \
             layer for two days.",
        "what_to_change":
            "Promote connector-health checks to a pre-flight gate on the digest \
             skill so a stale token surfaces as `connector.unhealthy` (not a \
             generic halt). Document the Slack token rotation in the runbook.",
        "action_items":
            "1) Peta writes the runbook entry by Thu. 2) Engineering adds the \
             connector pre-flight gate against `{WORKFLOW_NAME}` step \
             `summarize_question` by Mon. 3) Standup-digest SLA re-baselines \
             next sprint after the gate ships. Negative verdict — 3 items to \
             change, 0 to keep as-is.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::personas::DEMO_SEED_PERSONAS;
    use argo_identity::{IdentityStore, Persona, Role, Worker};
    use std::sync::Arc;

    async fn open_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        Arc::new(store)
    }

    async fn seed_canonical_workers(store: &SessionStore, tenant: TenantId) {
        let identities = IdentityStore::new(store);
        identities.ensure_schema().await.unwrap();
        for spine in DEMO_SEED_PERSONAS {
            let persona = Persona::named(spine.name, "test");
            let role = Role::new("test");
            let worker = Worker::draft(tenant, spine.vertical_id, persona, role);
            identities.create_worker(&worker).await.unwrap();
        }
    }

    #[tokio::test]
    async fn seeds_three_retros_against_canonical_personas() {
        let store = open_store().await;
        let tenant = TenantId::new();
        seed_canonical_workers(&store, tenant).await;

        let inserted = seed_demo_retros(&store, tenant).await.unwrap();
        assert_eq!(inserted, 3);

        let retros = RetroStore::new(store.pool())
            .list_all(tenant.0.as_bytes(), None, 10)
            .await
            .unwrap();
        assert_eq!(retros.len(), 3);
        for row in &retros {
            // Every retro has the full 5-section narrative populated.
            for key in [
                "surface_metrics",
                "what_went_well",
                "surprises",
                "what_to_change",
                "action_items",
            ] {
                assert!(
                    row.sections_json[key]
                        .as_str()
                        .is_some_and(|s| !s.is_empty()),
                    "retro section `{key}` empty (id {})",
                    row.id
                );
            }
            // Workflow provenance is preserved in the first section so the
            // SPA card preview surfaces a real chain back to the workflow
            // definition seeded by `crate::workflow`.
            assert!(
                row.sections_json["surface_metrics"]
                    .as_str()
                    .is_some_and(|s| s.contains(WORKFLOW_NAME)),
                "retro `surface_metrics` must cite `{WORKFLOW_NAME}` for \
                 provenance chain"
            );
        }
    }

    #[tokio::test]
    async fn skips_missing_personas_without_failure() {
        let store = open_store().await;
        let tenant = TenantId::new();
        // No workers seeded — nothing to resolve, so we expect 0 inserts
        // and no error.
        let inserted = seed_demo_retros(&store, tenant).await.unwrap();
        assert_eq!(inserted, 0);
    }

    #[tokio::test]
    async fn sibling_tenant_sees_zero() {
        let store = open_store().await;
        let tenant = TenantId::new();
        let other = TenantId::new();
        seed_canonical_workers(&store, tenant).await;
        seed_demo_retros(&store, tenant).await.unwrap();
        let other_retros = RetroStore::new(store.pool())
            .list_all(other.0.as_bytes(), None, 10)
            .await
            .unwrap();
        assert!(
            other_retros.is_empty(),
            "sibling tenant must not see seeded retros"
        );
    }
}
