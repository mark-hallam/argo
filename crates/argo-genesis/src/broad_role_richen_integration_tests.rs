//! Headline richening tests (Wave-2 item ⑧): a one-line off-taxonomy role
//! yields a FULL teammate — non-empty responsibilities + typed KPIs + a firing
//! cadence + least-privilege access — not one whose system prompt says
//! "(derive your responsibilities from the charter above)".
//!
//! Lives in its own sibling test module so `broad_role_tests.rs` stays under
//! the 500-LOC per-file ceiling the loc-budget gate enforces.

use super::{genesis_broad_role, BroadRoleClient};
use crate::archetype::Archetype;
use argo_types::ArgoResult;
use std::sync::Mutex;

/// A FULLY-richened model envelope: the four base fields PLUS concrete
/// responsibilities, a cron cadence, typed KPIs, and an access list (with one
/// invented `internal_db` the whitelist must drop).
const CANNED_RICH_RESPONSE: &str = r#"{
  "persona_description": "A calm, observant chief vibes officer who keeps team energy positive.",
  "responsibilities": [
    "Run a weekly check-in with each teammate and log the mood read",
    "Draft and send kudos for wins worth celebrating",
    "Surface any team-energy risk to the operator before it festers"
  ],
  "action_vocabulary": ["check_in", "draft_kudos", "schedule_offsite", "summarise_mood"],
  "cadence_cron": "0 9 * * 1",
  "success_metrics": ["weekly_vibe_score_up", "kudos_sent_per_week", "team_nps_quarterly"],
  "success_criteria": [
    {"name": "weekly vibe score", "kind": "percentage", "target": 80, "comparator": "gte"},
    {"name": "kudos sent per week", "kind": "count", "target": 5, "comparator": "gte"}
  ],
  "access_integrations": ["Slack", "gcal", "internal_db"],
  "onboarding_script": "Hi team! I'm Vibe, your new chief vibes officer."
}"#;

struct MockBroadClient {
    response: String,
    last_prompt: Mutex<Option<String>>,
}

#[async_trait::async_trait]
impl BroadRoleClient for MockBroadClient {
    async fn synthesize(&self, prompt: &str) -> ArgoResult<String> {
        if let Ok(mut slot) = self.last_prompt.lock() {
            *slot = Some(prompt.to_string());
        }
        Ok(self.response.clone())
    }
}

fn rich_client() -> MockBroadClient {
    MockBroadClient {
        response: CANNED_RICH_RESPONSE.to_string(),
        last_prompt: Mutex::new(None),
    }
}

#[tokio::test]
async fn richened_broad_role_carries_responsibilities_kpis_cadence_access() {
    let client = rich_client();
    let draft = genesis_broad_role(
        "be my chief vibes officer, keep the team energy positive",
        "vibe",
        &client,
    )
    .await
    .expect("rich envelope parses");

    // (1) The DRAFT carries every richened field.
    assert_eq!(
        draft.responsibilities.len(),
        3,
        "responsibilities must be populated, not empty"
    );
    assert!(draft.responsibilities[0].contains("weekly check-in"));
    assert_eq!(
        draft.cadence.as_deref(),
        Some("0 9 * * 1"),
        "cadence rides as a bridge-accepted cron"
    );
    assert_eq!(draft.success_criteria.len(), 2, "typed KPIs populated");
    // (2) least-privilege: the invented `internal_db` is dropped; only wired
    // connectors survive.
    assert_eq!(
        draft.access_integrations,
        vec!["slack", "gcal"],
        "only wired connectors survive; invented access dropped"
    );

    // The prompt itself asks for the richer fields + holds access to the wired
    // connector list (so the model cannot invent integrations).
    let prompt = client.last_prompt.lock().unwrap().clone().unwrap();
    assert!(prompt.contains("responsibilities"));
    assert!(prompt.contains("cadence_cron"));
    assert!(prompt.contains("success_criteria"));
    assert!(prompt.contains("access_integrations"));
    assert!(prompt.contains("never invent one"));
}

#[tokio::test]
async fn richened_spec_populates_typed_fields_and_drops_derive_fallback() {
    use argo_vertical::KpiKind;

    let client = rich_client();
    let draft = genesis_broad_role("be my chief vibes officer", "vibe", &client)
        .await
        .expect("rich envelope parses");
    let spec = crate::CapabilitySpec::from_broad_draft(draft, Archetype::ResearchAnalyst);

    // (3) The SPEC carries the richened fields the materializer + role contract
    // consume.
    assert!(
        !spec.responsibilities.is_empty(),
        "spec responsibilities non-empty → system prompt won't fall back",
    );
    assert_eq!(
        spec.cadence.as_deref(),
        Some("0 9 * * 1"),
        "cadence is a 5-field cron the hire→duty bridge accepts",
    );
    assert_eq!(spec.success_criteria.len(), 2, "typed success-criteria carried");
    assert_eq!(spec.success_criteria[0].kind, KpiKind::Percentage);
    assert_eq!(spec.access_integrations, vec!["slack", "gcal"]);

    // (4) The cadence is genuinely a shape the bridge's cron validator accepts
    // (5 or 6 whitespace fields) — so a richened broad role yields a FIRING
    // duty, not a skipped one.
    let field_count = spec.cadence.as_deref().unwrap().split_whitespace().count();
    assert!(
        field_count == 5 || field_count == 6,
        "cadence must be 5/6-field cron; got {field_count} fields",
    );

    // (5) Least-privilege access is bound as a DRAFT MCP intent (surfaced under
    // the manifest's `[mcp_servers]`), never an executor that can transmit.
    let bound: Vec<&str> = spec
        .capabilities
        .iter()
        .filter_map(|c| match &c.proposed_executor {
            crate::ExecutorIntent::Mcp { server, .. } => Some(server.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        bound.contains(&"slack") && bound.contains(&"gcal"),
        "each whitelisted connector is bound as a draft integration; got {bound:?}",
    );

    // (6) The synthesized system prompt no longer carries the "(derive your
    // responsibilities from the charter above)" fallback line — the literal
    // symptom item ⑧ targets — and the materializer still accepts the spec
    // (fail-closed hire gate unaffected).
    let system_prompt = crate::system_prompt_from_spec(&spec);
    assert!(
        !system_prompt.contains("derive your responsibilities from the charter"),
        "richened teammate must NOT emit the derive-from-charter fallback",
    );
    assert!(
        system_prompt.contains("weekly check-in"),
        "the authored responsibilities ride into the system prompt",
    );
    crate::materialize_spec(&spec).expect("rich spec still materializes (hire gate intact)");
}
