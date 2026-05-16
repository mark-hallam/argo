use crate::{
    cu_step_runner::CuStepRunner,
    e2b::{CuAction, E2bClient, E2bSandbox},
    profile::Profile,
    restart::{RestartCoordinator, RestartStrategy},
    CuError,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn e2b_runner_matches_direct_client_step_sequence() {
    let server = MockServer::start().await;
    mount_screenshot(&server, "sb-1", b"PNG").await;
    Mock::given(method("POST"))
        .and(path("/sandboxes/sb-1/click"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": true, "text": "clicked"})),
        )
        .mount(&server)
        .await;

    let client = E2bClient::new(server.uri(), "k".into(), 5).expect("client");
    let sandbox = E2bSandbox { id: "sb-1".into() };
    let action = CuAction::Click { x: 10, y: 20 };

    let trait_png = client.warm_up(&sandbox).await.expect("trait screenshot");
    let trait_result = client
        .execute_step_recorded(&sandbox, &action)
        .await
        .expect("trait action");
    let direct_png = client
        .screenshot(&sandbox)
        .await
        .expect("direct screenshot");
    let direct_result = client
        .execute_action(&sandbox, &action)
        .await
        .expect("direct action");

    assert_eq!(trait_png, direct_png);
    assert_eq!(trait_result, direct_result);
    assert_eq!(
        received_paths(&server).await,
        vec![
            "/sandboxes/sb-1/screenshot",
            "/sandboxes/sb-1/click",
            "/sandboxes/sb-1/screenshot",
            "/sandboxes/sb-1/click"
        ]
    );
}

#[tokio::test]
async fn e2b_runner_recovery_uses_existing_restart_path() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/sandboxes/old"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/sandboxes"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"sandboxId": "new"})),
        )
        .mount(&server)
        .await;
    mount_screenshot(&server, "new", b"FRESH").await;

    let client = E2bClient::new(server.uri(), "k".into(), 5).expect("client");
    let profile = Profile {
        restart: RestartStrategy::SnapshotResetRetry {
            max_attempts: 1,
            snapshot_every_n_steps: 1,
        },
        ..Profile::default()
    };
    let mut coordinator = RestartCoordinator::new(profile.restart.clone());
    let mut session = E2bSandbox { id: "old".into() };
    let mut screenshot = b"OLD".to_vec();
    let mut prompt = "current".to_string();
    let err = CuError::HttpStatus {
        stage: "e2b/action",
        status: 502,
        body: "bad gateway".into(),
    };

    let handoff = client
        .recover_after_error(
            &err,
            "sandbox_action",
            &profile,
            &mut coordinator,
            &mut session,
            &mut screenshot,
            &mut prompt,
            "base",
        )
        .await
        .expect("recovery ok");

    assert!(handoff.is_none());
    assert_eq!(session.id, "new");
    assert_eq!(screenshot, b"FRESH");
}

async fn mount_screenshot(server: &MockServer, id: &str, png: &'static [u8]) {
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{id}/screenshot")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(png.to_vec()))
        .mount(server)
        .await;
}

async fn received_paths(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .expect("recorded requests")
        .iter()
        .map(|r| r.url.path().to_string())
        .collect()
}
