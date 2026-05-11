//! Unit tests for [`super::skill_flag`].
#![cfg(test)]
use super::{get_flag, get_flag_or_default, set_flag, FLAG_NO_FORGE_FEEDBACK};
use crate::SessionStore;
use sqlx::{Pool, Sqlite};

async fn fresh_pool() -> Pool<Sqlite> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn set_then_get_returns_value() {
    let pool = fresh_pool().await;
    set_flag(&pool, "alpha", FLAG_NO_FORGE_FEEDBACK, 1)
        .await
        .unwrap();
    let v = get_flag(&pool, "alpha", FLAG_NO_FORGE_FEEDBACK).await.unwrap();
    assert_eq!(v, Some(1));
}

#[tokio::test]
async fn get_unknown_flag_returns_none() {
    let pool = fresh_pool().await;
    let v = get_flag(&pool, "alpha", FLAG_NO_FORGE_FEEDBACK).await.unwrap();
    assert!(v.is_none());
}

#[tokio::test]
async fn set_flag_upserts_value_on_conflict() {
    let pool = fresh_pool().await;
    set_flag(&pool, "alpha", FLAG_NO_FORGE_FEEDBACK, 1)
        .await
        .unwrap();
    set_flag(&pool, "alpha", FLAG_NO_FORGE_FEEDBACK, 0)
        .await
        .unwrap();
    let v = get_flag(&pool, "alpha", FLAG_NO_FORGE_FEEDBACK).await.unwrap();
    assert_eq!(v, Some(0), "second set_flag should overwrite, not append");
}

#[tokio::test]
async fn get_flag_or_default_falls_back_when_missing() {
    let pool = fresh_pool().await;
    let v = get_flag_or_default(&pool, "missing-skill", FLAG_NO_FORGE_FEEDBACK, 0)
        .await
        .unwrap();
    assert_eq!(v, 0);
}

#[tokio::test]
async fn flags_are_isolated_per_skill() {
    let pool = fresh_pool().await;
    set_flag(&pool, "alpha", FLAG_NO_FORGE_FEEDBACK, 1)
        .await
        .unwrap();
    // beta has no row.
    let v_beta = get_flag(&pool, "beta", FLAG_NO_FORGE_FEEDBACK).await.unwrap();
    assert!(v_beta.is_none());
    // alpha's row is still 1.
    let v_alpha = get_flag(&pool, "alpha", FLAG_NO_FORGE_FEEDBACK).await.unwrap();
    assert_eq!(v_alpha, Some(1));
}
