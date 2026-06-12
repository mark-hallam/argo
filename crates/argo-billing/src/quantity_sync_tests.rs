//! Tests for DEC-053 seat-quantity sync: the pure decision + form layer,
//! the active-teammate count query, and the Stripe write path against a
//! one-shot localhost mock (the "mock Stripe client" of the acceptance
//! criteria).

use super::*;
use crate::stripe_client::StripeConfig;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use chrono::Utc;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ---- pure decision + floor layer ----

#[test]
fn quantity_floors_at_one_while_subscribed() {
    // Acceptance: never below 1 while subscribed, even with every
    // teammate paused (active == 0).
    assert_eq!(desired_quantity(0), 1);
    assert_eq!(desired_quantity(1), 1);
    assert_eq!(desired_quantity(5), 5);
}

#[test]
fn hire_increments_pause_decrements_target() {
    // Acceptance: hire → quantity increments; pause → decrements.
    assert_eq!(plan_reconcile(2, 3).target, 3, "hire 3rd teammate");
    assert!(plan_reconcile(2, 3).needs_update());
    assert_eq!(plan_reconcile(3, 2).target, 2, "pause one teammate");
    assert!(plan_reconcile(3, 2).needs_update());
}

#[test]
fn in_sync_is_a_noop() {
    let d = plan_reconcile(4, 4);
    assert!(!d.needs_update());
    assert_eq!(d.target, 4);
}

#[test]
fn reconciliation_heals_injected_drift() {
    // Acceptance: reconciliation heals an injected drift. Stripe stuck
    // at 5 seats while only 2 teammates are active → heal down to 2.
    let drift = plan_reconcile(5, 2);
    assert!(drift.needs_update());
    assert_eq!(drift.target, 2);
    // And drift the other way (missed two hires) heals up.
    let up = plan_reconcile(1, 3);
    assert!(up.needs_update());
    assert_eq!(up.target, 3);
}

#[test]
fn update_form_carries_quantity_and_proration() {
    let form = quantity_update_form(4);
    assert!(form.contains(&("quantity".to_string(), "4".to_string())));
    assert!(form.contains(&(
        "proration_behavior".to_string(),
        "create_prorations".to_string()
    )));
}

#[test]
fn change_reason_slugs_are_stable() {
    assert_eq!(QuantityChangeReason::Hire.as_str(), "hire");
    assert_eq!(QuantityChangeReason::Pause.as_str(), "pause");
    assert_eq!(QuantityChangeReason::Unpause.as_str(), "unpause");
    assert_eq!(QuantityChangeReason::Delete.as_str(), "delete");
    assert_eq!(QuantityChangeReason::Reconcile.as_str(), "reconcile");
}

// ---- subscription-item parsing ----

#[test]
fn parse_seat_item_skips_the_metered_overage_item() {
    // A real Pro subscription carries TWO items: the licensed per-seat
    // price and the metered overage price. We must update the licensed
    // one and never touch the metered one.
    let sub = json!({
        "items": { "data": [
            { "id": "si_overage", "quantity": 0,
              "price": { "recurring": { "usage_type": "metered" } } },
            { "id": "si_seats", "quantity": 3,
              "price": { "recurring": { "usage_type": "licensed" } } }
        ]}
    });
    let seat = parse_seat_item(&sub).expect("licensed item resolved");
    assert_eq!(seat.item_id, "si_seats");
    assert_eq!(seat.quantity, 3);
}

#[test]
fn parse_seat_item_defaults_missing_usage_type_to_licensed() {
    let sub = json!({
        "items": { "data": [
            { "id": "si_seats", "quantity": 2, "price": {} }
        ]}
    });
    let seat = parse_seat_item(&sub).expect("licensed item resolved");
    assert_eq!(seat.item_id, "si_seats");
    assert_eq!(seat.quantity, 2);
}

#[test]
fn parse_seat_item_is_none_for_metered_only() {
    let sub = json!({
        "items": { "data": [
            { "id": "si_overage", "quantity": 0,
              "price": { "recurring": { "usage_type": "metered" } } }
        ]}
    });
    assert_eq!(parse_seat_item(&sub), None);
}

// ---- active-teammate count ----

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("memory store");
    store.migrate().await.expect("migrate");
    store
}

async fn hire(store: &SessionStore, tenant: TenantId, name: &str) -> Worker {
    let identities = IdentityStore::new(store);
    let worker = Worker::draft(
        tenant,
        "gtm",
        Persona::named(name, "op"),
        Role::new("Sell."),
    );
    identities.create_worker(&worker).await.expect("create worker");
    worker
}

#[tokio::test]
async fn count_excludes_archived_and_frozen() {
    // Acceptance support: the count that drives quantity excludes paused
    // (frozen) and deleted (archived) teammates.
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);

    let _keep = hire(&store, tenant, "Active").await;
    let frozen = hire(&store, tenant, "Paused").await;
    let archived = hire(&store, tenant, "Deleted").await;

    // Three hired → all three active to start.
    assert_eq!(count_active_teammates(&store, tenant).await.unwrap(), 3);

    // Pause one (open worker_freeze row).
    sqlx::query(
        "INSERT INTO worker_freeze (tenant_id, worker_id, frozen_at, frozen_by, reason) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(tenant.to_string())
    .bind(frozen.id.to_string())
    .bind(Utc::now())
    .bind("op")
    .bind("test pause")
    .execute(store.pool())
    .await
    .expect("freeze row");

    // Delete one (soft archive).
    identities
        .archive_worker(tenant, archived.id)
        .await
        .expect("archive");

    // Only the untouched teammate counts.
    assert_eq!(count_active_teammates(&store, tenant).await.unwrap(), 1);
}

#[tokio::test]
async fn count_is_tenant_scoped() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    hire(&store, tenant_a, "A1").await;
    hire(&store, tenant_a, "A2").await;
    hire(&store, tenant_b, "B1").await;
    assert_eq!(count_active_teammates(&store, tenant_a).await.unwrap(), 2);
    assert_eq!(count_active_teammates(&store, tenant_b).await.unwrap(), 1);
}

// ---- Stripe write path (one-shot localhost mock) ----

/// Bind a localhost listener that accepts ONE connection, captures the
/// raw request, and replies `200` with `response_json`. The join handle
/// yields the captured request so the test can assert what was POSTed.
async fn one_shot_stripe(response_json: &str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    let resp = response_json.to_string();
    let handle = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await.expect("read req");
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        let out = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            resp.len(),
            resp
        );
        sock.write_all(out.as_bytes()).await.expect("write resp");
        sock.flush().await.expect("flush");
        req
    });
    (base, handle)
}

fn mock_client(base_url: String) -> StripeClient {
    StripeClient::new(StripeConfig {
        secret_key: "sk_test_mock".to_string(),
        webhook_secret: "whsec_mock".to_string(),
        base_url,
    })
    .expect("client")
}

#[tokio::test]
async fn set_seat_quantity_posts_quantity_and_proration() {
    // Acceptance: the change reaches Stripe with the right quantity and
    // the documented proration behaviour.
    let (base, handle) =
        one_shot_stripe(r#"{"id":"si_seats","object":"subscription_item","quantity":3}"#).await;
    let client = mock_client(base);

    let resp = set_seat_quantity(&client, "si_seats", 3)
        .await
        .expect("set quantity");
    assert_eq!(resp.get("quantity").and_then(Value::as_u64), Some(3));

    let captured = handle.await.expect("join mock");
    assert!(
        captured.contains("POST /v1/subscription_items/si_seats"),
        "hit the item-update endpoint: {captured}"
    );
    assert!(captured.contains("quantity=3"), "carried quantity: {captured}");
    assert!(
        captured.contains("proration_behavior=create_prorations"),
        "carried proration: {captured}"
    );
}
