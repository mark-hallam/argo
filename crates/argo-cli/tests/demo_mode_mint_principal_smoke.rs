//! Integration smoke for the env-var-gated demo-mode magic-link
//! bypass (`demo-ops-add-magic-link-bypass-for-recording`).
//!
//! Three scenarios:
//!
//! 1. **Off.** Without `ARGO_DEMO_MODE=1`, `POST /api/v1/demo/mint-principal`
//!    falls through to the default 404 — the dispatcher never mounts
//!    the demo route.
//! 2. **On + bypass.** With `ARGO_DEMO_MODE=1` + `ARGO_GATEWAY_AUTH_BYPASS=1`,
//!    the route mints a JSON envelope including
//!    `principal: "human:<email>"` and `session_token` (a real JWT),
//!    and the gateway writes an `auth.succeeded` audit row whose
//!    payload contains `demo_mode_bypass: true`.
//! 3. **On without bypass.** With `ARGO_DEMO_MODE=1` but no
//!    `ARGO_GATEWAY_AUTH_BYPASS`, the startup-check refuses to bind
//!    and exits with the documented message.
mod common;

use common::{listener_signal::await_listener_signal, pick_free_port};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn http_post(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn status_line(resp: &str) -> &str {
    resp.lines().next().unwrap_or("")
}

fn body_of(resp: &str) -> &str {
    resp.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

/// A fixed all-zero 32-byte SQLCipher key (base64). Supplying it via
/// `SKYCOT_DB_KEY_BASE64` makes the gateway derive its store key from
/// the env var instead of the OS keyring (`boot::open_store_at` →
/// `get_or_create_db_key`). On macOS the keyring path can block the
/// boot indefinitely under contention / a locked keychain, which
/// otherwise flakes these gateway-spawn smoke tests as
/// `listener did not announce`. The DB is a throwaway tempdir, so a
/// constant test key is fine.
const TEST_DB_KEY_BASE64: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

fn spawn_gateway(home: &Path, port: u16, demo_mode: bool) -> Child {
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home)
        .env("OPENAI_API_KEY", "test-key")
        .env("ARGO_GATEWAY_AUTH_BYPASS", "1")
        .env("SKYCOT_DB_KEY_BASE64", TEST_DB_KEY_BASE64)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if demo_mode {
        cmd.env("ARGO_DEMO_MODE", "1");
    } else {
        cmd.env_remove("ARGO_DEMO_MODE");
    }
    cmd.spawn().expect("spawn")
}

/// Pull the `session_token` JWT out of a demo-mint JSON response body.
fn session_token_of(resp: &str) -> String {
    let body = body_of(resp);
    let v: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("mint body not JSON ({e}): {body}"));
    v.get("session_token")
        .and_then(|t| t.as_str())
        .unwrap_or_else(|| panic!("mint body missing session_token: {body}"))
        .to_string()
}

/// Decode the (unverified) `tnt` (tenant id) claim from a JWT's payload
/// segment. The smoke test only needs the claim value, not a signature
/// check — the production verify path is covered by the crate's unit
/// tests. Returns `None` when the claim is absent.
fn jwt_tnt_claim(jwt: &str) -> Option<String> {
    use base64::Engine;
    let payload = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("tnt").and_then(|t| t.as_str()).map(str::to_string)
}

async fn read_demo_audit_payloads(home: &Path) -> Vec<String> {
    let store = argo_store::SessionStore::open(home.join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let rows = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT kind, payload_json FROM audit_record WHERE kind = 'auth.succeeded' ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    rows.into_iter().filter_map(|(_, p)| p).collect()
}

#[test]
fn demo_route_returns_404_when_flag_unset() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = spawn_gateway(home.path(), port, false);
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let resp = http_post(
        port,
        "/api/v1/demo/mint-principal",
        r#"{"email":"demo@throwaway.cc"}"#,
    );
    assert!(
        resp.contains("HTTP/1.1 404 Not Found"),
        "expected 404 with demo mode off, got: {}",
        status_line(&resp),
    );

    let _ = child.wait();
}

#[test]
fn demo_route_mints_principal_and_writes_audit_when_flag_set() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = spawn_gateway(home.path(), port, true);
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let resp = http_post(
        port,
        "/api/v1/demo/mint-principal",
        r#"{"email":"demo@throwaway.cc"}"#,
    );
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "mint failed: {}",
        status_line(&resp),
    );
    let body = body_of(&resp);
    assert!(
        body.contains(r#""ok":true"#),
        "mint body missing ok=true: {body}",
    );
    assert!(
        body.contains(r#""principal":"human:demo@throwaway.cc""#),
        "mint body missing principal: {body}",
    );
    assert!(
        body.contains(r#""session_token":""#),
        "mint body missing session_token: {body}",
    );

    let _ = child.wait();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let payloads = rt.block_on(read_demo_audit_payloads(home.path()));
    assert!(
        payloads
            .iter()
            .any(|p| p.contains("demo_mode_bypass") && p.contains("true")),
        "no auth.succeeded row with demo_mode_bypass=true: {payloads:?}",
    );
    assert!(
        payloads.iter().any(|p| p.contains("demo@throwaway.cc")),
        "no auth.succeeded row stamped with email: {payloads:?}",
    );
}

/// Tenant-isolation fix (`fix-demo-mint-tenant-isolation`): two
/// demo-mint principals with different emails must each get their OWN
/// provisioned tenant — the minted JWT carries a per-principal `tnt`
/// (tenant id) claim, which `resolve_tenant_for_request` honours over
/// the shared cwd fallback. A repeat mint of the same email reuses its
/// tenant (idempotent) rather than minting a third.
///
/// Spawns the gateway once per request (the smoke harness uses
/// `--max-requests 1`), all against ONE `ARGO_HOME` so the three mints
/// share state. Asserts directly on the `tnt` claim decoded from each
/// returned session JWT — the value the resolver keys tenant on — so
/// the proof is independent of internal table layout.
#[test]
fn two_demo_principals_get_isolated_tenants() {
    let home = tempfile::tempdir().unwrap();

    let mint = |email: &str| -> String {
        let port = pick_free_port();
        let mut child = spawn_gateway(home.path(), port, true);
        let stderr = child.stderr.take().expect("stderr piped");
        let _drain = await_listener_signal(stderr).expect("gateway listener signal");
        let resp = http_post(
            port,
            "/api/v1/demo/mint-principal",
            &format!(r#"{{"email":"{email}"}}"#),
        );
        let _ = child.wait();
        assert!(
            resp.contains("HTTP/1.1 200 OK"),
            "mint of {email} failed: {}",
            status_line(&resp),
        );
        session_token_of(&resp)
    };

    let alice_tnt = jwt_tnt_claim(&mint("alice@throwaway.cc"))
        .expect("alice's demo JWT must carry a tnt claim — the isolation fix");
    let bob_tnt = jwt_tnt_claim(&mint("bob@throwaway.cc"))
        .expect("bob's demo JWT must carry a tnt claim — the isolation fix");
    // Re-mint alice: must reuse her existing tenant (idempotent).
    let alice_tnt_again = jwt_tnt_claim(&mint("alice@throwaway.cc"))
        .expect("alice's re-mint must still carry a tnt claim");

    assert!(!alice_tnt.is_empty(), "alice tnt must be non-empty");
    assert!(!bob_tnt.is_empty(), "bob tnt must be non-empty");
    assert_ne!(
        alice_tnt, bob_tnt,
        "two demo principals must resolve to DIFFERENT tenants — the isolation fix",
    );
    assert_eq!(
        alice_tnt, alice_tnt_again,
        "re-minting the same demo email must reuse its tenant (idempotent provision)",
    );
}

#[test]
fn gateway_refuses_to_bind_when_demo_mode_set_without_bypass() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("OPENAI_API_KEY", "test-key")
        .env("ARGO_DEMO_MODE", "1")
        .env_remove("ARGO_GATEWAY_AUTH_BYPASS")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    assert!(
        !child.status.success(),
        "gateway should have refused to bind",
    );
    let stderr = String::from_utf8_lossy(&child.stderr);
    assert!(
        stderr.contains("ARGO_DEMO_MODE=1 cannot be used in production"),
        "stderr missing demo-mode refusal: {stderr}",
    );
    assert!(
        stderr.contains("ARGO_GATEWAY_AUTH_BYPASS=1"),
        "stderr missing remediation hint: {stderr}",
    );
}
