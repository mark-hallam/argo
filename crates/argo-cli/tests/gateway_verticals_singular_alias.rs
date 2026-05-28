//! Smoke coverage for the per-vertical detail alias:
//! `GET /api/v1/verticals/<id>` must return the same envelope as
//! the bare `GET /verticals/<id>`. Without the alias the api-
//! prefixed path 404s while the unprefixed path 200s — a silent
//! drift caught by the 2026-05-27 onboarding-hire-an-ai-teammate
//! UX audit (F9 [P1]).

mod common;

use common::listener_signal::await_listener_signal;
use common::{argo_bin, body_of, gateway_lock_guard, http_get, pick_free_port};
use std::path::Path;
use std::process::{Command, Stdio};

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn scaffold_vertical(parent: &Path) {
    let dir = parent.join("hello-vertical");
    std::fs::create_dir_all(dir.join("prompts")).expect("prompts dir");
    std::fs::write(
        dir.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"hello-vertical\"\nname = \"Hello vertical\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"prompts/system.md\"\n",
    )
    .expect("write vertical.toml");
    std::fs::write(
        dir.join("prompts/system.md"),
        "You are a friendly assistant.",
    )
    .expect("write system prompt");
}

#[test]
fn vertical_detail_api_prefix_mirrors_bare_path() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME tempdir");
    let verticals = tempfile::tempdir().expect("verticals tempdir");
    scaffold_vertical(verticals.path());
    if std::env::var_os("ARGO_TEST_LISTENER_DEADLINE_MS").is_none() {
        std::env::set_var("ARGO_TEST_LISTENER_DEADLINE_MS", "60000");
    }
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "4",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    // Both list variants 200; envelopes differ by design (bare is
    // on-disk pack discovery, api-prefixed is tenant-scoped DB), so
    // we only assert the contract — they answer the request.
    let bare_list = http_get(port, "/verticals");
    assert!(
        bare_list.contains("HTTP/1.1 200 OK"),
        "GET /verticals must 200: {bare_list}"
    );
    let api_list = http_get(port, "/api/v1/verticals");
    assert!(
        api_list.contains("HTTP/1.1 200 OK"),
        "GET /api/v1/verticals must 200: {api_list}"
    );

    // Singular detail: api-prefixed and bare must return byte-
    // identical bodies. This is the regression-pin for the F9 fix —
    // before the alias landed the api-prefixed path 404'd.
    let bare_show = http_get(port, "/verticals/hello-vertical");
    assert!(
        bare_show.contains("HTTP/1.1 200 OK"),
        "GET /verticals/<id> must 200: {bare_show}"
    );
    let api_show = http_get(port, "/api/v1/verticals/hello-vertical");
    assert!(
        api_show.contains("HTTP/1.1 200 OK"),
        "GET /api/v1/verticals/<id> must 200 (this is the alias under test): {api_show}"
    );
    let bare_body: serde_json::Value =
        serde_json::from_str(body_of(&bare_show)).expect("bare detail json");
    let api_body: serde_json::Value =
        serde_json::from_str(body_of(&api_show)).expect("api detail json");
    assert_eq!(
        bare_body, api_body,
        "singular detail envelopes must match: bare={bare_body} api={api_body}"
    );
    assert_eq!(bare_body["id"], "hello-vertical");

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
