//! Tests for `routes_tenant_theme` — the per-tenant white-label theme
//! override.
//!
//! Drives `dispatch_get` / `dispatch_patch` directly against a `:memory:`
//! `SessionStore` (migrated, so the v85 `workspace_settings` table — which
//! also backs the `theme` group — exists). The upstream bearer-auth gate
//! lives in `routes.rs` and is not re-exercised here.
//!
//! Two tenant-resolution paths are exercised:
//!   - cwd-resolved (no `?tenant=`) for the default + round-trip tests — a
//!     process-wide mutex serialises the cwd-dependent resolution.
//!   - `?tenant=<uuid>`-resolved for the isolation test, so tenant A and
//!     tenant B resolve to genuinely distinct ids on the no-session path.

use super::{dispatch_get, dispatch_patch};
use argo_store::SessionStore;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn get_req(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn patch_req(path: &str, body: &Value) -> String {
    let body = body.to_string();
    format!("PATCH {path} HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\n\r\n{body}")
}

fn patch_req_empty(path: &str) -> String {
    format!("PATCH {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn parse_ok(resp: Option<(&'static str, String)>) -> Value {
    let (status, body) = resp.expect("route matched");
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

async fn audit_count(store: &SessionStore, kind: &str) -> i64 {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM audit_record WHERE kind = ?")
        .bind(kind)
        .fetch_one(store.pool())
        .await
        .expect("count audit rows");
    row.get::<i64, _>("n")
}

const THEME: &str = "/api/v1/tenant/theme";

#[tokio::test]
async fn get_returns_locked_skycot_default_when_no_override() {
    let _g = env_lock().await;
    let store = mem_store().await;
    let v = parse_ok(dispatch_get(&get_req(THEME), Some(&store)).await);
    // With NO override set the brand is unchanged: the locked SKYCOT
    // default accent, and `is_override = false` tells the SPA to keep the
    // shipped brand tokens.
    assert_eq!(v["accent_hex"], "#8B5CF6");
    assert_eq!(v["neutral_tint"], "cool");
    assert_eq!(v["radius"], "default");
    assert_eq!(v["is_override"], false);
}

#[tokio::test]
async fn patch_then_get_round_trips_the_override() {
    let _g = env_lock().await;
    let store = mem_store().await;

    let patched = parse_ok(
        dispatch_patch(
            &patch_req(
                THEME,
                &json!({ "accent_hex": "#1FAA59", "neutral_tint": "warm" }),
            ),
            Some(&store),
        )
        .await,
    );
    assert_eq!(patched["accent_hex"], "#1FAA59");
    assert_eq!(patched["neutral_tint"], "warm");
    assert_eq!(patched["is_override"], true);

    // A fresh GET reads the persisted override back, merged over the
    // default (untouched `radius` keeps the default).
    let got = parse_ok(dispatch_get(&get_req(THEME), Some(&store)).await);
    assert_eq!(got["accent_hex"], "#1FAA59");
    assert_eq!(got["neutral_tint"], "warm");
    assert_eq!(got["radius"], "default");
    assert_eq!(got["is_override"], true);
}

#[tokio::test]
async fn patch_appends_one_audit_row() {
    let _g = env_lock().await;
    let store = mem_store().await;
    assert_eq!(audit_count(&store, "tenant_theme_update").await, 0);
    let _ = dispatch_patch(
        &patch_req(THEME, &json!({ "accent_hex": "#5B6BFF" })),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(audit_count(&store, "tenant_theme_update").await, 1);
}

#[tokio::test]
async fn tenant_a_patch_never_changes_tenant_b() {
    // No env_lock needed: `?tenant=<uuid>` resolves directly to the given
    // tenant on the no-session path, so neither request touches cwd.
    let store = mem_store().await;
    let a = "11111111-1111-1111-1111-111111111111";
    let b = "22222222-2222-2222-2222-222222222222";

    // Tenant A sets a white-label accent.
    let patched = parse_ok(
        dispatch_patch(
            &patch_req(
                &format!("{THEME}?tenant={a}"),
                &json!({ "accent_hex": "#EC4899" }),
            ),
            Some(&store),
        )
        .await,
    );
    assert_eq!(patched["accent_hex"], "#EC4899");

    // Tenant B was never written: GET returns B's own (locked default),
    // never A's accent.
    let b_theme = parse_ok(dispatch_get(&get_req(&format!("{THEME}?tenant={b}")), Some(&store)).await);
    assert_eq!(b_theme["accent_hex"], "#8B5CF6");
    assert_eq!(b_theme["is_override"], false);

    // Tenant A still reads its own override back.
    let a_theme = parse_ok(dispatch_get(&get_req(&format!("{THEME}?tenant={a}")), Some(&store)).await);
    assert_eq!(a_theme["accent_hex"], "#EC4899");
    assert_eq!(a_theme["is_override"], true);
}

#[tokio::test]
async fn patch_rejects_non_hex_accent() {
    let _g = env_lock().await;
    let store = mem_store().await;
    for bad in [json!("blue"), json!("#12g"), json!("#1234567"), json!(42)] {
        let (status, body) = dispatch_patch(
            &patch_req(THEME, &json!({ "accent_hex": bad })),
            Some(&store),
        )
        .await
        .expect("matched");
        assert_eq!(status, "400 Bad Request", "accepted bad accent {bad}");
        assert!(body.contains("accent_hex"), "body: {body}");
    }
    // Nothing persisted: still the locked default.
    let after = parse_ok(dispatch_get(&get_req(THEME), Some(&store)).await);
    assert_eq!(after["is_override"], false);
}

#[tokio::test]
async fn patch_rejects_bad_enum_and_unknown_field() {
    let _g = env_lock().await;
    let store = mem_store().await;

    let (status, _) = dispatch_patch(
        &patch_req(THEME, &json!({ "neutral_tint": "neon" })),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status, "400 Bad Request");

    let (status, _) = dispatch_patch(
        &patch_req(THEME, &json!({ "radius": "huge" })),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status, "400 Bad Request");

    let (status, body) = dispatch_patch(
        &patch_req(THEME, &json!({ "bogus": "x" })),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("unknown field"), "body: {body}");
}

#[tokio::test]
async fn empty_patch_is_noop_and_stays_default() {
    let _g = env_lock().await;
    let store = mem_store().await;
    let v = parse_ok(dispatch_patch(&patch_req_empty(THEME), Some(&store)).await);
    // An empty patch writes no meaningful fields, so `is_override` stays
    // false on both the PATCH echo and a follow-up GET — the locked brand
    // is never flipped to "white-label" by a no-op write.
    assert_eq!(v["is_override"], false);
    let got = parse_ok(dispatch_get(&get_req(THEME), Some(&store)).await);
    assert_eq!(got["is_override"], false);
    assert_eq!(got["accent_hex"], "#8B5CF6");
}

#[tokio::test]
async fn non_matching_path_returns_none() {
    let _g = env_lock().await;
    let store = mem_store().await;
    assert!(dispatch_get(&get_req("/api/v1/tenant/usage"), Some(&store))
        .await
        .is_none());
    assert!(dispatch_patch(&patch_req("/api/v1/workspace", &json!({})), Some(&store))
        .await
        .is_none());
}
