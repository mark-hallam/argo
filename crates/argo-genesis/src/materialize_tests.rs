//! Headline P1 test: a NOVEL role → a synthesized [`CapabilitySpec`] →
//! a `load_dir`-valid vertical → a real least-privilege capability profile
//! (NOT owner-`*`), with the deterministic safety floor preserved.

use super::*;
use crate::spec::{CapabilityDecl, CapabilitySpec, EscalationSpec, ExecutorIntent};
use crate::{Archetype, BroadRoleDraft};
use argo_capability::WorkerCapabilityProfile;
use argo_vertical::ActionRiskLevel;

/// The brief's exact novel-role example: "a vineyard logistics coordinator
/// who schedules harvests". No archetype fits cleanly; the spec is
/// synthesized from scratch with `CalendarCoordinator` as the cheap prior.
fn vineyard_spec() -> CapabilitySpec {
    let draft = BroadRoleDraft {
        charter: "a vineyard logistics coordinator who schedules harvests, \
                  drafts trucking plans, and dispatches haulage to the winery"
            .into(),
        persona: "vinny".into(),
        persona_description: "Coordinates vineyard harvest scheduling and haulage logistics."
            .into(),
        action_vocabulary: vec![
            "schedule_harvest".into(),
            "draft_truck_plan".into(),
            "send_dispatch_notice".into(),
        ],
        success_metrics: vec!["zero missed harvest windows".into()],
        onboarding_script: "Hi, I keep the harvest calendar tight.".into(),
    };
    CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator)
}

#[test]
fn novel_role_materializes_to_a_load_dir_valid_vertical() {
    let spec = vineyard_spec();
    let plan = materialize_spec(&spec).expect("materialize a novel role");

    // The plan carries the spec (so the disk writer goes archetype-free) and
    // the nearest-archetype prior (so the SSE payload has a stable slug).
    assert!(plan.spec.is_some(), "materialized plan carries its spec");
    assert_eq!(plan.archetype, Archetype::CalendarCoordinator);
    // From-scratch id prefix — NOT an archetype-templated id.
    assert_eq!(plan.vertical_id, "genesis-vinny");

    // The manifest declares actions, tools, escalation — NONE of which the
    // archetype path's broad dead-end could ever write.
    let manifest = &plan.manifest;
    let actions = manifest.actions.as_ref().expect("[actions] present");
    assert_eq!(actions.actions.len(), 3);
    assert!(manifest.tools.is_some(), "[tools] present");
    assert!(manifest.escalation.is_some(), "[escalation] present");
    assert!(!manifest.all_kpis().is_empty(), "KPIs present");

    // Round-trip through the REAL loader and assert zero LoadErrors.
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&plan.vertical_id);
    write_capability_vertical(&spec, &out).expect("write the vertical");
    for rel in [
        "vertical.toml",
        "worker.toml",
        "eval-seed.toml",
        "prompts/system.md",
        "prompts/persona.md",
        "memory/schema.toml",
        "eval/seed.jsonl",
    ] {
        assert!(out.join(rel).exists(), "missing {rel}");
    }
    let (registry, errors) = argo_vertical::load_dir(dir.path()).expect("load_dir");
    assert!(errors.is_empty(), "load errors for novel role: {errors:?}");
    let loaded = registry
        .get(&plan.vertical_id)
        .expect("novel vertical loaded by id");
    assert_eq!(loaded.manifest.vertical.id, plan.vertical_id);
}

#[test]
fn materialized_teammate_gets_a_real_least_privilege_profile_not_owner_star() {
    // The capability fence (argo-agent-runner::turn_vertical_fence) derives a
    // WorkerCapabilityProfile from the manifest's [actions] keys +
    // [tools].allowed − [tools].denied + [mcp_servers] (draft-only). We
    // replicate that EXACT mapping here against the MATERIALIZED manifest to
    // prove a from-scratch teammate is fenced like an archetype one: a real,
    // non-empty, non-`*` allow-set — never owner reach.
    let spec = vineyard_spec();
    let plan = materialize_spec(&spec).expect("materialize");
    let m = &plan.manifest;

    let declared_actions: Vec<String> =
        m.actions.as_ref().unwrap().actions.keys().cloned().collect();
    let (allowed, denied): (Vec<String>, Vec<String>) = m
        .tools
        .as_ref()
        .map(|t| (t.allowed.clone(), t.denied.clone()))
        .unwrap();
    let integrations: Vec<String> = m
        .mcp_servers
        .as_ref()
        .map(|s| s.servers.keys().cloned().collect())
        .unwrap_or_default();

    let profile = WorkerCapabilityProfile::derive(
        declared_actions,
        allowed,
        denied.clone(),
        integrations,
    );

    // The teammate's own actions are permitted...
    assert!(profile.permits_tool("schedule_harvest"));
    assert!(profile.permits_tool("draft_truck_plan"));
    // ...and the owner's `*` reach is NOT: undeclared/destructive tools denied.
    assert!(!profile.permits_tool("terminal"));
    assert!(!profile.permits_tool("wire_funds"));
    assert!(!profile.permits_tool("drop_table"));
    // The allow-set is real (non-empty) — the fence will bind concrete
    // ToolDispatch grants, never owner-`*`.
    assert!(
        !profile.allowed_tools().is_empty(),
        "a materialized teammate must carry a real allow-set, not deny-all"
    );
    // `[tools].denied` lists the sensitive verbs not in the vocabulary, so the
    // fence refuses them even if hallucinated.
    assert!(denied.contains(&"pay".to_string()));
    assert!(denied.contains(&"delete".to_string()));
}

#[test]
fn materialized_actions_pass_the_safety_floor() {
    // The deterministic safety judge MUST have run during materialization:
    // every sensitive verb requires approval, and judging the on-disk actions
    // again is a no-op (already fixed).
    let spec = vineyard_spec();
    let plan = materialize_spec(&spec).expect("materialize");
    let actions = plan.manifest.actions.as_ref().unwrap();
    // `send_dispatch_notice` is sensitive → must require approval.
    assert!(
        actions.actions["send_dispatch_notice"].requires_approval,
        "sensitive verb must be approval-gated after materialization"
    );
    // Re-judging is clean (the auto_fix + judge ran in materialize_spec).
    crate::safety_judge::judge(actions).expect("materialized actions pass safety judge");
}

#[test]
fn materialized_action_with_sensitive_verb_synthesized_unsafe_is_still_gated() {
    // Adversarial: even if synthesis proposes a sensitive action as
    // NOT requiring approval, materialization's auto_fix forces it gated.
    let spec = CapabilitySpec {
        charter: "a comms assistant".into(),
        persona: "cassie".into(),
        persona_description: "Drafts and sends comms.".into(),
        responsibilities: vec![],
        capabilities: vec![CapabilityDecl {
            action_name: "send_broadcast".into(),
            intent: "Send a broadcast".into(),
            risk: ActionRiskLevel::Low,
            reversible: true,
            // Synthesis (wrongly) said no approval needed:
            requires_approval: false,
            proposed_executor: ExecutorIntent::Human,
        }],
        success_metrics: vec!["broadcasts land".into()],
        onboarding_script: "Hi.".into(),
        escalation: EscalationSpec::default(),
        nearest_archetype: Archetype::SocialMediaDrafter,
        from_scratch: true,
    };
    let plan = materialize_spec(&spec).expect("materialize");
    assert!(
        plan.manifest.actions.as_ref().unwrap().actions["send_broadcast"].requires_approval,
        "auto_fix must override an unsafe synthesized approval flag"
    );
}

#[test]
fn empty_capability_spec_fails_closed() {
    let spec = CapabilitySpec {
        charter: "an empty role".into(),
        persona: "ghost".into(),
        persona_description: "Does nothing.".into(),
        responsibilities: vec![],
        capabilities: vec![],
        success_metrics: vec![],
        onboarding_script: String::new(),
        escalation: EscalationSpec::default(),
        nearest_archetype: Archetype::ResearchAnalyst,
        from_scratch: true,
    };
    let err = materialize_spec(&spec).expect_err("empty spec must fail closed");
    assert!(matches!(err, crate::GenesisError::Validation { .. }));
}

#[test]
fn mcp_intent_is_draft_only_action_stays_human() {
    // A capability proposing an MCP executor records the server in
    // [mcp_servers] (draft) but the action still runs `human` at birth
    // (DEC-025 #6 — never auto-bind integrations).
    let spec = CapabilitySpec {
        charter: "an email assistant".into(),
        persona: "emma".into(),
        persona_description: "Drafts email.".into(),
        responsibilities: vec![],
        capabilities: vec![CapabilityDecl {
            action_name: "draft_email".into(),
            intent: "Draft an email".into(),
            risk: ActionRiskLevel::Low,
            reversible: true,
            requires_approval: false,
            proposed_executor: ExecutorIntent::Mcp {
                server: "gmail".into(),
                tool: Some("drafts.create".into()),
            },
        }],
        success_metrics: vec!["emails drafted".into()],
        onboarding_script: "Hi.".into(),
        escalation: EscalationSpec::default(),
        nearest_archetype: Archetype::CustomerSupportAgent,
        from_scratch: true,
    };
    let plan = materialize_spec(&spec).expect("materialize");
    // gmail is declared as a draft mcp server...
    let servers = plan.manifest.mcp_servers.as_ref().expect("[mcp_servers]");
    assert!(servers.servers.contains_key("gmail"));
    // ...but the action executor is still `human` (no live bind).
    assert_eq!(
        plan.manifest.actions.as_ref().unwrap().actions["draft_email"]
            .executor
            .kind_str(),
        "human"
    );
}

#[test]
fn broad_outcome_materialize_seam_is_no_longer_terminal() {
    // The integration seam: GenesisOutcome::Broad → materialize() yields a
    // loadable plan. (Builds the Broad outcome directly from a draft, then
    // proves it materializes — the dead-end is closed.)
    let draft = BroadRoleDraft {
        charter: "a chief vibes officer who keeps team morale high".into(),
        persona: "vibe".into(),
        persona_description: "Keeps the team's energy positive.".into(),
        action_vocabulary: vec!["run_morale_check".into(), "draft_shoutout".into()],
        success_metrics: vec!["rising team sentiment".into()],
        onboarding_script: "Hi, I keep the vibes immaculate.".into(),
    };
    let outcome = crate::GenesisOutcome::Broad(Box::new(draft));
    assert_eq!(outcome.archetype_slug(), "broad");
    let plan = outcome.materialize().expect("broad outcome materializes");
    assert!(plan.vertical_id.starts_with("genesis-"));
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&plan.vertical_id);
    crate::write_vertical(&plan, &out).expect("write");
    let (_registry, errors) = argo_vertical::load_dir(dir.path()).expect("load_dir");
    assert!(errors.is_empty(), "chief vibes officer load errors: {errors:?}");
}

// --- Display-name derivation (demo-blocker fix) ---------------------------
// Regression guard: a Genesis-hired teammate's `vertical.name` (surfaced as the
// SSE `display_name` and persisted as `persona.name` at hire) must read like a
// real person's name, NEVER the old `"Genesis Teammate (<persona>)"`
// placeholder. The live repro was a charter naming "Maya" → a worker called
// "Genesis Teammate (sales)".

/// A from-scratch spec for a marketing intern, with an explicit persona slug +
/// charter. Mirrors the real `from_broad_draft` (`from_scratch = true`) path —
/// the exact path that produced the placeholder bug.
fn named_spec(charter: &str, persona: &str) -> CapabilitySpec {
    let draft = BroadRoleDraft {
        charter: charter.into(),
        persona: persona.into(),
        persona_description: "Drafts and schedules marketing content.".into(),
        action_vocabulary: vec!["draft_post".into(), "schedule_post".into()],
        success_metrics: vec!["posts shipped on time".into()],
        onboarding_script: "Hi, I run the content calendar.".into(),
    };
    CapabilitySpec::from_broad_draft(draft, Archetype::MarketingIntern)
}

#[test]
fn genesis_hire_naming_maya_in_charter_yields_display_name_maya() {
    // Even when the persona slug is a generic synthesis fallback, an explicit
    // "named Maya" in the charter wins — the operator said the name out loud.
    let spec = named_spec(
        "a marketing intern named Maya who drafts and schedules social posts",
        "operator",
    );
    let plan = materialize_spec(&spec).expect("materialize a named teammate");
    assert_eq!(
        plan.manifest.vertical.name, "Maya",
        "charter `named Maya` must surface as the display name"
    );
    // And the placeholder must be gone entirely.
    assert!(
        !plan.manifest.vertical.name.contains("Genesis Teammate"),
        "display name must never be the generic placeholder"
    );
}

#[test]
fn genesis_hire_naming_variants_all_extract_the_name() {
    for charter in [
        "a sdr whose name is Sasha and who books meetings",
        "an analyst, call her Priya, who summarises reports",
        "a coordinator called Marcus who keeps the calendar tight",
        "a researcher who goes by Nori and triages inbound",
    ] {
        let spec = named_spec(charter, "operator");
        let name = materialize_spec(&spec).expect("materialize").manifest.vertical.name;
        assert!(
            ["Sasha", "Priya", "Marcus", "Nori"].contains(&name.as_str()),
            "charter `{charter}` extracted unexpected name `{name}`"
        );
    }
}

#[test]
fn genesis_hire_no_name_in_charter_humanizes_the_persona_slug() {
    // No naming phrase in the charter, but a real persona slug → Title-Cased
    // name from the slug (`maya` → "Maya", `maria-garcia` → "Maria Garcia").
    let spec = named_spec("a marketing intern who drafts social posts", "maya");
    assert_eq!(materialize_spec(&spec).expect("materialize").manifest.vertical.name, "Maya");

    let spec = named_spec("a marketing intern who drafts social posts", "maria-garcia");
    assert_eq!(
        materialize_spec(&spec).expect("materialize").manifest.vertical.name,
        "Maria Garcia"
    );
}

#[test]
fn genesis_hire_no_name_and_generic_slug_falls_back_to_role_label_not_placeholder() {
    // Worst case: no name anywhere AND a generic synthesis-fallback slug. The
    // display name must still be a real role label, NOT "Genesis Teammate".
    for slug in ["operator", "worker", "pioneer"] {
        let spec = named_spec("someone who drafts and schedules social posts", slug);
        let name = materialize_spec(&spec).expect("materialize").manifest.vertical.name;
        assert_eq!(
            name, "Marketing Intern",
            "generic slug `{slug}` must fall back to the archetype label"
        );
        assert!(
            !name.contains("Genesis Teammate"),
            "fallback must never be the generic placeholder"
        );
    }
}
