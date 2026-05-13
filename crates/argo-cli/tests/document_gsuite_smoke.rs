//! Env-gated smoke test for the Google Workspace executor.
//!
//! Skipped unless **both** env vars are set:
//!
//! - `ARGO_TEST_GSUITE_SERVICE_ACCOUNT_EMAIL` —
//!   `<sa>@<project>.iam.gserviceaccount.com` of a Workspace
//!   service-account with the four scopes
//!   (`drive`, `documents`, `spreadsheets`, `presentations`) enabled.
//! - `ARGO_TEST_GSUITE_SERVICE_ACCOUNT_KEY` — PEM body of the matching
//!   RS256 private key (paste the whole
//!   `-----BEGIN PRIVATE KEY-----…END-----` block).
//!
//! Optional: `ARGO_TEST_GSUITE_DRIVE_FOLDER_ID` to root drafts under a
//! tenant folder; absent ⇒ service-account's "My Drive" root.
//!
//! The test creates one of each draft kind (Doc / Sheet / Slides),
//! verifies the typed report fields (`doc_id`, `web_view_link`,
//! `mime_type`, `sha256_of_request`), then trashes the three drafts
//! via the Drive `files.update` `trashed=true` field-mask so the
//! test account doesn't accumulate clutter.
//!
//! Run locally with:
//! ```bash
//! ARGO_TEST_GSUITE_SERVICE_ACCOUNT_EMAIL='argo@<proj>.iam.gserviceaccount.com' \
//!   ARGO_TEST_GSUITE_SERVICE_ACCOUNT_KEY="$(cat sa-key.pem)" \
//!   cargo test -p argo-cli --test document_gsuite_smoke -- --ignored --nocapture
//! ```

use argo_executor_document_gsuite::{
    DocumentGsuite, GsuiteAuthConfig, GsuiteDocKind, GsuiteSlideSpec, GsuiteWriteRequest,
};

fn skip_unless_env_set() -> Option<(String, String, Option<String>)> {
    let email = std::env::var("ARGO_TEST_GSUITE_SERVICE_ACCOUNT_EMAIL").ok()?;
    let key = std::env::var("ARGO_TEST_GSUITE_SERVICE_ACCOUNT_KEY").ok()?;
    if email.is_empty() || key.is_empty() {
        return None;
    }
    let folder = std::env::var("ARGO_TEST_GSUITE_DRIVE_FOLDER_ID")
        .ok()
        .filter(|s| !s.is_empty());
    Some((email, key, folder))
}

fn auth_from_env(email: String, key: String) -> GsuiteAuthConfig {
    GsuiteAuthConfig {
        client_email: email,
        private_key_pem: key,
        token_uri: "https://oauth2.googleapis.com/token".into(),
        scopes: concat!(
            "https://www.googleapis.com/auth/drive ",
            "https://www.googleapis.com/auth/documents ",
            "https://www.googleapis.com/auth/spreadsheets ",
            "https://www.googleapis.com/auth/presentations",
        )
        .into(),
    }
}

async fn trash(doc_id: &str, auth: &GsuiteAuthConfig) {
    // Best-effort cleanup; failures are not the test's concern. We
    // re-mint a fresh client + token so the trash call doesn't
    // depend on cached state from the write path.
    let api =
        argo_executor_document_gsuite::auth::GsuiteAuth::new(auth.clone());
    let Ok(token) = api.acquire_token().await else {
        return;
    };
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{doc_id}?fields=id"
    );
    let client = reqwest::Client::new();
    let _ = client
        .patch(&url)
        .bearer_auth(token)
        .json(&serde_json::json!({ "trashed": true }))
        .send()
        .await;
}

#[tokio::test]
#[ignore = "env-gated; requires ARGO_TEST_GSUITE_SERVICE_ACCOUNT_EMAIL+KEY"]
async fn smoke_create_doc_sheet_slides_against_live_api() {
    let Some((email, key, folder)) = skip_unless_env_set() else {
        eprintln!(
            "skip: ARGO_TEST_GSUITE_SERVICE_ACCOUNT_EMAIL / \
             ARGO_TEST_GSUITE_SERVICE_ACCOUNT_KEY unset"
        );
        return;
    };
    let auth = auth_from_env(email, key);
    let exec = DocumentGsuite::new(auth.clone());

    let mk = |kind: GsuiteDocKind, title: &str| GsuiteWriteRequest {
        kind,
        title: title.into(),
        drive_folder_id: folder.clone(),
        doc_body_text: matches!(kind, GsuiteDocKind::Doc).then(|| "Smoke draft.".into()),
        sheet_schema: None,
        sheet_rows: vec![],
        slides: if matches!(kind, GsuiteDocKind::Slides) {
            vec![GsuiteSlideSpec {
                title: "Smoke deck".into(),
                body: "One slide.".into(),
                image_url: None,
            }]
        } else {
            vec![]
        },
    };

    let doc_report = exec
        .write(mk(GsuiteDocKind::Doc, "argo-smoke-doc"))
        .await
        .expect("doc write");
    assert!(!doc_report.doc_id.0.is_empty());
    assert!(doc_report
        .web_view_link
        .starts_with("https://docs.google.com/"));
    assert_eq!(doc_report.kind, GsuiteDocKind::Doc);
    assert_eq!(doc_report.sha256_of_request.len(), 64);

    let sheet_report = exec
        .write(mk(GsuiteDocKind::Sheet, "argo-smoke-sheet"))
        .await
        .expect("sheet write");
    assert!(!sheet_report.doc_id.0.is_empty());
    assert_eq!(sheet_report.kind, GsuiteDocKind::Sheet);

    let slides_report = exec
        .write(mk(GsuiteDocKind::Slides, "argo-smoke-slides"))
        .await
        .expect("slides write");
    assert!(!slides_report.doc_id.0.is_empty());
    assert_eq!(slides_report.kind, GsuiteDocKind::Slides);

    // Cleanup — best-effort.
    trash(&doc_report.doc_id.0, &auth).await;
    trash(&sheet_report.doc_id.0, &auth).await;
    trash(&slides_report.doc_id.0, &auth).await;
}
