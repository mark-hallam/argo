//! Integration coverage for mission-pm-skill-overlay-load: a worker
//! that has overlay skills attached via `worker_skill` must see those
//! skill bodies in the system prompt on its next session.
//!
//! Shape matches `cross_tenant_skill_discovery_negative.rs`: stand up
//! a one-shot HTTP listener acting as an OpenAI-compatible provider,
//! drive one turn through `run_turn`, capture the outgoing system
//! prompt from the request body, and assert.

use argo_agent_runner::run_turn;
use argo_skill::SkillStore;
use argo_store::{SessionCreate, SessionStore, WorkerId};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const OVERLAY_BODY: &str = "per-worker-overlay-body-mission-pm-load";
const PROVIDER_ID: &str = "per-worker-overlay-test";
const MODEL_ID: &str = "per-worker-overlay-model";
const API_KEY_ENV: &str = "SKYCOT_PER_WORKER_OVERLAY_TEST_KEY";

fn skill_md(name: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"fixture overlay\"\n\
         provenance:\n  source: builtin\n  hash: sha256-x\n---\n{body}",
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

        let body = r#"{"id":"chatcmpl-test","model":"per-worker-overlay-model","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;
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
         display_name = \"Per-Worker Overlay Test\"\nbase_url = \"{base_url}\"\n\
         [auth]\napi_key_env = \"{API_KEY_ENV}\"\n[transport]\nstreaming = \"sse\"\n\
         [tool_calls]\nformat = \"openai_tool_calls\"\n[[models]]\nid = \"{MODEL_ID}\"\n"
    );
    std::fs::write(dir.join(format!("{PROVIDER_ID}.toml")), manifest).expect("write manifest");
}

#[tokio::test]
async fn worker_skill_overlay_body_lands_in_system_prompt() {
    let cwd = tempfile::tempdir().expect("cwd");
    let (base_url, rx, handle) = serve_openai_once();
    write_provider_manifest(cwd.path(), &base_url);
    unsafe {
        std::env::set_var(API_KEY_ENV, "sk-test");
    }

    let store = migrated_store(":memory:").await;
    let skills = SkillStore::new(Arc::clone(&store));

    // Install a workspace skill, then attach it to W1 via `worker_skill`
    // — the cron-side write path that
    // `load_prompt_skills_with_library` must now observe.
    let skill = skills
        .install(&skill_md("worker-overlay-skill", OVERLAY_BODY), None)
        .await
        .expect("install overlay skill");
    let worker = WorkerId::new();
    sqlx::query("INSERT INTO worker_skill (worker_id, skill_id, proficiency) VALUES (?, ?, 5)")
        .bind(worker.to_string())
        .bind(skill.id.to_string())
        .execute(store.pool())
        .await
        .expect("attach worker overlay");

    let (tenant, workspace) = store
        .ensure_workspace("per-worker-overlay", "/tmp/per-worker-overlay")
        .await
        .expect("workspace");
    let session = store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: workspace,
            model: MODEL_ID,
            provider: PROVIDER_ID,
            title: Some("per-worker-overlay-session"),
            vertical_id: None,
            worker_id: Some(worker),
        })
        .await
        .expect("session");

    run_turn(&store, cwd.path(), session, Some("hello"), false, 1)
        .await
        .expect("run turn");

    let raw = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("captured provider request");
    handle.join().expect("provider thread");
    let body: Value = serde_json::from_str(&raw).expect("request JSON");
    let messages = body["messages"].as_array().expect("messages array");
    assert_eq!(messages[0]["role"], "system");
    let system_prompt = messages[0]["content"]
        .as_str()
        .expect("system prompt string");

    assert!(
        system_prompt.contains(OVERLAY_BODY),
        "per-worker overlay body must surface in the system prompt:\n{system_prompt}",
    );
}
