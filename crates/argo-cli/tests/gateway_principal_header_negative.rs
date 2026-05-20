//! Negative tests for the principal/tenant header cross-check
//! (`mission-audit-principal-tenant-from-token`, P0).
//!
//! Boots the gateway with `ARGO_GATEWAY_BEARER_TOKEN` set + an
//! optional pre-seeded `service_token_binding` row, then exercises
//! `/api/v1/skills/.../ir` to assert:
//!  1. bound token + matching principal/tenant headers → 200
//!  2. bound token + mismatched `X-Argo-Principal` → 403
//!  3. bound token + mismatched `X-Argo-Tenant-Id` → 403
//!  4. unbound token + caller-supplied principal header → 403
//!  5. no `Authorization` header (token configured) → 401

mod common;

use argo_skill::ir::{
    ensure_schema as ensure_skill_ir_schema, Node, NodeAnnotations, NodeId, NodeKind, SkillGraph,
    SkillIrStore, SqliteSkillIrStore,
};
use argo_store::service_token_binding::insert_binding;
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use common::pick_free_port;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, ChildStderr, Command, Stdio};
use std::time::Duration;

const BOUND_TOKEN: &str = "test-bound-token-zzzz";
const UNBOUND_TOKEN: &str = "test-unbound-token-yyyy";
const PRINCIPAL: &str = "human:alice@example.com";
const TENANT_HEX: &str = "11111111-1111-7111-9111-111111111111";

fn sha256_hex(s: &str) -> String {
    let digest = Sha256::digest(s.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

async fn seed_skill_ir(home: &Path, skill: SkillId, tenant: TenantId) {
    let db = home.join("sessions.db");
    let store = SessionStore::open(&db).await.expect("open store");
    store.migrate().await.expect("migrate");
    ensure_skill_ir_schema(&store).await.expect("ensure skill_ir");
    let ir = SqliteSkillIrStore::new(&store);
    ir.put(
        tenant,
        skill,
        "v1",
        &SkillGraph {
            entry: NodeId::from("n1"),
            nodes: vec![Node {
                id: NodeId::from("n1"),
                label: "test".into(),
                kind: NodeKind::Action {
                    instruction: "noop".into(),
                },
                annotations: NodeAnnotations::default(),
            }],
            edges: vec![],
            variables: vec![],
            metadata: Default::default(),
        },
    )
    .await
    .expect("put ir");
}

async fn seed_binding(home: &Path, token: &str, principal: &str, tenant: TenantId) {
    let db = home.join("sessions.db");
    let store = SessionStore::open(&db).await.expect("open store");
    store.migrate().await.expect("migrate");
    insert_binding(
        store.pool(),
        &sha256_hex(token),
        principal,
        &tenant.to_string(),
        Some("test"),
        "2026-05-20T00:00:00Z",
    )
    .await
    .expect("insert binding");
}

fn boot_gateway(home: &Path, port: u16, token: &str) -> (Child, BufReader<ChildStderr>) {
    let mut child = Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_GATEWAY_BEARER_TOKEN", token)
        .env("ANTHROPIC_API_KEY", "sk-ant-gateway-principal-header-negative")
        .env_remove("ARGO_GATEWAY_AUTH_BYPASS")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "16",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let mut line = String::new();
    loop {
        if std::time::Instant::now() > deadline {
            panic!("listener did not announce in 30s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }
    (child, stderr)
}

struct Req<'a> {
    auth: Option<&'a str>,
    principal: Option<&'a str>,
    tenant: Option<&'a str>,
}

fn http_get(port: u16, path: &str, req: Req<'_>) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut s = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    if let Some(v) = req.auth {
        s.push_str(&format!("Authorization: Bearer {v}\r\n"));
    }
    if let Some(v) = req.principal {
        s.push_str(&format!("X-Argo-Principal: {v}\r\n"));
    }
    if let Some(v) = req.tenant {
        s.push_str(&format!("X-Argo-Tenant-Id: {v}\r\n"));
    }
    s.push_str("\r\n");
    stream.write_all(s.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn status_line(resp: &str) -> &str {
    resp.lines().next().unwrap_or("")
}

/// Cases (1), (2), (3), (5) — exercised against a gateway whose
/// bearer token is bound to `(PRINCIPAL, TENANT_HEX)` in the
/// `service_token_binding` table.
#[tokio::test]
async fn bound_token_matrix() {
    let _lock = common::gateway_lock_guard();
    let home = tempfile::tempdir().unwrap();
    let port = pick_free_port();
    let tenant = TenantId(uuid::Uuid::parse_str(TENANT_HEX).unwrap());
    let skill = SkillId::new();
    seed_skill_ir(home.path(), skill, tenant).await;
    seed_binding(home.path(), BOUND_TOKEN, PRINCIPAL, tenant).await;
    let (mut child, _stderr) = boot_gateway(home.path(), port, BOUND_TOKEN);
    let path = format!("/api/v1/skills/{skill}/ir?version=v1");

    // (1) bound token + matching principal+tenant headers → 200
    let r = http_get(
        port,
        &path,
        Req {
            auth: Some(BOUND_TOKEN),
            principal: Some(PRINCIPAL),
            tenant: Some(TENANT_HEX),
        },
    );
    assert!(
        status_line(&r).contains("200 OK"),
        "(1) matching: {}",
        status_line(&r)
    );

    // (2) bound token + mismatched principal → 403
    let r = http_get(
        port,
        &path,
        Req {
            auth: Some(BOUND_TOKEN),
            principal: Some("human:imposter@example.com"),
            tenant: Some(TENANT_HEX),
        },
    );
    assert!(
        status_line(&r).contains("403 Forbidden"),
        "(2) bad principal: {}",
        status_line(&r)
    );

    // (3) bound token + mismatched tenant → 403
    let r = http_get(
        port,
        &path,
        Req {
            auth: Some(BOUND_TOKEN),
            principal: Some(PRINCIPAL),
            tenant: Some("99999999-9999-7999-8999-999999999999"),
        },
    );
    assert!(
        status_line(&r).contains("403 Forbidden"),
        "(3) bad tenant: {}",
        status_line(&r)
    );

    // (5) configured token but no Authorization header → 401
    let r = http_get(
        port,
        &path,
        Req {
            auth: None,
            principal: None,
            tenant: None,
        },
    );
    assert!(
        status_line(&r).contains("401 Unauthorized"),
        "(5) no auth: {}",
        status_line(&r)
    );

    let _ = child.kill();
    let _ = child.wait();
}

/// Case (4) — separate gateway boot where the configured bearer
/// token has NO `service_token_binding` row. The bearer passes the
/// gate, the gateway derives a synthetic `service:<fp>` principal,
/// and a caller-supplied principal header that doesn't equal the
/// fingerprint is rejected with `403`.
#[tokio::test]
async fn unbound_token_rejects_supplied_principal() {
    let _lock = common::gateway_lock_guard();
    let home = tempfile::tempdir().unwrap();
    let port = pick_free_port();
    let tenant = TenantId::new();
    let skill = SkillId::new();
    seed_skill_ir(home.path(), skill, tenant).await;
    // Deliberately do NOT bind UNBOUND_TOKEN.
    let (mut child, _stderr) = boot_gateway(home.path(), port, UNBOUND_TOKEN);
    let path = format!("/api/v1/skills/{skill}/ir?version=v1");

    let r = http_get(
        port,
        &path,
        Req {
            auth: Some(UNBOUND_TOKEN),
            principal: Some("human:claimed-by-caller@example.com"),
            tenant: None,
        },
    );
    assert!(
        status_line(&r).contains("403 Forbidden"),
        "(4) unbound + supplied principal: {}",
        status_line(&r)
    );

    let _ = child.kill();
    let _ = child.wait();
}
