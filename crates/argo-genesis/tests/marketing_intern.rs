//! Integration test for the brief's acceptance criterion:
//! "feed a charter ('be my marketing intern, run weekly
//! campaigns to cold leads'), assert the generated vertical
//! loads + passes lint + the action vocabulary contains zero
//! autonomous-safe money-touching actions".
//!
//! Runs offline — no API keys needed, no network calls. The
//! deterministic offline pipeline always returns the same
//! result for a given charter, so the assertion set is stable.

use argo_genesis::{plan, write_vertical, Archetype};

#[test]
fn marketing_intern_charter_passes_safety_judge_and_round_trip_load() {
    // 1. Plan from the brief's exact example charter.
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
    )
    .expect("marketing intern charter plans cleanly");
    assert_eq!(p.archetype, Archetype::MarketingIntern);

    // 2. Write to a tempdir.
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write succeeds");

    // 3. Round-trip load via the standard loader. No errors.
    let (registry, errors) = argo_vertical::load_dir(dir.path()).expect("load_dir");
    assert!(
        errors.is_empty(),
        "load errors must be empty; got: {errors:?}"
    );
    let loaded = registry.get(&p.vertical_id).expect("vertical loaded");
    let actions = loaded
        .manifest
        .actions
        .as_ref()
        .expect("actions section present");

    // 4. Brief acceptance: zero autonomous-safe actions match
    //    a sensitive verb. The safety judge already enforces
    //    this; the integration test asserts it end-to-end.
    for (name, decl) in &actions.actions {
        if argo_genesis::SENSITIVE_VERBS
            .iter()
            .any(|verb| name.split('_').any(|tok| tok.eq_ignore_ascii_case(verb)))
        {
            assert!(
                decl.requires_approval,
                "{name} matches a sensitive verb but is autonomous-safe — \
                 the brief forbids this for any newly-generated worker"
            );
        }
    }

    // 5. The marketing-intern vocab specifically includes
    //    `post_campaign` and `send_followup` — both must be
    //    requires_approval=true.
    assert!(
        actions.actions["post_campaign"].requires_approval,
        "post_campaign must require approval"
    );
    assert!(
        actions.actions["send_followup"].requires_approval,
        "send_followup must require approval"
    );

    // 6. Per the brief, the worker ships in dry_run = true.
    let worker_body =
        std::fs::read_to_string(out.join("worker.toml")).expect("worker.toml present");
    assert!(
        worker_body.contains("dry_run = true"),
        "worker.toml must ship with dry_run = true"
    );
    assert!(
        worker_body.contains("dry_run_until"),
        "worker.toml must carry the dry_run_until timestamp"
    );
}
