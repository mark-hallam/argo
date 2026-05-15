//! Cross-tenant skill discovery regression test for marketplace-library
//! prompt loading. Tenant-private library skills must not leak into another
//! worker prompt, while published marketplace skills still load.

use argo_agent_runner::run_turn;
use argo_skill::SkillStore;
use argo_store::SessionStore;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const PRIVATE_BODY: &str = "tenant-a-private-body";
const PUBLISHED_BODY: &str = "published-marketplace-body";
const PROVIDER_ID: &str = "cross-tenant-test";
const MODEL_ID: &str = "cross-tenant-model";
const API_KEY_ENV: &str = "SKYCOT_CROSS_TENANT_TEST_KEY";

fn skill_md(name: &str, source: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"fixture\"\n\
         provenance:\n  source: {source}\n  hash: sha256-x\n---\n{body}",
    )
}

async fn migrated_store(path: impl AsRef<Path>) -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(path).await.expect("open store"));
    store.migrate().await.expect("migrate store");
    store
}

fn serve_openai_once() -> (String, Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock provider");
    let addr = listener.local_addr().expect("provider addr");
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept provider request");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read header");
            if line == "\r\n" {
                break;
            }
            headers.push_str(&line);
        }
        let len = parse_content_length(&headers).expect("content-length");
        let mut body_bytes = vec![0_u8; len];
        reader.read_exact(&mut body_bytes).expect("read body");
        tx.send(String::from_utf8(body_bytes).expect("utf8 body"))
            .expect("send request body");

        let body = r#"{"id":"chatcmpl-test","model":"cross-tenant-model","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });
    (format!("http://{addr}"), rx, handle)
}

fn parse_content_length(headers: &str) -> Option<usize> {
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            value.trim().parse().ok()
        } else {
            None
        }
    })
}

fn write_provider_manifest(cwd: &Path, base_url: &str) {
    let dir = cwd.join("manifests/providers");
    std::fs::create_dir_all(&dir).expect("mkdir manifests");
    let manifest = format!(
        "[provider]\nid = \"{PROVIDER_ID}\"\nfamily = \"openai-compatible\"\n\
         display_name = \"Cross Tenant Test\"\nbase_url = \"{base_url}\"\n\
         [auth]\napi_key_env = \"{API_KEY_ENV}\"\n[transport]\nstreaming = \"sse\"\n\
         [tool_calls]\nformat = \"openai_tool_calls\"\n[[models]]\nid = \"{MODEL_ID}\"\n"
    );
    std::fs::write(dir.join(format!("{PROVIDER_ID}.toml")), manifest).expect("write manifest");
}

async fn captured_system_prompt(store: &Arc<SessionStore>, cwd: &Path, label: &str) -> String {
    let (base_url, rx, handle) = serve_openai_once();
    write_provider_manifest(cwd, &base_url);
    let (tenant, workspace) = store
        .ensure_workspace(label, &format!("/tmp/{label}"))
        .await
        .expect("workspace");
    let session = store
        .create_session(tenant, workspace, MODEL_ID, PROVIDER_ID, Some(label), None)
        .await
        .expect("session");

    run_turn(store, cwd, session, Some("hello"), false, 1)
        .await
        .expect("run turn");

    let raw = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("captured provider request");
    handle.join().expect("provider thread");
    let body: Value = serde_json::from_str(&raw).expect("request JSON");
    let messages = body["messages"].as_array().expect("messages array");
    assert_eq!(messages[0]["role"], "system");
    messages[0]["content"]
        .as_str()
        .expect("system prompt string")
        .to_string()
}

#[tokio::test]
async fn tenant_private_library_skill_does_not_enter_other_tenant_prompt() {
    let cwd = tempfile::tempdir().expect("cwd");
    let library_dir = tempfile::tempdir().expect("library dir");
    let library_path = library_dir.path().join("library.sqlite");

    let library_store = migrated_store(&library_path).await;
    let library = SkillStore::new(Arc::clone(&library_store));
    library
        .install(&skill_md("tenant-a-private", "user", PRIVATE_BODY), None)
        .await
        .expect("install private library skill");
    library
        .install(
            &skill_md(
                "published-followup",
                "published:verified:vendor",
                PUBLISHED_BODY,
            ),
            None,
        )
        .await
        .expect("install published library skill");

    unsafe {
        std::env::set_var("ARGO_SKILL_LIBRARY_DB", &library_path);
        std::env::set_var(API_KEY_ENV, "sk-test");
    }

    let tenant_a_store = migrated_store(":memory:").await;
    let tenant_a_skills = SkillStore::new(Arc::clone(&tenant_a_store));
    tenant_a_skills
        .install(&skill_md("tenant-a-private", "user", PRIVATE_BODY), None)
        .await
        .expect("install tenant A workspace skill");
    let tenant_a_prompt = captured_system_prompt(&tenant_a_store, cwd.path(), "tenant-a").await;
    assert!(
        tenant_a_prompt.contains(PRIVATE_BODY),
        "positive control: tenant A should see its own workspace skill:\n{tenant_a_prompt}"
    );

    let tenant_b_store = migrated_store(":memory:").await;
    let tenant_b_prompt = captured_system_prompt(&tenant_b_store, cwd.path(), "tenant-b").await;
    assert!(
        tenant_b_prompt.contains(PUBLISHED_BODY),
        "sanity check: tenant B should load published marketplace skills:\n{tenant_b_prompt}"
    );
    assert!(
        !tenant_b_prompt.contains(PRIVATE_BODY),
        "negative check: tenant B must not see tenant A's private skill:\n{tenant_b_prompt}"
    );
}
