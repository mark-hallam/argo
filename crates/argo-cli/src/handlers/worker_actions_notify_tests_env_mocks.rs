//! Network-mock helpers (HTTP + SMTP) and env-var test fixtures shared
//! by the per-channel `EnvChannelSender` tests in `env_channels.rs`.

use super::action;
use super::super::*;
use argo_identity::NotifyChannelRef;
use std::sync::OnceLock;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

pub(super) async fn env_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

pub(super) fn clear_env(vars: &[&str]) {
    for var in vars {
        std::env::remove_var(var);
    }
}

pub(super) async fn send_env_channel(channel: &str, target: &str) -> Result<(), String> {
    let sender = EnvChannelSender;
    let notification = build_notification(&action("act-env", "high"));
    sender
        .send_via(&NotifyChannelRef::new(channel, target), &notification)
        .await
}

pub(super) async fn http_mock(
    status: &'static str,
    body: &'static str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let n = socket.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n]);
            if request_complete(&bytes) {
                break;
            }
        }
        let resp = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&bytes).to_string()
    });
    (format!("http://{addr}"), handle)
}

fn request_complete(bytes: &[u8]) -> bool {
    let Some(head_end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&bytes[..head_end]);
    let len = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    bytes.len() >= head_end + 4 + len
}

pub(super) async fn smtp_mock() -> (u16, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let (reader, mut writer) = socket.into_split();
        let mut reader = BufReader::new(reader);
        let mut log = String::new();
        writer.write_all(b"220 mock.smtp\r\n").await.unwrap();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).await.unwrap() == 0 {
                break;
            }
            log.push_str(&line);
            let upper = line.to_ascii_uppercase();
            if upper.starts_with("EHLO") || upper.starts_with("HELO") {
                writer
                    .write_all(b"250-mock.smtp\r\n250-AUTH PLAIN LOGIN\r\n250 OK\r\n")
                    .await
                    .unwrap();
            } else if upper.starts_with("AUTH PLAIN") {
                writer.write_all(b"235 2.7.0 OK\r\n").await.unwrap();
            } else if upper.starts_with("AUTH LOGIN") {
                writer.write_all(b"334 VXNlcm5hbWU6\r\n").await.unwrap();
                let mut user = String::new();
                reader.read_line(&mut user).await.unwrap();
                log.push_str(&user);
                writer.write_all(b"334 UGFzc3dvcmQ6\r\n").await.unwrap();
                let mut pass = String::new();
                reader.read_line(&mut pass).await.unwrap();
                log.push_str(&pass);
                writer.write_all(b"235 2.7.0 OK\r\n").await.unwrap();
            } else if upper.starts_with("MAIL FROM") || upper.starts_with("RCPT TO") {
                writer.write_all(b"250 2.1.0 OK\r\n").await.unwrap();
            } else if upper.starts_with("DATA") {
                writer
                    .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                    .await
                    .unwrap();
                loop {
                    let mut data = String::new();
                    reader.read_line(&mut data).await.unwrap();
                    log.push_str(&data);
                    if data.trim_end() == "." {
                        break;
                    }
                }
                writer.write_all(b"250 2.0.0 queued\r\n").await.unwrap();
            } else if upper.starts_with("QUIT") {
                writer.write_all(b"221 2.0.0 bye\r\n").await.unwrap();
                break;
            } else {
                writer.write_all(b"250 OK\r\n").await.unwrap();
            }
        }
        log
    });
    (port, handle)
}
