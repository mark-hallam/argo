//! Integration coverage for `claude-skill-canary-deployment`:
//! DB-backed `CanaryStore` + the `pick_canary_for_skill` /
//! `load_canary_body` helpers the runner's `apply_canary_routing`
//! drives. Picker math is in `argo-skill::canary_tests`.

use argo_audit::{AuditKind, AuditSink};
use argo_skill::{
    load_canary_body, new_canary_config, pick_canary_for_skill, seed_for_session_skill,
    CanaryConfig, CanaryStore, PrincipalOverride, SkillStore,
};
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

fn skill_doc(name: &str, version: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: d\n\
         lock: none\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\n{body}"
    )
}

async fn install_two_versions(skills: &SkillStore) {
    skills
        .install(&skill_doc("demo", "v1", "BASELINE-BODY"), None)
        .await
        .unwrap();
    skills
        .supersede(&skill_doc("demo", "v2", "CANARY-BODY"), None, PrincipalOverride::Force)
        .await
        .unwrap();
}

// Mirrors the runner's per-skill loop (turn::apply_canary_routing)
// for one skill, so the integration test exercises the full
// pick → audit → body-swap flow end-to-end.
async fn drive_one(
    canary: &CanaryStore,
    skills: &SkillStore,
    audit: &AuditSink,
    sid: SessionId,
    name: &str,
    now: i64,
) -> Option<String> {
    let seed = seed_for_session_skill(sid, name);
    let d = pick_canary_for_skill(canary, name, seed, now).await.unwrap()?;
    audit
        .write(
            Some(sid),
            AuditKind::RoutingDecision,
            serde_json::json!({
                "kind": "canary_pick",
                "skill_id": name,
                "baseline_version": d.baseline_version,
                "canary_version": d.canary_version,
                "chosen_version": d.chosen_version,
                "traffic_percent": d.traffic_percent,
                "seed_hash": d.seed_hash,
            }),
            None,
        )
        .await
        .unwrap();
    if d.chose_canary {
        Some(load_canary_body(skills, name, &d.chosen_version).await.unwrap().unwrap())
    } else {
        None
    }
}

#[tokio::test]
async fn store_set_get_clear_round_trip() {
    let store = fresh_store().await;
    let canary = CanaryStore::new(Arc::clone(&store));
    let cfg = new_canary_config("demo", "v1", "v2", 25).unwrap();
    canary.set(&cfg).await.unwrap();

    let got = canary.get("demo").await.unwrap().expect("config present");
    assert_eq!(got.skill_id, "demo");
    assert_eq!(got.baseline_version, "v1");
    assert_eq!(got.canary_version, "v2");
    assert_eq!(got.traffic_percent, 25);

    let mut bumped = cfg.clone();
    bumped.traffic_percent = 75;
    canary.set(&bumped).await.unwrap();
    assert_eq!(canary.get("demo").await.unwrap().unwrap().traffic_percent, 75);

    assert_eq!(canary.clear("demo").await.unwrap(), 1);
    assert!(canary.get("demo").await.unwrap().is_none());
}

#[tokio::test]
async fn store_set_rejects_identical_versions() {
    let store = fresh_store().await;
    let canary = CanaryStore::new(Arc::clone(&store));
    let bad = CanaryConfig {
        skill_id: "demo".into(),
        baseline_version: "v1".into(),
        canary_version: "v1".into(),
        traffic_percent: 10,
        started_at: 0,
        expires_at: None,
    };
    let err = canary.set(&bad).await.unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::Schema(_)));
}

#[tokio::test]
async fn store_list_returns_all_rows_sorted() {
    let store = fresh_store().await;
    let canary = CanaryStore::new(Arc::clone(&store));
    canary
        .set(&new_canary_config("zeta", "v1", "v2", 5).unwrap())
        .await
        .unwrap();
    canary
        .set(&new_canary_config("alpha", "v1", "v2", 5).unwrap())
        .await
        .unwrap();
    let all = canary.list().await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].skill_id, "alpha");
    assert_eq!(all[1].skill_id, "zeta");
}

#[tokio::test]
async fn pick_returns_none_when_no_canary_configured() {
    let store = fresh_store().await;
    let canary = CanaryStore::new(Arc::clone(&store));
    let sid = SessionId(uuid::Uuid::nil());
    let d = pick_canary_for_skill(&canary, "demo", seed_for_session_skill(sid, "demo"), 100)
        .await
        .unwrap();
    assert!(d.is_none());
}

#[tokio::test]
async fn pick_returns_none_when_expired() {
    let store = fresh_store().await;
    let canary = CanaryStore::new(Arc::clone(&store));
    let mut cfg = new_canary_config("demo", "v1", "v2", 100).unwrap();
    cfg.started_at = 100;
    cfg.expires_at = Some(200);
    canary.set(&cfg).await.unwrap();
    let sid = SessionId(uuid::Uuid::nil());
    // now=300 > expires_at=200
    let d = pick_canary_for_skill(&canary, "demo", seed_for_session_skill(sid, "demo"), 300)
        .await
        .unwrap();
    assert!(d.is_none());
}

#[tokio::test]
async fn runner_swaps_body_at_full_traffic() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    install_two_versions(&skills).await;
    let canary = CanaryStore::new(Arc::clone(&store));
    let audit = AuditSink::new(Arc::clone(&store));
    canary
        .set(&new_canary_config("demo", "v1", "v2", 100).unwrap())
        .await
        .unwrap();
    let sid = SessionId(uuid::Uuid::from_bytes([7_u8; 16]));
    let body = drive_one(&canary, &skills, &audit, sid, "demo", 100).await;
    assert_eq!(body.as_deref(), Some("CANARY-BODY"));
    assert_eq!(audit.count_for(sid).await.unwrap(), 1);
}

#[tokio::test]
async fn runner_keeps_baseline_body_at_zero_percent_but_audits() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    install_two_versions(&skills).await;
    let canary = CanaryStore::new(Arc::clone(&store));
    let audit = AuditSink::new(Arc::clone(&store));
    canary
        .set(&new_canary_config("demo", "v1", "v2", 0).unwrap())
        .await
        .unwrap();
    let sid = SessionId(uuid::Uuid::from_bytes([8_u8; 16]));
    let body = drive_one(&canary, &skills, &audit, sid, "demo", 100).await;
    // 0% means baseline — runner keeps existing body (drive_one returns None).
    assert!(body.is_none());
    // ...but the routing decision is still recorded so analytics
    // can attribute baseline traffic to the active canary's denominator.
    assert_eq!(audit.count_for(sid).await.unwrap(), 1);
}

#[tokio::test]
async fn audit_event_carries_canary_pick_kind_in_payload() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    install_two_versions(&skills).await;
    let canary = CanaryStore::new(Arc::clone(&store));
    let audit = AuditSink::new(Arc::clone(&store));
    let mut rx = audit.subscribe();
    canary
        .set(&new_canary_config("demo", "v1", "v2", 100).unwrap())
        .await
        .unwrap();
    let sid = SessionId(uuid::Uuid::from_bytes([9_u8; 16]));
    drive_one(&canary, &skills, &audit, sid, "demo", 100).await;
    let evt = rx.recv().await.unwrap();
    assert_eq!(evt.kind, AuditKind::RoutingDecision.label());
    assert_eq!(evt.payload["kind"], "canary_pick");
    assert_eq!(evt.payload["skill_id"], "demo");
    assert_eq!(evt.payload["chosen_version"], "v2");
    assert_eq!(evt.payload["traffic_percent"], 100);
}

#[tokio::test]
async fn load_body_returns_none_when_version_missing() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    skills
        .install(&skill_doc("demo", "v1", "BASELINE"), None)
        .await
        .unwrap();
    let absent = load_canary_body(&skills, "demo", "v2").await.unwrap();
    assert!(absent.is_none());
}
