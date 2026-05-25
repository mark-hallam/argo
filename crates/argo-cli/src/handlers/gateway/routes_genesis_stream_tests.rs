//! Unit tests for `routes_genesis_stream` (`POST /api/v1/genesis`).
//! Drives the handler over a loopback TCP pair so the full HTTP/1.1
//! and SSE wire format is exercised end-to-end. Each test owns its
//! own `ARGO_VERTICALS_DIR` tempdir so concurrent runs do not fight
//! over generated vertical paths. Env vars are process-global, so a
//! single mutex serializes mutations across tests.
use super::*;
use argo_store::SessionStore;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    keys: Vec<(&'static str, Option<String>)>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn set(entries: &[(&'static str, &str)]) -> Self {
        let guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut keys = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            keys.push((*k, std::env::var(k).ok()));
            std::env::set_var(k, v);
        }
        Self { keys, _guard: guard }
    }

    fn unset(entries: &[&'static str]) -> Self {
        let guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut keys = Vec::with_capacity(entries.len());
        for k in entries {
            keys.push((*k, std::env::var(k).ok()));
            std::env::remove_var(k);
        }
        Self { keys, _guard: guard }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, old) in &self.keys {
            match old {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn build_request(body: &str, tenant: Option<&str>) -> String {
    let tenant_hdr = tenant
        .map(|t| format!("X-Argo-Tenant-Id: {t}\r\n"))
        .unwrap_or_default();
    format!(
        "POST /api/v1/genesis HTTP/1.1\r\nHost: x\r\n{tenant_hdr}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn drive_handler(req: String, store: Arc<SessionStore>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut server_side, _) = listener.accept().await.unwrap();
        serve_genesis_stream(&req, Some(&store), &mut server_side)
            .await
            .unwrap();
    });
    let mut client = TcpStream::connect(addr).await.unwrap();
    // Half-close after the request body is not required — the
    // handler doesn't read; it inspects the pre-built `&str` we
    // passed in. We just need the connection alive to read the
    // response.
    let mut buf = Vec::new();
    let n = tokio::time::timeout(Duration::from_secs(10), client.read_to_end(&mut buf))
        .await
        .unwrap()
        .unwrap();
    buf.truncate(n);
    server.await.unwrap();
    String::from_utf8(buf).unwrap()
}

#[tokio::test]
async fn fails_closed_without_provider_key() {
    let _env = EnvGuard::unset(&[PROVIDER_KEY_ENV]);
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets",
    })
    .to_string();
    let req = build_request(&body, Some("tenant-x"));
    let raw = drive_handler(req, store).await;
    assert!(
        raw.starts_with("HTTP/1.1 503 Service Unavailable"),
        "expected 503 fail-closed; got: {raw}"
    );
    assert!(raw.contains("genesis_provider_key_missing"));
}

#[tokio::test]
async fn rejects_short_role_description() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ]);
    let store = mem_store().await;
    let body = serde_json::json!({ "role_description": "too short" }).to_string();
    let req = build_request(&body, Some("tenant-x"));
    let raw = drive_handler(req, store).await;
    assert!(raw.starts_with("HTTP/1.1 400 Bad Request"), "got: {raw}");
    assert!(raw.contains("role_description"));
}

#[tokio::test]
async fn rejects_malformed_json_body() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ]);
    let store = mem_store().await;
    let req = build_request("{not valid json", Some("tenant-x"));
    let raw = drive_handler(req, store).await;
    assert!(raw.starts_with("HTTP/1.1 400 Bad Request"), "got: {raw}");
    assert!(raw.contains("bad_request"));
}

#[tokio::test]
async fn happy_path_emits_sse_stream_with_vertical_ready() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ]);
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets and escalates refunds over $500",
        "persona": "Casey",
    })
    .to_string();
    let req = build_request(&body, Some("11111111-1111-1111-1111-111111111111"));
    let raw = drive_handler(req, store).await;

    assert!(raw.starts_with("HTTP/1.1 200 OK"), "got: {raw}");
    let lower = raw.to_lowercase();
    assert!(
        lower.contains("content-type: text/event-stream"),
        "expected text/event-stream content-type; got: {raw}"
    );
    // Progress phases must all appear, in order.
    for phase in [
        "\"phase\":\"drafting_persona\"",
        "\"phase\":\"compiling_charter\"",
        "\"phase\":\"generating_skill_bundle\"",
        "\"phase\":\"validating_eval_seeds\"",
        "\"phase\":\"ready\"",
    ] {
        assert!(raw.contains(phase), "expected phase {phase}; got: {raw}");
    }
    // The final event is `vertical_ready` with a payload pointing
    // at the on-disk vertical dir.
    let ready_idx = raw.find("event: vertical_ready").expect("vertical_ready event");
    let after_ready = &raw[ready_idx..];
    assert!(after_ready.contains("\"vertical_dir_path\":"));
    assert!(after_ready.contains("\"eval_seed_count\":"));
    assert!(after_ready.contains("\"judge_verdict\":\"pass\""));
}

#[tokio::test]
async fn cross_tenant_paths_isolate_generated_verticals() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ]);
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a marketing intern who writes weekly newsletters for B2B SaaS",
        "persona": "Maya",
    })
    .to_string();

    let tenant_a = "11111111-1111-1111-1111-111111111111";
    let tenant_b = "22222222-2222-2222-2222-222222222222";
    let raw_a = drive_handler(build_request(&body, Some(tenant_a)), Arc::clone(&store)).await;
    let raw_b = drive_handler(build_request(&body, Some(tenant_b)), Arc::clone(&store)).await;
    assert!(raw_a.starts_with("HTTP/1.1 200 OK"));
    assert!(raw_b.starts_with("HTTP/1.1 200 OK"));

    // Each tenant's vertical_dir_path lives under its own sanitized
    // tenant subdirectory; load_dir scoped to one tenant must not see
    // the other.
    let genesis_root = tmp.path().join(".genesis");
    let tenant_a_dir = genesis_root.join(tenant_a);
    let tenant_b_dir = genesis_root.join(tenant_b);
    assert!(tenant_a_dir.is_dir(), "tenant A subdir created");
    assert!(tenant_b_dir.is_dir(), "tenant B subdir created");
    let (reg_a, errs_a) = argo_vertical::load_dir(&tenant_a_dir).unwrap();
    let (reg_b, errs_b) = argo_vertical::load_dir(&tenant_b_dir).unwrap();
    assert!(errs_a.is_empty(), "tenant A load errors: {errs_a:?}");
    assert!(errs_b.is_empty(), "tenant B load errors: {errs_b:?}");
    assert_eq!(reg_a.len(), 1);
    assert_eq!(reg_b.len(), 1);
    let id_a: Vec<&str> = reg_a.list().map(|(id, _)| id).collect();
    let id_b: Vec<&str> = reg_b.list().map(|(id, _)| id).collect();
    // Same vertical_id (same role + persona) but at different
    // tenant-scoped paths — neither registry can observe the other.
    assert_eq!(id_a, id_b, "same vertical id under both tenants");
    assert_ne!(
        reg_a.get(id_a[0]).unwrap().root,
        reg_b.get(id_b[0]).unwrap().root,
        "tenant filesystem roots differ",
    );
}

#[test]
fn is_streaming_request_matches_post_only() {
    assert!(is_genesis_streaming_request(
        "POST /api/v1/genesis HTTP/1.1\r\n\r\n"
    ));
    assert!(is_genesis_streaming_request(
        "POST /api/v1/genesis?dry=1 HTTP/1.1\r\n\r\n"
    ));
    // The legacy sub-routes (`classify`, `draft-worker`) keep their
    // own handlers and must not be swallowed by the streaming check.
    assert!(!is_genesis_streaming_request(
        "POST /api/v1/genesis/classify HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_genesis_streaming_request(
        "POST /api/v1/genesis/draft-worker HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_genesis_streaming_request(
        "GET /api/v1/genesis HTTP/1.1\r\n\r\n"
    ));
}

#[test]
fn sanitize_tenant_normalises_special_chars() {
    assert_eq!(sanitize_tenant("AAAA-BBBB"), "aaaa-bbbb");
    assert_eq!(sanitize_tenant("..::weird::.."), "weird");
    assert_eq!(sanitize_tenant(""), "anonymous");
    assert_eq!(sanitize_tenant("///"), "anonymous");
}

#[test]
fn derive_persona_falls_back_to_operator_on_noise() {
    assert_eq!(derive_persona("!!! ??? ###"), "operator");
    assert_eq!(derive_persona("a b c"), "operator"); // tokens too short
    let derived = derive_persona("Marketing intern needed asap");
    assert_eq!(derived, "Marketing");
}
