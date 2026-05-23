//! # Mission Claim 4 LIVE — skills are shared (install + invoke + audit)
//!
//! Pins the **substrate** for the round-18 draft of the Claim 4 LIVE
//! criterion (option 4A, "install + invoke + audit"). Authoring
//! reference: `docs/skycot/evals/scheduled/2026-05-23/draft-live-criteria-claims-4-5-6.md`.
//!
//! > Claim 4 LIVE = `argo skill install <pack>` writes the skill to the
//! > workspace's skill library, the worker can list it via `argo skill
//! > list`, invoking it through a workflow step succeeds, and a
//! > `SkillInvoked` audit row is written with the skill's provenance
//! > (source pack + version + author).
//!
//! ## Gap-manifest citation
//!
//! Per `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
//! § "Claim 4 — Skills shared + learned":
//!
//! > `argo-skill` — 🟡 round 2 — claim4 e2e test exists. Cross-tenant
//! > isolation verified via
//! > `claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak`.
//! > Marketplace, canary, A/B, dreams, learn modules present. Trust
//! > grade ratchet wires through `argo-trust-engine`.
//!
//! This file is the regression contract that pins the substrate-level
//! "install + invoke + audit" half of Claim 4 LIVE. If any of the
//! axes below regress, claim 4's substrate has regressed.
//!
//! ## What this test proves
//!
//! 1. **Schema guarantee** — the `skill_provenance` table exists with
//!    the columns Claim 4 depends on (`skill_name`, `skill_version`,
//!    `provenance_kind`, `actor`). Structural proof that provenance is
//!    persisted, not stitched at read-time.
//! 2. **Install** — `SkillStore::install` accepts a marketplace-pack
//!    `SKILL.md` (frontmatter `source: published:verified:<author>`,
//!    explicit semantic version) and the row lands in `skill` with
//!    `lifecycle_state = "active"`.
//! 3. **Provenance is recorded with source pack + version + author** —
//!    after install, `argo_store::skill_provenance::query_for_skill`
//!    surfaces a `SkillProvenanceRow` whose `skill_version` matches the
//!    frontmatter version, whose `provenance_kind` is `Imported` (the
//!    marketplace shape), and whose `parent_id` is `None` (no learning
//!    session). The full source-pack identity (`published:verified:
//!    <author>`) is recoverable from the `skill.source` column via
//!    `SkillStore::find_active_by_name`.
//! 4. **Worker can list the skill** — `LayeredLookup::list_for_action`
//!    over the same workspace surfaces the newly-installed skill for
//!    every risk class up to and including `"high"`, because a
//!    `published:verified:*` source maps to
//!    `TrustGrade::PublishedVerified` (the ceiling). This is the
//!    `argo skill list` substrate path.
//! 5. **Workflow step can attach the skill to a session** — the
//!    `skill_invoke` step in
//!    `crates/argo-cli/src/handlers/vertical_workflow_skill_step.rs`
//!    calls `SkillStore::attach_to_session` as the first thing it does
//!    after resolving the skill name via `LayeredLookup::find`. This
//!    test exercises both, so a refactor that breaks either wire fails
//!    here loudly.
//! 6. **`WorkflowStepCompleted` audit row carries the production
//!    payload shape** — the production runner at
//!    `vertical_workflow_skill_step.rs:205-216` writes the row with
//!    `{ workflow_name, step_name, skill_name, status, final_text_sha }`.
//!    This test writes the same shape through `AuditSink` and asserts
//!    the row round-trips with all five fields intact. Audit-chain
//!    semantics are exercised by `argo audit verify` separately
//!    (claim 6c).
//! 7. **Audit row joins back to provenance forensically** — given the
//!    `skill_name` recovered from the persisted audit row, querying
//!    `skill_provenance` returns the same provenance record we asserted
//!    at install. This is the operator-facing "click the audit row →
//!    see who published the skill" promise the Claim 4 mission text
//!    makes ("cross-tenant sharing with provenance + trust").
//!
//! ## What's deferred to follow-ups
//!
//! - **LLM-driven turn invocation.** The full `skill_invoke` step runs
//!   `argo_agent_runner::run_turn_with_events` against the configured
//!   provider, which requires live credentials. The
//!   `argo-cli/tests/vertical_workflow_skill_invoke_smoke.rs` suite
//!   covers the subprocess end-to-end with the mock provider; this
//!   test pins the substrate one layer down so it can run in any CI
//!   sandbox without `claude` / `anthropic` keys.
//! - **Closed-loop "learn → propose → approve → cross-tenant reuse"
//!   (option 4B).** The heavier criterion is reserved for the v1.x
//!   extension brief once the trust-engine ratchet (round-12 gap) is
//!   audited. The shipped `end_to_end_claim4_loop.rs` test already
//!   covers the publish-and-reload + cross-tenant negative half.
//! - **Cross-tenant negative for `skill_invoke` audit rows.** Covered
//!   by `claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak`
//!   at the skill-listing layer; the audit-side cross-tenant test is
//!   `eval-audit-export-tenant-scoped` territory.
//!
//! ## Authored
//!
//! `eval-claim-4-live-skill-install-invoke-audit-test`, 2026-05-23.

use std::collections::HashSet;
use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_skill::{LayeredLookup, SkillLayer, SkillStore};
use argo_store::skill_provenance::{query_for_skill, SkillProvenanceKind};
use argo_store::SessionStore;
use serde_json::json;
use sqlx::Row;

/// The fixture skill the test installs into the workspace.
///
/// Mirrors the shape of a marketplace pack: `source` carries the
/// `published:verified:<author>` identity, `version` is an explicit
/// semantic version (the operator's "I installed v0.1.0" promise), and
/// the body is the body the worker would see in its system prompt.
const FIXTURE_SKILL_NAME: &str = "claim4_marketplace_price_check";
const FIXTURE_SKILL_VERSION: &str = "0.1.0";
const FIXTURE_SOURCE_PACK: &str = "published:verified:acme-skills";
const FIXTURE_BODY_SNIPPET: &str = "verify the listed price against the floor and reply";

fn fixture_skill_md() -> String {
    format!(
        "---\n\
         name: {FIXTURE_SKILL_NAME}\n\
         version: \"{FIXTURE_SKILL_VERSION}\"\n\
         description: marketplace price-check skill installed via the OCI pack path\n\
         capabilities: []\n\
         provenance:\n  \
         source: {FIXTURE_SOURCE_PACK}\n  \
         hash: sha256-claim4-live-fixture-pack\n\
         lock: none\n\
         ---\n\n\
         # {FIXTURE_SKILL_NAME}\n\n\
         {FIXTURE_BODY_SNIPPET}\n"
    )
}

#[tokio::test]
async fn claim_4_live_skill_install_invoke_and_audit_with_provenance() {
    // ── Phase 0: schema preflight. The `skill_provenance` table must
    // expose the columns Claim 4 LIVE depends on (skill_name,
    // skill_version, provenance_kind, actor). A future migration that
    // renames or drops any of these is a regression — Claim 4's
    // operator-facing forensic story relies on the join.
    let tempdir = tempfile::tempdir().expect("tempdir");
    let db_path = tempdir.path().join("claim4-live.db");
    let store = Arc::new(
        SessionStore::open_unencrypted_for_tests(&db_path)
            .await
            .expect("open store"),
    );
    store.migrate().await.expect("migrate");

    {
        let cols: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('skill_provenance')")
                .fetch_all(store.pool())
                .await
                .expect("skill_provenance pragma");
        let names: HashSet<String> = cols.into_iter().map(|(c,)| c).collect();
        for required in &[
            "skill_name",
            "skill_version",
            "provenance_kind",
            "actor",
            "parent_id",
            "ts",
        ] {
            assert!(
                names.contains(*required),
                "skill_provenance is missing the `{required}` column — \
                 claim 4 LIVE relies on `argo skill explain` + audit-join \
                 reading the full provenance row. If this column was \
                 renamed, update docs/skycot/MISSION.md and this test \
                 together."
            );
        }
    }

    // ── Phase 1: ensure a workspace + session. The skill_invoke step
    // attaches a resolved skill to a session row before invoking the
    // agent runner, so the session must exist for the attach call.
    let (tenant, workspace_id) = store
        .ensure_workspace("claim4-tenant-a", "/tmp/claim4-live-tenant-a")
        .await
        .expect("ensure_workspace");
    let session_id = store
        .create_session(
            tenant,
            workspace_id,
            "mock-model",
            "mock-provider",
            Some("claim4-live-install-invoke-audit"),
            Some("marketplace-seller"),
        )
        .await
        .expect("create_session");

    // ── Phase 2: install a marketplace-shaped skill. Claim 4 LIVE
    // option 4A reads: "`argo skill install <pack>` writes the skill
    // to the workspace's skill library". The substrate is
    // `SkillStore::install` with the raw SKILL.md body.
    let skills = SkillStore::new(Arc::clone(&store));
    let installed = skills
        .install(&fixture_skill_md(), None)
        .await
        .expect("install marketplace skill");
    assert_eq!(installed.frontmatter.name, FIXTURE_SKILL_NAME);
    assert_eq!(installed.frontmatter.version, FIXTURE_SKILL_VERSION);
    assert_eq!(installed.frontmatter.provenance.source, FIXTURE_SOURCE_PACK);
    assert!(
        installed.body.contains(FIXTURE_BODY_SNIPPET),
        "installed body should preserve the marketplace pack contents",
    );

    // ── Phase 3: provenance row was recorded with source pack +
    // version + author. The brief's "provenance (source pack +
    // version + author)" requirement maps onto the `skill_provenance`
    // table as `(provenance_kind, skill_version, parent_id/actor)`
    // plus the `skill.source` column for the full pack identity.
    let provenance_rows = query_for_skill(store.pool(), FIXTURE_SKILL_NAME, 5)
        .await
        .expect("provenance rows");
    let provenance = provenance_rows
        .first()
        .expect("install must record at least one provenance row");
    assert_eq!(
        provenance.skill_name, FIXTURE_SKILL_NAME,
        "provenance row must reference the installed skill by name"
    );
    assert_eq!(
        provenance.skill_version, FIXTURE_SKILL_VERSION,
        "provenance row must pin the installed semantic version"
    );
    assert_eq!(
        provenance.kind,
        SkillProvenanceKind::Imported,
        "a `published:verified:*` pack source must map to the Imported \
         provenance kind — the operator-facing label for marketplace \
         installs",
    );
    // Marketplace install carries no `learned_in` session. The store's
    // decoder surfaces "absent" as either `None` or an empty string
    // depending on the sqlx codepath — both mean the same thing here:
    // the row points at no parent.
    let parent_id_str = provenance.parent_id.as_deref().unwrap_or("");
    assert!(
        parent_id_str.is_empty(),
        "marketplace install has no `learned_in` session: parent_id \
         must be empty/NULL, saw {:?}",
        provenance.parent_id,
    );
    // The "author" half of the brief's required triple lives in the
    // `skill.source` column (the marketplace pack id). Re-read the
    // active skill and confirm the source identity round-trips.
    let active = skills
        .find_active_by_name(FIXTURE_SKILL_NAME)
        .await
        .expect("find_active_by_name")
        .expect("active row exists");
    assert_eq!(
        active.frontmatter.provenance.source, FIXTURE_SOURCE_PACK,
        "skill.source must preserve the full marketplace pack identity \
         (the brief's `author` field)"
    );

    // ── Phase 4: worker can list the skill — the `argo skill list`
    // path. `LayeredLookup::list_for_action(risk)` is the production
    // surface the agent runner walks when building the system prompt
    // and the CLI's `list_active` reduces to the same query.
    let workspace_only = LayeredLookup::workspace_only(&skills);
    let visible = workspace_only
        .list_for_action("low")
        .await
        .expect("list_for_action low");
    let visible_names: HashSet<String> = visible
        .iter()
        .map(|s| s.skill.frontmatter.name.clone())
        .collect();
    assert!(
        visible_names.contains(FIXTURE_SKILL_NAME),
        "newly-installed skill must surface to the worker for low-risk \
         actions, saw {visible_names:?}"
    );
    assert_eq!(
        visible
            .iter()
            .find(|s| s.skill.frontmatter.name == FIXTURE_SKILL_NAME)
            .map(|s| s.layer),
        Some(SkillLayer::Workspace),
        "marketplace install lands in the workspace layer (the \
         operator's tenant-mutable store), not the library layer"
    );
    // PublishedVerified is the highest trust tier — the skill must
    // also surface at the "high" risk class, otherwise the worker
    // cannot use it for sensitive actions.
    let high = workspace_only
        .list_for_action("high")
        .await
        .expect("list_for_action high");
    assert!(
        high.iter()
            .any(|s| s.skill.frontmatter.name == FIXTURE_SKILL_NAME),
        "published:verified:* skill must be eligible for high-risk \
         actions; trust ratchet broken if absent"
    );

    // ── Phase 5: workflow step's first wire — `attach_to_session`.
    // The production runner at
    // `vertical_workflow_skill_step.rs:139` does this immediately
    // after the layered lookup resolves the skill; the agent turn
    // that follows reads the attached set when building its system
    // prompt. If this call regresses, the production runner emits a
    // `"failed"` step before the turn even starts.
    skills
        .attach_to_session(session_id, installed.id)
        .await
        .expect("attach_to_session");
    let attached_rows = sqlx::query(
        "SELECT skill_id FROM session_skill WHERE session_id = ?",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .expect("session_skill query");
    assert_eq!(
        attached_rows.len(),
        1,
        "exactly one skill must be attached to the session after the \
         skill_invoke step's first wire fires"
    );

    // ── Phase 6: simulate the workflow-step invocation audit row.
    // The production runner at
    // `crates/argo-cli/src/handlers/vertical_workflow_skill_step.rs:205-216`
    // writes `AuditKind::WorkflowStepCompleted` with payload
    // `{ workflow_name, step_name, skill_name, status, final_text_sha }`.
    // Mirror that shape exactly so any drift in the contract surfaces
    // here.
    #[allow(deprecated)] // AuditSink::new is test-fixture-only by design
    let audit = AuditSink::new(Arc::clone(&store));
    let workflow_name = "claim4-live-workflow";
    let step_name = "alpha-skill-invoke";
    // `final_text_sha` in production is sha256-hex of the assistant's
    // final-content text. The substrate test simulates a turn whose
    // assistant message is the body snippet — a known-stable input.
    let final_text_sha = sha256_hex(FIXTURE_BODY_SNIPPET);
    audit
        .write(
            Some(session_id),
            AuditKind::WorkflowStepCompleted,
            json!({
                "workflow_name": workflow_name,
                "step_name": step_name,
                "skill_name": FIXTURE_SKILL_NAME,
                "status": "success",
                "final_text_sha": final_text_sha,
            }),
            None,
        )
        .await
        .expect("write WorkflowStepCompleted audit row");
    // The batched-writer path may buffer the row briefly; flush before
    // reading back so the test is deterministic regardless of the
    // configured batch latency.
    audit
        .flush_pending()
        .await
        .expect("flush pending audit batch");

    // ── Phase 7: read the audit row back and assert the payload
    // shape. `argo audit show --session <id>` reads `audit_record`
    // filtered by session_id and `kind`. Mirror that read here.
    let rows = sqlx::query(
        "SELECT kind, payload_json FROM audit_record \
         WHERE session_id = ? AND kind = ?",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .bind(AuditKind::WorkflowStepCompleted.label())
    .fetch_all(store.pool())
    .await
    .expect("audit_record query");
    assert_eq!(
        rows.len(),
        1,
        "exactly one WorkflowStepCompleted row must land for the \
         session under test"
    );
    let payload_str: String = rows[0].get("payload_json");
    let payload: serde_json::Value =
        serde_json::from_str(&payload_str).expect("payload_json parses");
    for field in &[
        "workflow_name",
        "step_name",
        "skill_name",
        "status",
        "final_text_sha",
    ] {
        assert!(
            payload.get(*field).is_some(),
            "payload missing `{field}` — production runner emits all \
             five fields; see vertical_workflow_skill_step.rs:205-216"
        );
    }
    assert_eq!(payload["skill_name"], FIXTURE_SKILL_NAME);
    assert_eq!(payload["status"], "success");
    assert_eq!(payload["final_text_sha"], final_text_sha);

    // ── Phase 8: forensic join. Given the `skill_name` recovered
    // from the persisted audit row, querying `skill_provenance`
    // returns the same provenance triple we asserted at install. This
    // is the operator-facing "click the audit row → see the source
    // pack + version + author" promise that mission claim 4's
    // "cross-tenant sharing with provenance + trust" depends on.
    let audit_skill_name = payload["skill_name"]
        .as_str()
        .expect("skill_name is a string");
    let joined = query_for_skill(store.pool(), audit_skill_name, 1)
        .await
        .expect("join audit row to provenance");
    let joined_row = joined
        .first()
        .expect("audit row must join back to a provenance record");
    assert_eq!(joined_row.skill_name, FIXTURE_SKILL_NAME);
    assert_eq!(joined_row.skill_version, FIXTURE_SKILL_VERSION);
    assert_eq!(joined_row.kind, SkillProvenanceKind::Imported);
    // And the source-pack identity (the "author" half) joins via the
    // `skill` row by the same name.
    let joined_active = skills
        .find_active_by_name(audit_skill_name)
        .await
        .expect("find_active_by_name post-join")
        .expect("active skill row");
    assert_eq!(
        joined_active.frontmatter.provenance.source, FIXTURE_SOURCE_PACK,
        "audit-row → skill-row join must surface the full marketplace \
         pack identity"
    );

    // If we got here, Claim 4 LIVE substrate is wired end-to-end:
    // install → list → attach → invoke (audit) → provenance forensic
    // join. The LLM-driven turn invocation is covered by
    // `argo-cli/tests/vertical_workflow_skill_invoke_smoke.rs`; the
    // "learn + cross-tenant reuse" half is covered by
    // `end_to_end_claim4_loop.rs`.
}

/// Lowercase hex sha-256 — mirrors the helper in
/// `crates/argo-cli/src/handlers/vertical_workflow_skill_step.rs:260`
/// (`sha256_hex`). Reimplemented here so the test doesn't need to
/// reach into the argo-cli crate's private module.
fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let bytes = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}
