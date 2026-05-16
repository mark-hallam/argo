//! Integration tests for [`crate::user_store`].

use super::{get_user, get_user_by_email, insert_user, update_last_login};
use crate::user::{UserAccount, UserId};
use argo_store::SessionStore;
use chrono::{TimeZone, Utc};

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn insert_and_get_by_email_round_trips() {
    let store = store().await;
    let u = UserAccount::new("alice@acme.com".into());
    insert_user(&store, &u).await.unwrap();
    let fetched = get_user_by_email(&store, "alice@acme.com")
        .await
        .unwrap()
        .expect("row");
    assert_eq!(fetched, u);
}

#[tokio::test]
async fn get_by_id_returns_same_user() {
    let store = store().await;
    let u = UserAccount::new("alice@acme.com".into());
    insert_user(&store, &u).await.unwrap();
    assert_eq!(
        get_user(&store, u.id).await.unwrap().expect("row"),
        u,
    );
}

#[tokio::test]
async fn duplicate_email_is_rejected() {
    let store = store().await;
    let a = UserAccount::new("alice@acme.com".into());
    insert_user(&store, &a).await.unwrap();
    let b = UserAccount::new("alice@acme.com".into());
    let err = insert_user(&store, &b).await.expect_err("uniqueness");
    assert!(err.to_string().to_lowercase().contains("unique"), "got: {err}");
}

#[tokio::test]
async fn update_last_login_sets_timestamp() {
    let store = store().await;
    let u = UserAccount::new("alice@acme.com".into());
    insert_user(&store, &u).await.unwrap();
    assert!(u.last_login_at.is_none());
    let now = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    let affected = update_last_login(&store, u.id, now).await.unwrap();
    assert_eq!(affected, 1);
    let row = get_user(&store, u.id).await.unwrap().expect("row");
    assert_eq!(row.last_login_at, Some(now.timestamp()));
}

#[tokio::test]
async fn update_last_login_on_missing_id_returns_zero() {
    let store = store().await;
    let affected = update_last_login(&store, UserId::new(), Utc::now())
        .await
        .unwrap();
    assert_eq!(affected, 0);
}

#[tokio::test]
async fn get_missing_user_returns_none() {
    let store = store().await;
    assert!(get_user_by_email(&store, "ghost@acme.com")
        .await
        .unwrap()
        .is_none());
    assert!(get_user(&store, UserId::new()).await.unwrap().is_none());
}
