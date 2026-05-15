//! Unit tests for [`super::skill_provenance`].
#![cfg(test)]

use super::{query_for_skill, record_provenance, NewSkillProvenance, SkillProvenanceKind};
use crate::SessionStore;
use sqlx::{Pool, Sqlite};

async fn fresh_pool() -> Pool<Sqlite> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn record_and_query_round_trips() {
    let pool = fresh_pool().await;
    record_provenance(
        &pool,
        NewSkillProvenance {
            skill_name: "alpha",
            skill_version: "0.1",
            kind: SkillProvenanceKind::OperatorAuthored,
            actor: Some("operator"),
            parent_id: None,
            change_reason: Some("created"),
            breaking_change: false,
        },
    )
    .await
    .unwrap();

    let rows = query_for_skill(&pool, "alpha", 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, SkillProvenanceKind::OperatorAuthored);
    assert_eq!(rows[0].actor.as_deref(), Some("operator"));
}

#[tokio::test]
async fn second_record_replaces_same_version() {
    let pool = fresh_pool().await;
    for kind in [
        SkillProvenanceKind::Bootstrap,
        SkillProvenanceKind::ForgePromoted,
    ] {
        record_provenance(
            &pool,
            NewSkillProvenance {
                skill_name: "alpha",
                skill_version: "0.1",
                kind,
                actor: None,
                parent_id: Some("0.0"),
                change_reason: None,
                breaking_change: false,
            },
        )
        .await
        .unwrap();
    }

    let rows = query_for_skill(&pool, "alpha", 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, SkillProvenanceKind::ForgePromoted);
}

#[tokio::test]
async fn query_limit_is_honored() {
    let pool = fresh_pool().await;
    for version in ["0.1", "0.2", "0.3"] {
        record_provenance(
            &pool,
            NewSkillProvenance {
                skill_name: "alpha",
                skill_version: version,
                kind: SkillProvenanceKind::Bootstrap,
                actor: None,
                parent_id: None,
                change_reason: None,
                breaking_change: false,
            },
        )
        .await
        .unwrap();
    }

    let rows = query_for_skill(&pool, "alpha", 2).await.unwrap();
    assert_eq!(rows.len(), 2);
}
