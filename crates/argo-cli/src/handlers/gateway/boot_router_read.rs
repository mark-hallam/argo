//! HTTP/1.1 request-drain helpers used by `boot_router::handle_connection`.
//!
//! The accept loop's old single 4 KB `stream.read` worked for curl
//! (one `writev` of headers+body) but mis-handled the SPA's browser
//! fetch on macOS Chrome, which sometimes ships the headers in one
//! TCP segment and the body in another. When the first segment landed
//! alone, the dispatcher saw an empty body — or a truncated request
//! line — and the second `POST /api/v1/genesis` (the operator's "Try
//! again" click after the idempotency error) fell through to a
//! 503-returning surface even though a curl call with byte-identical
//! headers + body returned 200 + the full SSE stream. See
//! `fix-spa-genesis-try-again-503-vs-curl-200`.

use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Drain one HTTP/1.1 request off `stream`. Reads in a loop until the
/// header terminator `\r\n\r\n` is seen AND any declared
/// `Content-Length` body bytes are accumulated, or until `deadline`
/// elapses / the `ceiling` byte cap is hit.
///
/// Errors and timeouts return whatever bytes were accumulated so far;
/// downstream parsers then surface the right shape (400 on partial
/// JSON, 404 on truncated path, etc.) instead of hanging the accept
/// loop on a slow / malicious peer.
pub(super) async fn read_full_request<R: AsyncRead + Unpin>(
    stream: &mut R,
    deadline: Duration,
    ceiling: usize,
) -> Vec<u8> {
    let started = Instant::now();
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    loop {
        if request_is_complete(&buf) {
            return buf;
        }
        if buf.len() >= ceiling {
            return buf;
        }
        let remaining = match deadline.checked_sub(started.elapsed()) {
            Some(d) if !d.is_zero() => d,
            _ => return buf,
        };
        let n = match tokio::time::timeout(remaining, stream.read(&mut chunk)).await {
            Ok(Ok(n)) => n,
            _ => return buf,
        };
        if n == 0 {
            return buf;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// `true` when `buf` contains a complete HTTP/1.1 request head and
/// (if `Content-Length` is declared) the full body bytes. Byte-oriented
/// so a request line split mid-UTF-8 still resolves correctly;
/// non-`Content-Length` requests (GET / OPTIONS / chunked upload)
/// short-circuit on the header terminator.
fn request_is_complete(buf: &[u8]) -> bool {
    let Some(headers_end) = find_header_terminator(buf) else {
        return false;
    };
    let body_start = headers_end + 4;
    let declared_len = parse_content_length(&buf[..headers_end]).unwrap_or(0);
    buf.len() >= body_start + declared_len
}

fn find_header_terminator(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Parse the `Content-Length` value out of a header block (no body
/// bytes present). Returns `None` when the header is absent or
/// malformed; the caller treats that as "no body declared" and only
/// requires the header terminator to consider the request complete.
fn parse_content_length(headers: &[u8]) -> Option<usize> {
    let headers = std::str::from_utf8(headers).ok()?;
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt as _;
    use tokio::net::{TcpListener, TcpStream};

    const TEST_CEILING: usize = 64 * 1024;

    fn browser_shape_request(body: &str) -> Vec<u8> {
        let mut req = String::new();
        req.push_str("POST /api/v1/genesis HTTP/1.1\r\n");
        req.push_str("Host: localhost:7400\r\n");
        req.push_str("Connection: keep-alive\r\n");
        req.push_str("Accept: text/event-stream\r\n");
        req.push_str("Content-Type: application/json\r\n");
        req.push_str("Origin: http://localhost:3000\r\n");
        req.push_str("Sec-Fetch-Site: same-site\r\n");
        req.push_str("Sec-Fetch-Mode: cors\r\n");
        req.push_str("Sec-Fetch-Dest: empty\r\n");
        req.push_str(
            "User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36\r\n",
        );
        req.push_str("X-Argo-Tenant-Id: op-abc123def456789012\r\n");
        req.push_str(&format!("Content-Length: {}\r\n", body.len()));
        req.push_str("\r\n");
        req.push_str(body);
        req.into_bytes()
    }

    /// Stage a loopback connection where the client writes the headers
    /// and body in two separate `write_all` calls — the shape the macOS
    /// Chrome fetch path produces for the SPA's second-attempt
    /// `POST /api/v1/genesis`. `read_full_request` must reassemble both
    /// halves so the dispatcher routes to the genesis SSE arm.
    async fn drive_split_write(bytes: &[u8], split_at: usize) -> Vec<u8> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (head, tail) = bytes.split_at(split_at);
        let head = head.to_vec();
        let tail = tail.to_vec();
        let client = tokio::spawn(async move {
            let mut sock = TcpStream::connect(addr).await.unwrap();
            sock.set_nodelay(true).unwrap();
            sock.write_all(&head).await.unwrap();
            sock.flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
            sock.write_all(&tail).await.unwrap();
            sock.flush().await.unwrap();
            sock
        });
        let (mut server_side, _) = listener.accept().await.unwrap();
        let result =
            read_full_request(&mut server_side, Duration::from_millis(500), TEST_CEILING).await;
        let _ = client.await.unwrap();
        result
    }

    #[test]
    fn finds_header_terminator_at_known_offset() {
        let buf = b"POST / HTTP/1.1\r\nHost: x\r\n\r\nbody";
        assert_eq!(find_header_terminator(buf), Some(24));
        let no_term = b"POST / HTTP/1.1\r\nHost: x\r\n";
        assert_eq!(find_header_terminator(no_term), None);
    }

    #[test]
    fn parses_content_length_case_insensitive() {
        let headers = b"POST / HTTP/1.1\r\nHost: x\r\ncontent-length: 87\r\n";
        assert_eq!(parse_content_length(headers), Some(87));
        let headers_mixed = b"POST / HTTP/1.1\r\nContent-Length:  42 \r\n";
        assert_eq!(parse_content_length(headers_mixed), Some(42));
        let headers_none = b"GET / HTTP/1.1\r\nHost: x\r\n";
        assert_eq!(parse_content_length(headers_none), None);
    }

    #[test]
    fn request_is_complete_only_when_body_fully_present() {
        let head_only = b"POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\nab";
        assert!(!request_is_complete(head_only));
        let full = b"POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\nabcde";
        assert!(request_is_complete(full));
        let get = b"GET / HTTP/1.1\r\nHost: x\r\n\r\n";
        assert!(request_is_complete(get));
    }

    #[tokio::test]
    async fn read_full_request_reassembles_split_browser_post() {
        let body = r#"{"role_description":"I need a customer success lead who triages tier-1 tickets and escalates refunds over $500","persona":"Casey","overwrite":true}"#;
        let raw = browser_shape_request(body);
        let split_at = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|i| i + 4)
            .unwrap();
        let assembled = drive_split_write(&raw, split_at).await;
        assert_eq!(
            assembled, raw,
            "headers+body must be reassembled byte-for-byte across two TCP writes"
        );
        let assembled_str = std::str::from_utf8(&assembled).unwrap();
        assert!(assembled_str.starts_with("POST /api/v1/genesis HTTP/1.1"));
        assert!(assembled_str.contains("\"overwrite\":true"));
    }

    #[tokio::test]
    async fn read_full_request_handles_split_inside_request_line() {
        // Worst case: first segment ends mid-path. Without the read
        // loop, the dispatcher would see `POST /api/v1/ge` and fall
        // through to a non-genesis arm.
        let body = r#"{"role_description":"customer success lead, refund triage","overwrite":true}"#;
        let raw = browser_shape_request(body);
        let assembled = drive_split_write(&raw, 15).await;
        assert_eq!(assembled, raw);
    }

    #[tokio::test]
    async fn read_full_request_returns_partial_on_deadline_exhausted() {
        // Slow client that never finishes: the helper must still return
        // so the accept loop is not blocked.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let _client = tokio::spawn(async move {
            let mut sock = TcpStream::connect(addr).await.unwrap();
            sock.write_all(b"POST /api/v1/genesis HTTP/1.1\r\n")
                .await
                .unwrap();
            sock.flush().await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;
        });
        let (mut server_side, _) = listener.accept().await.unwrap();
        let started = Instant::now();
        let buf =
            read_full_request(&mut server_side, Duration::from_millis(120), TEST_CEILING).await;
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_millis(400),
            "deadline must short-circuit; took {elapsed:?}"
        );
        assert!(buf.starts_with(b"POST /api/v1/genesis"));
    }
}
