//! End-to-end integration test for `vision_analyze` with a
//! stub [`VisionDispatcher`]. Covers the brief's
//! "1 integration: end-to-end with a stub provider proving
//! the content block is shaped per provider." Per-provider
//! image-block shaping (Anthropic image-block, OpenAI
//! image_url, Bedrock Claude vision) lives in the provider
//! crates and is wired in a follow-up brief — out of scope
//! for `argo-tool`. This test asserts the tool hands a
//! provider-shaped [`ResolvedImage`] (bytes+mime for local
//! files, passthrough URL for remote) to its dispatcher.
use argo_tool::{ResolvedImage, Tool, ToolContext, Vision, VisionAnswer, VisionDispatcher};
use argo_types::{ContentBlock, Principal};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct CaptureDispatcher {
    provider: String,
    captured: Mutex<Option<(ResolvedImage, String)>>,
}

#[async_trait]
impl VisionDispatcher for CaptureDispatcher {
    fn provider_id(&self) -> &str {
        &self.provider
    }
    async fn analyze(
        &self,
        image: ResolvedImage,
        question: &str,
    ) -> Result<VisionAnswer, String> {
        *self.captured.lock().unwrap() = Some((image, question.to_string()));
        Ok(VisionAnswer {
            answer: "the picture shows a cat".into(),
            tokens_used: 128,
        })
    }
}

fn ctx(root: PathBuf) -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        root,
    )
}

#[tokio::test]
async fn end_to_end_local_png_routes_through_dispatcher_as_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let img = dir.path().join("shot.png");
    tokio::fs::write(&img, b"\x89PNG\r\n\x1a\n").await.unwrap();

    let stub = Arc::new(CaptureDispatcher {
        provider: "anthropic".into(),
        ..Default::default()
    });
    let tool = Vision::new(stub.clone());
    let r = tool
        .dispatch(
            serde_json::json!({
                "image": img.to_string_lossy(),
                "question": "what's in this picture?",
            }),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .expect("dispatch ok");

    assert!(!r.is_error, "got error: {r:?}");
    let text = match &r.content[0] {
        ContentBlock::Text { text } => text.clone(),
        _ => panic!("expected text content"),
    };
    let payload: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(payload["answer"], "the picture shows a cat");
    assert_eq!(payload["provider"], "anthropic");
    assert_eq!(payload["tokens_used"], 128);

    let captured = stub.captured.lock().unwrap();
    let (image, q) = captured.as_ref().unwrap();
    assert_eq!(q, "what's in this picture?");
    match image {
        ResolvedImage::Bytes { mime, bytes } => {
            assert_eq!(mime, "image/png");
            assert!(bytes.starts_with(b"\x89PNG"));
        }
        other => panic!("expected Bytes, got {other:?}"),
    }
}

#[tokio::test]
async fn end_to_end_https_url_passes_through_to_dispatcher_as_url() {
    let stub = Arc::new(CaptureDispatcher {
        provider: "openai".into(),
        ..Default::default()
    });
    let tool = Vision::new(stub.clone());
    let r = tool
        .dispatch(
            serde_json::json!({
                "image": "https://example.com/cat.png",
                "question": "describe the image",
            }),
            &ctx(PathBuf::from("/tmp")),
        )
        .await
        .expect("dispatch ok");

    assert!(!r.is_error, "got error: {r:?}");
    let captured = stub.captured.lock().unwrap();
    match &captured.as_ref().unwrap().0 {
        ResolvedImage::Url(u) => assert_eq!(u, "https://example.com/cat.png"),
        other => panic!("expected Url passthrough, got {other:?}"),
    }
}

#[tokio::test]
async fn default_form_returns_not_wired_error_with_hint() {
    let tool = Vision::default();
    let r = tool
        .dispatch(
            serde_json::json!({
                "image": "https://example.com/x.png",
                "question": "what?",
            }),
            &ctx(PathBuf::from("/tmp")),
        )
        .await
        .expect("dispatch ok");
    assert!(r.is_error);
    let text = match &r.content[0] {
        ContentBlock::Text { text } => text.clone(),
        _ => panic!("expected text"),
    };
    assert!(
        text.contains("not wired") && text.contains("Vision::new"),
        "expected not-wired hint pointing at Vision::new; got: {text}"
    );
}

#[tokio::test]
async fn path_traversal_outside_workspace_rejected_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let stub = Arc::new(CaptureDispatcher {
        provider: "anthropic".into(),
        ..Default::default()
    });
    let tool = Vision::new(stub);
    let r = tool
        .dispatch(
            serde_json::json!({
                "image": "../escape.png",
                "question": "what?",
            }),
            &ctx(dir.path().to_path_buf()),
        )
        .await
        .expect("dispatch ok");
    assert!(r.is_error);
    let text = match &r.content[0] {
        ContentBlock::Text { text } => text.clone(),
        _ => panic!("expected text"),
    };
    assert!(text.contains("`..`"), "got: {text}");
}
