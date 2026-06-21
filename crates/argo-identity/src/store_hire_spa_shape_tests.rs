//! Regression proof for the HIRE→DUTY bridge shape mismatch (Item-2a/2b).
//!
//! The SPA hire flow (`web/lib/hire/draft-types.ts`) sends a duty as
//! `{label, schedule:{kind,spec}, defaultEnabled}` and the operator's
//! success-measures under `successCriteria`. The gateway's canonical
//! [`HireDraft`] expects `{cron, entryAction, description}` duties and a
//! `kpis` field. Before the dual-shape deserialiser + the `successCriteria`
//! alias, a realistic SPA hire deserialised to ZERO usable duties (the
//! `worker_draft` was rejected outright) and empty KPIs — so the teammate's
//! schedule dropped on the floor and its goals rendered blank with no error.
//!
//! These tests pin that a realistic SPA payload deserialises into a draft
//! whose duty carries a non-empty cron AND whose KPIs are non-empty — the
//! exact inputs the hire→`role_duty` bridge needs to write a live duty.

use super::*;

/// The realistic "morning report" hire the operator describes: a teammate
/// that emails a daily report at 09:00 on weekdays, with two success
/// measures. Shaped EXACTLY as the SPA serialises `worker_draft`
/// (`schedule:{kind,spec}` duties + camelCase `successCriteria`).
const SPA_HIRE_BODY: &str = r#"{
  "suggestedName": "Morgan",
  "charter": "Email me a concise morning report every weekday at 9am.",
  "suggestedVertical": "marketplace-seller",
  "approvalPolicy": { "level": "balanced" },
  "duties": [
    {
      "label": "Send the morning report",
      "schedule": { "kind": "cron", "spec": "0 9 * * 1-5" },
      "defaultEnabled": true
    }
  ],
  "successCriteria": [
    "Report lands before 9:15am",
    "Covers overnight orders + flagged messages"
  ]
}"#;

#[test]
fn spa_hire_payload_yields_duty_with_cron_and_kpis() {
    // Deserialise EXACTLY the way the gateway hire route does
    // (`serde_json::from_value` on the `worker_draft` envelope field).
    let draft: HireDraft =
        serde_json::from_str(SPA_HIRE_BODY).expect("SPA-shaped worker_draft must deserialise");

    // (1) Schedule-shape transform: the SPA's `schedule.spec` became the
    //     canonical `cron`, `label` became the description, and an
    //     `entry_action` was synthesised — so the bridge has a real duty.
    assert_eq!(draft.duties.len(), 1, "the morning-report duty survived");
    let duty = &draft.duties[0];
    assert_eq!(duty.cron, "0 9 * * 1-5", "schedule.spec mapped onto cron");
    assert!(duty.default_enabled, "duty stays enabled");
    assert!(
        !duty.entry_action.trim().is_empty(),
        "entry_action synthesised from the label, never blank"
    );
    assert_eq!(duty.entry_action, "send_the_morning_report");
    assert_eq!(duty.description, "Send the morning report");

    // (2) KPI field map: `successCriteria` aliased onto `kpis`, so the
    //     teammate's goals are NON-EMPTY (the blank-goals churn path).
    assert_eq!(draft.kpis.len(), 2, "successCriteria mapped onto kpis");
    assert_eq!(draft.kpis[0], "Report lands before 9:15am");

    // The whole point: this draft now passes hire validation, so it reaches
    // the role_duty bridge instead of being rejected at the boundary.
    validate_hire_draft(&draft).expect("the transformed draft is a valid hire");
}

#[test]
fn canonical_shape_still_deserialises_unchanged() {
    // The Genesis SSE / first-hire / curl path must keep working: explicit
    // `cron` + `entryAction` + `kpis` ride through untouched.
    let body = r#"{
      "suggestedName": "Maya",
      "charter": "Own outbound marketplace follow-up end-to-end.",
      "suggestedVertical": "marketplace-seller",
      "approvalPolicy": { "level": "balanced" },
      "duties": [
        { "cron": "0 9 * * 1-5", "entryAction": "follow_up",
          "description": "Send approved follow-up.", "defaultEnabled": true }
      ],
      "kpis": ["Reply within 2h"]
    }"#;
    let draft: HireDraft = serde_json::from_str(body).expect("canonical shape deserialises");
    assert_eq!(draft.duties[0].cron, "0 9 * * 1-5");
    assert_eq!(draft.duties[0].entry_action, "follow_up");
    assert_eq!(draft.kpis, vec!["Reply within 2h".to_string()]);
}

#[test]
fn duty_without_any_schedule_is_rejected() {
    // A duty carrying neither `cron` nor `schedule.spec` can never fire;
    // deserialisation must fail loudly rather than commit a scheduleless row.
    let body = r#"{ "label": "do a thing", "defaultEnabled": true }"#;
    let parsed: Result<HireDutyDraft, _> = serde_json::from_str(body);
    assert!(parsed.is_err(), "scheduleless duty must be rejected");
}

#[test]
fn slugify_entry_action_handles_punctuation_and_empty() {
    assert_eq!(slugify_entry_action("Send the report!"), "send_the_report");
    assert_eq!(slugify_entry_action("  Daily  Check-In  "), "daily_check_in");
    assert_eq!(slugify_entry_action("***"), "scheduled_duty");
    assert_eq!(slugify_entry_action(""), "scheduled_duty");
}
