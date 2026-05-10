//! Auth-variant round-trip for `OciDistributionRegistry`.
//!
//! Sibling of `oci_distribution_roundtrip.rs`. Exercises the
//! `WWW-Authenticate: Bearer` challenge → token-endpoint →
//! authenticated request flow. The shared HTTP-1.1 mock helpers
//! are duplicated rather than shared via a common module: each
//! `tests/*.rs` is its own integration-test binary, and duplication
//! keeps each binary self-contained.
use argo_marketplace::{BundleManifest, OciDistributionRegistry, OciReference, OciRegistry};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Default)]
struct State {
    blobs: HashMap<String, Vec<u8>>,
    manifests: HashMap<(String, String), Vec<u8>>,
    next_upload: u64,
    pending_uploads: HashMap<String, (String, Vec<u8>)>,
}

#[tokio::test]
async fn challenge_handshake_yields_a_bearer_then_round_trips() {
    // Mock that requires authentication: every v2 request without
    // a valid Bearer returns 401 + a `Bearer realm="…"` challenge;
    // the realm endpoint accepts Basic auth and returns a token;
    // requests with `Authorization: Bearer test-token` work normally.
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
                handle_with_auth(&mut stream, st, port).await;
            });
        }
    });

    let registry = OciDistributionRegistry::new(&format!("http://127.0.0.1:{port}"), None)
        .with_basic_auth("user", "secret");
    let reference = OciReference::parse(&format!("127.0.0.1:{port}/argo/auth-test:v1")).unwrap();
    let tar_bytes = b"authed-tar-bytes".to_vec();
    let bundle = BundleManifest {
        digest: "sha256:fake".into(),
        workspace_name: "auth-test".into(),
        tag: "v1".into(),
        entries: vec![],
    };

    registry
        .push(&reference, &tar_bytes, &bundle)
        .await
        .unwrap();
    let (pulled_tar, pulled_manifest) = registry.pull(&reference).await.unwrap();
    assert_eq!(pulled_tar, tar_bytes);
    assert_eq!(pulled_manifest.workspace_name, "auth-test");

    server.abort();
}

async fn handle_with_auth(stream: &mut tokio::net::TcpStream, state: Arc<Mutex<State>>, port: u16) {
    let mut buf = vec![0u8; 65536];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    if n == 0 {
        return;
    }
    let raw = &buf[..n];
    let header_end = match find_subseq(raw, b"\r\n\r\n") {
        Some(i) => i,
        None => return,
    };
    let header_str = std::str::from_utf8(&raw[..header_end]).unwrap_or("");
    let body_start = header_end + 4;
    let mut lines = header_str.lines();
    let request_line = lines.next().unwrap_or("");
    let mut content_length = 0usize;
    let mut authorization = None::<String>;
    for h in lines {
        let lower = h.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = lower.strip_prefix("authorization:") {
            authorization = Some(v.trim().to_string());
        }
    }
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

    // Token endpoint: any GET to /token returns `{"token":"test-token"}`.
    if method == "GET" && path.starts_with("/token") {
        let body = b"{\"token\":\"test-token\"}";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(resp.as_bytes()).await;
        let _ = stream.write_all(body).await;
        let _ = stream.shutdown().await;
        return;
    }

    // /v2/... requests require Bearer test-token.
    if path.starts_with("/v2/") && authorization.as_deref() != Some("bearer test-token") {
        let challenge = format!(
            "HTTP/1.1 401 Unauthorized\r\n\
             WWW-Authenticate: Bearer realm=\"http://127.0.0.1:{port}/token\",service=\"argo-test\",scope=\"repository:argo/auth-test:pull,push\"\r\n\
             Content-Length: 0\r\n\r\n"
        );
        let _ = stream.write_all(challenge.as_bytes()).await;
        let _ = stream.shutdown().await;
        return;
    }

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
    if method == "POST" {
        if let Some(rest) = path.strip_prefix("/v2/") {
            if let Some(repo) = rest.strip_suffix("/blobs/uploads/") {
                let mut st = state.lock().await;
                st.next_upload += 1;
                let id = format!("u-{}", st.next_upload);
                let location = format!("/v2/{repo}/blobs/uploads/{id}");
                st.pending_uploads
                    .insert(id, (repo.to_string(), Vec::new()));
                return format!(
                    "HTTP/1.1 202 Accepted\r\nLocation: http://127.0.0.1:{port}{location}\r\nContent-Length: 0\r\n\r\n"
                ).into_bytes();
            }
        }
    }
    if method == "PATCH" {
        if let Some(rest) = path.strip_prefix("/v2/") {
            if let Some((repo, upload_id)) = split_repo_and_tail(rest, "/blobs/uploads/") {
                let mut st = state.lock().await;
                let entry = st
                    .pending_uploads
                    .entry(upload_id.to_string())
                    .or_insert_with(|| (repo.to_string(), Vec::new()));
                entry.1.extend_from_slice(body);
                let last = entry.1.len().saturating_sub(1);
                let location = format!("/v2/{repo}/blobs/uploads/{upload_id}");
                return format!(
                    "HTTP/1.1 202 Accepted\r\nLocation: http://127.0.0.1:{port}{location}\r\nRange: 0-{last}\r\nContent-Length: 0\r\n\r\n"
                ).into_bytes();
            }
        }
    }
    if method == "PUT" {
        if let Some((path_only, query)) = path.split_once('?') {
            if let Some(rest) = path_only.strip_prefix("/v2/") {
                if let Some((repo, tail)) = split_repo_and_tail(rest, "/blobs/uploads/") {
                    let upload_id = tail;
                    let digest = query
                        .split('&')
                        .find_map(|kv| kv.strip_prefix("digest="))
                        .unwrap_or("");
                    let digest = percent_decode_component(digest);
                    let mut st = state.lock().await;
                    let bytes = st
                        .pending_uploads
                        .remove(upload_id)
                        .map(|(_, pending)| pending)
                        .filter(|pending| !pending.is_empty())
                        .unwrap_or_else(|| body.to_vec());
                    st.blobs.insert(digest.clone(), bytes);
                    let _ = repo;
                    let location = format!("/v2/{repo}/blobs/{digest}");
                    return format!(
                        "HTTP/1.1 201 Created\r\nLocation: http://127.0.0.1:{port}{location}\r\nContent-Length: 0\r\n\r\n"
                    ).into_bytes();
                }
            }
        }
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
                    return http_200_bytes("application/vnd.oci.image.manifest.v1+json", m);
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

fn split_repo_and_tail<'a>(rest: &'a str, separator: &str) -> Option<(&'a str, &'a str)> {
    let idx = rest.find(separator)?;
    let repo = &rest[..idx];
    let tail = &rest[idx + separator.len()..];
    if repo.is_empty() || tail.is_empty() {
        return None;
    }
    Some((repo, tail))
}

fn percent_decode_component(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'%' && idx + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_value(bytes[idx + 1]), hex_value(bytes[idx + 2])) {
                out.push((hi << 4) | lo);
                idx += 3;
                continue;
            }
        }
        out.push(bytes[idx]);
        idx += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
