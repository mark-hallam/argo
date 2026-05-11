//! Integration smoke test for `argo_store::skill_rating`: write a
//! rating row through the public surface and read it back via
//! `aggregate_ratings` + `recent_ratings`. Mirrors the brief's
//! "1 integration smoke (rate + read-back)" acceptance criterion
//! (Stream `mission-skill-ratings-publisher`).

use argo_store::skill_rating::{
    aggregate_ratings, list_for_publisher, recent_ratings, record_rating, SkillVote,
};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};

#[tokio::test]
async fn rate_and_read_back_round_trip() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    let skill = SkillId::new();
    let alice = TenantId::new();
    let bob = TenantId::new();

    record_rating(
        store.pool(),
        skill,
        "1.0.0",
        "acme",
        alice,
        SkillVote::Up,
        Some("great"),
    )
    .await
    .unwrap();
    record_rating(
        store.pool(),
        skill,
        "1.0.0",
        "acme",
        bob,
        SkillVote::Down,
        None,
    )
    .await
    .unwrap();

    let (up, down, score) = aggregate_ratings(store.pool(), skill, "1.0.0")
        .await
        .unwrap();
    assert_eq!(up, 1);
    assert_eq!(down, 1);
    assert!((score - 0.5).abs() < 1e-5);

    let rows = recent_ratings(store.pool(), skill, "1.0.0", 10).await.unwrap();
    assert_eq!(rows.len(), 2);

    let pub_rows = list_for_publisher(store.pool(), "acme").await.unwrap();
    assert_eq!(pub_rows.len(), 1);
    assert_eq!(pub_rows[0].publisher_tenant, "acme");
    assert_eq!(pub_rows[0].up_count, 1);
    assert_eq!(pub_rows[0].down_count, 1);
}
