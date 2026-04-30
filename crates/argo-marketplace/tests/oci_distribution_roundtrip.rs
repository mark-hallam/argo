//! Hermetic round-trip test for `OciDistributionRegistry`.
//!
//! Spins up a tokio `TcpListener` that handles a small subset of
//! the OCI Distribution Spec v2 — enough for a single push +
//! pull cycle to succeed. Asserts that a bundle pushed against
//! the mock comes back byte-identical on pull.
//!
//! The mock is intentionally minimal:
//!   - `POST /v2/<repo>/blobs/uploads/`         → 202 + Location
//!   - `PUT  <upload_url>?digest=sha256:<x>`    → 201 (stores blob)
//!   - `GET  /v2/<repo>/blobs/<digest>`         → 200 + blob bytes
//!   - `PUT  /v2/<repo>/manifests/<tag>`        → 201 (stores manifest)
//!   - `GET  /v2/<repo>/manifests/<tag>`        → 200 + manifest body
//!
//! Anything else returns 404. No auth.
use argo_marketplace::{
    BundleEntry, BundleManifest, OciDistributionRegistry, OciReference, OciRegistry,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// Server-side state: blobs keyed by digest, manifests keyed by
/// `(repo, tag)`. Shared across requests via `Arc<Mutex<…>>`.
#[derive(Default)]
struct State {
    blobs: HashMap<String, Vec<u8>>,
    manifests: HashMap<(String, String), Vec<u8>>,
    next_upload: u64,
    pending_uploads: HashMap<String, String>, // upload_id → repo
}

#[tokio::test]
async fn pushed_bundle_round_trips_via_mock_oci_v2() {
    let state: Arc<Mutex<State>> = Arc::new(Mutex::new(State::default()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_state = Arc::clone(&state);

    // Server task — accepts connections until the test drops the
    // join handle. Each connection is a single HTTP/1.1 request.
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let st = Arc::clone(&server_state);
            tokio::spawn(async move {
                handle(&mut stream, st, port).await;
            });
        }
    });

    let registry =
        OciDistributionRegistry::new(&format!("http://127.0.0.1:{port}"), None);
    let reference =
        OciReference::parse(&format!("127.0.0.1:{port}/argo/test-bundle:v1")).unwrap();
    let tar_bytes = b"fake-tarball-bytes-for-round-trip-test".to_vec();
    let bundle = BundleManifest {
        digest: "sha256:fake".into(),
        workspace_name: "test-bundle".into(),
        tag: "v1".into(),
        entries: vec![BundleEntry {
            path: "argo.toml".into(),
            digest: "sha256:fake-entry".into(),
            size: 7,
        }],
    };

    registry.push(&reference, &tar_bytes, &bundle).await.unwrap();
    let (pulled_tar, pulled_manifest) = registry.pull(&reference).await.unwrap();

    assert_eq!(pulled_tar, tar_bytes, "tar round-trip");
    assert_eq!(
        pulled_manifest.workspace_name,
        bundle.workspace_name,
        "manifest workspace_name round-trip"
    );
    assert_eq!(pulled_manifest.tag, bundle.tag, "manifest tag round-trip");
    assert_eq!(
        pulled_manifest.entries.len(),
        1,
        "manifest entries round-trip"
    );

    // Server shutdown — drop the join handle so accept() bails.
    server.abort();
}

#[tokio::test]
async fn pull_against_unknown_tag_returns_provider_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let state: Arc<Mutex<State>> = Arc::new(Mutex::new(State::default()));
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let st = Arc::clone(&server_state);
            tokio::spawn(async move {
                handle(&mut stream, st, port).await;
            });
        }
    });

    let registry =
        OciDistributionRegistry::new(&format!("http://127.0.0.1:{port}"), None);
    let reference =
        OciReference::parse(&format!("127.0.0.1:{port}/argo/missing:v1")).unwrap();
    match registry.pull(&reference).await {
        Err(argo_types::ArgoError::Provider(msg)) => {
            assert!(msg.contains("404") || msg.contains("manifest"));
        }
        other => panic!("expected Provider error, got {other:?}"),
    }
    server.abort();
}

/// Tiny HTTP/1.1 handler — reads the request, dispatches by
/// method+path, writes a single response. Keeps the test code
/// self-contained.
async fn handle(stream: &mut tokio::net::TcpStream, state: Arc<Mutex<State>>, port: u16) {
    let mut buf = vec![0u8; 65536];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    if n == 0 {
        return;
    }
    let raw = &buf[..n];
    // Split header from body.
    let header_end = match find_subseq(raw, b"\r\n\r\n") {
        Some(i) => i,
        None => return,
    };
    let header_str = std::str::from_utf8(&raw[..header_end]).unwrap_or("");
    let body_start = header_end + 4;
    let mut lines = header_str.lines();
    let request_line = lines.next().unwrap_or("");
    let mut content_length = 0usize;
    for h in lines {
        let lower = h.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    // We may not have read the full body yet — drain the rest.
    let mut body = raw[body_start..].to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0u8; content_length - body.len()];
        let m = stream.read(&mut chunk).await.unwrap_or(0);
        if m == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..m]);
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    let resp = route(method, path, &body, &state, port).await;
    let _ = stream.write_all(&resp).await;
    let _ = stream.shutdown().await;
}

fn find_subseq(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

async fn route(
    method: &str,
    path: &str,
    body: &[u8],
    state: &Arc<Mutex<State>>,
    port: u16,
) -> Vec<u8> {
    // POST /v2/<repo>/blobs/uploads/
    if method == "POST" {
        if let Some(rest) = path.strip_prefix("/v2/") {
            if let Some(repo) = rest.strip_suffix("/blobs/uploads/") {
                let mut st = state.lock().await;
                st.next_upload += 1;
                let id = format!("u-{}", st.next_upload);
                let location = format!("/v2/{repo}/blobs/uploads/{id}");
                st.pending_uploads.insert(id, repo.to_string());
                return format!(
                    "HTTP/1.1 202 Accepted\r\nLocation: http://127.0.0.1:{port}{location}\r\nContent-Length: 0\r\n\r\n"
                ).into_bytes();
            }
        }
    }
    // PUT /v2/<repo>/blobs/uploads/<id>?digest=…
    if method == "PUT" {
        if let Some((path_only, query)) = path.split_once('?') {
            if let Some(rest) = path_only.strip_prefix("/v2/") {
                if let Some((repo, tail)) = split_repo_and_tail(rest, "/blobs/uploads/") {
                    let _id = tail; // We don't validate id matches state.
                    let digest = query
                        .split('&')
                        .find_map(|kv| kv.strip_prefix("digest="))
                        .unwrap_or("");
                    let mut st = state.lock().await;
                    st.blobs.insert(digest.to_string(), body.to_vec());
                    let _ = repo;
                    return b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n".to_vec();
                }
            }
        }
        // PUT /v2/<repo>/manifests/<tag>
        if let Some(rest) = path.strip_prefix("/v2/") {
            if let Some((repo, tag)) = split_repo_and_tail(rest, "/manifests/") {
                let mut st = state.lock().await;
                st.manifests
                    .insert((repo.to_string(), tag.to_string()), body.to_vec());
                return b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n".to_vec();
            }
        }
    }
    if method == "GET" {
        if let Some(rest) = path.strip_prefix("/v2/") {
            if let Some((_repo, digest)) = split_repo_and_tail(rest, "/blobs/") {
                let st = state.lock().await;
                if let Some(blob) = st.blobs.get(digest) {
                    return http_200_bytes("application/octet-stream", blob);
                }
                return not_found();
            }
            if let Some((repo, tag)) = split_repo_and_tail(rest, "/manifests/") {
                let st = state.lock().await;
                if let Some(m) = st.manifests.get(&(repo.to_string(), tag.to_string())) {
                    return http_200_bytes(
                        "application/vnd.oci.image.manifest.v1+json",
                        m,
                    );
                }
                return not_found();
            }
        }
    }
    not_found()
}

fn http_200_bytes(content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

fn not_found() -> Vec<u8> {
    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec()
}

/// Split `<repo>/<separator><tail>` into `(repo, tail)`. The
/// separator helps us distinguish `/blobs/<digest>` from
/// `/manifests/<tag>`.
fn split_repo_and_tail<'a>(rest: &'a str, separator: &str) -> Option<(&'a str, &'a str)> {
    let idx = rest.find(separator)?;
    let repo = &rest[..idx];
    let tail = &rest[idx + separator.len()..];
    if repo.is_empty() || tail.is_empty() {
        return None;
    }
    Some((repo, tail))
}
