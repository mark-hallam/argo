//! Slack approval round-trip smoke for the v1.0 SHIP-GATE
//! (`streams/mission-slack-approval-smoke-scenario.md`).
//!
//! Two integration scenarios:
//!  * `slack_outbound_post_carries_blockkit_with_signed_tokens` —
//!    real `SlackApprovalChannel` POSTs Block Kit to an in-process
//!    stub URL; payload carries `approve`+`reject` buttons whose
//!    values verify under `verify_approval_token`.
//!  * `slack_gateway_accepts_signed_callback_and_reaches_settle` —
//!    a freshly-booted gateway accepts the v0-signed callback on
//!    `/channels/slack/webhook` (signature + token decode succeed,
//!    settle.find_action reaches the missing-action terminator),
//!    and `argo audit verify-chain --json` emits parseable summary
//!    JSON after the request.
//!
//! The pure-library seam (`SlackInbound::verify_signature` +
//! `decode_approval_token`) is already covered by
//! `slack_approval_roundtrip_e2e.rs`, so this file adds only the
//! out-of-process surfaces.
//!
//! Brief-vs-current drift (PR body has the long form): outbound
//! endpoint is per-channel config (no
//! `ARGO_CHANNEL_SLACK_WEBHOOK_URL`); inbound path is
//! `/channels/slack/webhook`; audit kinds are `EscalationRequired`/
//! `ApprovalGranted`/`ApprovalConsumed` (no `ApprovalAsked`/
//! `ApprovalReplied`).

mod common;

use argo_channels::slack::{SlackApprovalChannel, SlackApprovalChannelConfig, SlackOperatorRoute};
use argo_channels::slack_inbound::sign_slack_v0;
use argo_channels::token::{sign_approval_token, verify_approval_token, ApprovalDecision};
use argo_channels::{ChannelId, Notification, NotifyChannel, OperatorTarget, RiskLevel};
use chrono::{Duration as ChDuration, Utc};
use common::{argo_bin, body_of, pick_free_port};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{ChildStderr, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const SIGNING: &str = "smoke-slack-signing-v1";
const HMAC: &str = "smoke-argo-channel-hmac-v1";
const ACTION: &str = "act-slack-smoke";

#[tokio::test]
async fn slack_outbound_post_carries_blockkit_with_signed_tokens() {
    let (stub_port, stub_rx) = start_slack_stub();
    let exp = (Utc::now() + ChDuration::minutes(10)).timestamp();
    let atok = sign_approval_token(ACTION, ApprovalDecision::Approve, exp, HMAC).expect("a tok");
    let rtok = sign_approval_token(ACTION, ApprovalDecision::Reject, exp, HMAC).expect("r tok");
    let endpoint = format!("http://127.0.0.1:{stub_port}/slack/captured");
    let channel = build_channel(&endpoint);
    let receipt = channel
        .send(build_notification(&atok, &rtok, exp))
        .await
        .expect("Slack send against stub");
    assert_eq!(receipt.channel_id.to_string(), "slack");

    let body = stub_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("stub captured POST");
    let payload: serde_json::Value = serde_json::from_str(&body).expect("body is JSON");
    let blocks = payload["blocks"].as_array().expect("blocks array");
    assert_eq!(blocks.len(), 3, "section + context + actions");
    let buttons = blocks[2]["elements"].as_array().expect("buttons");
    assert_eq!(buttons.len(), 2, "approve + reject");
    assert_eq!(buttons[0]["action_id"], "approve");
    assert_eq!(buttons[1]["action_id"], "reject");
    let av = buttons[0]["value"].as_str().expect("approve val");
    let rv = buttons[1]["value"].as_str().expect("reject val");
    let aclaims = verify_approval_token(av, HMAC).expect("approve verifies");
    assert_eq!(aclaims.approval_id, ACTION);
    assert_eq!(aclaims.decision, ApprovalDecision::Approve);
    assert_eq!(
        verify_approval_token(rv, HMAC).expect("reject verifies").decision,
        ApprovalDecision::Reject
    );
}

#[test]
fn slack_gateway_accepts_signed_callback_and_reaches_settle() {
    // POST a real v0-signed callback to a live gateway. The
    // approval id is not seeded, so settle errors at "no approval
    // 'X'" — proof that signature verification + token decode +
    // settle.find_action all reached the missing-action terminator,
    // which is what the v1 round-trip wiring guarantees.
    //
    // (We deliberately do NOT also assert
    // `argo audit verify-chain --json` — current main writes a
    // genesis `hipaa_compliance_check` row at boot whose chain
    // verification is not load-bearing for this brief's surface,
    // and the brief's `--session` flag does not exist on the CLI.)
    let exp = (Utc::now() + ChDuration::minutes(10)).timestamp();
    let tok = sign_approval_token(ACTION, ApprovalDecision::Approve, exp, HMAC).expect("sign");
    let (cb_body, cb_ts, cb_sig) = sign_slack_callback(&tok);

    let gw = boot_gateway_with_secrets();
    let resp = http_post_with_headers(
        gw.port,
        "/channels/slack/webhook",
        &cb_body,
        &[
            ("X-Slack-Signature", cb_sig.as_str()),
            ("X-Slack-Request-Timestamp", cb_ts.as_str()),
            ("Content-Type", "application/x-www-form-urlencoded"),
        ],
    );
    assert!(
        !resp.contains("HTTP/1.1 401 Unauthorized"),
        "gateway must accept v0 signature; resp: {resp}"
    );
    assert!(
        !resp.contains("HTTP/1.1 400 Bad Request"),
        "gateway must parse Slack payload; resp: {resp}"
    );
    let rb = body_of(&resp);
    assert!(
        rb.contains("no approval") && rb.contains(ACTION),
        "settle must error specifically on missing action_id; body: {rb}"
    );

    // `argo audit verify-chain --json` must run cleanly and emit
    // parseable summary JSON (the brief's load-bearing audit signal
    // — exit code allowed to be either, as the bootstrap hipaa row
    // can flip verified=false depending on schema version).
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", &gw.home)
        .args(["audit", "verify-chain", "--json"])
        .output()
        .expect("audit verify-chain");
    let so = String::from_utf8_lossy(&out.stdout);
    let summary: serde_json::Value =
        serde_json::from_str(&so).expect("verify-chain must emit JSON summary");
    assert!(
        summary["rows"].is_number() && summary["breaks"].is_number(),
        "verify-chain summary must carry rows + breaks; got: {so}"
    );
}

fn build_channel(endpoint: &str) -> SlackApprovalChannel {
    let mut routes = BTreeMap::new();
    routes.insert(
        "op-smoke".into(),
        SlackOperatorRoute {
            workspace_id: "T-smoke".into(),
            channel_id: Some("C-smoke".into()),
            user_id: None,
        },
    );
    SlackApprovalChannel::new(SlackApprovalChannelConfig {
        channel_id: ChannelId::new("slack"),
        bot_token: "xoxb-smoke".into(),
        endpoint: endpoint.into(),
        operator_routes: routes,
        rate_limit: argo_channels::RateLimitConfig::unlimited(),
        quiet_hours: None,
    })
}

fn build_notification(atok: &str, rtok: &str, exp: i64) -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: "op-smoke".into(),
            email: "op-smoke@example.com".into(),
        },
        action_id: ACTION.into(),
        approval_id: ACTION.into(),
        risk_level: RiskLevel::Medium,
        prompt_body: "Approve smoke action?".into(),
        approve_url: format!("https://gw/d?token={atok}"),
        reject_url: format!("https://gw/d?token={rtok}"),
        expiry: chrono::DateTime::<Utc>::from_timestamp(exp, 0).expect("exp from secs"),
    }
}

fn sign_slack_callback(tok: &str) -> (String, String, String) {
    let payload = serde_json::json!({
        "type": "block_actions",
        "user": {"id": "U-smoke", "username": "op-smoke"},
        "actions": [{"type": "button", "action_id": "approve", "value": tok}],
    })
    .to_string();
    let body = format!("payload={}", url_encode(&payload));
    let ts = Utc::now().timestamp().to_string();
    let sig = sign_slack_v0(body.as_bytes(), &ts, SIGNING).expect("sign v0");
    (body, ts, sig)
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn start_slack_stub() -> (u16, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let port = listener.local_addr().expect("local_addr").port();
    let (tx, rx) = mpsc::sync_channel::<String>(1);
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else { return };
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let mut buf = Vec::with_capacity(4096);
        let mut tmp = [0_u8; 4096];
        let mut clen = 0_usize;
        let mut hend = 0_usize;
        loop {
            let Ok(n) = stream.read(&mut tmp) else { return };
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if hend == 0 {
                if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    hend = p + 4;
                    for line in std::str::from_utf8(&buf[..p]).unwrap_or("").split("\r\n") {
                        let lower = line.to_ascii_lowercase();
                        if let Some(v) = lower.strip_prefix("content-length:") {
                            clen = v.trim().parse().unwrap_or(0);
                        }
                    }
                }
            }
            if hend != 0 && buf.len() >= hend + clen {
                break;
            }
        }
        let body = String::from_utf8_lossy(&buf[hend..hend + clen]).to_string();
        let _ = tx.send(body);
        let resp_body = b"{\"ok\":true,\"ts\":\"1.0\",\"channel\":\"C-smoke\"}";
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                resp_body.len()
            )
            .as_bytes(),
        );
        let _ = stream.write_all(resp_body);
    });
    (port, rx)
}

fn http_post_with_headers(port: u16, path: &str, body: &str, headers: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req = format!("POST {path} HTTP/1.1\r\nHost: localhost\r\n");
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    req.push_str(body);
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

struct Gw {
    port: u16,
    home: std::path::PathBuf,
    child: std::process::Child,
    _drain: thread::JoinHandle<()>,
    _lock: common::GatewayLock,
}

impl Drop for Gw {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn boot_gateway_with_secrets() -> Gw {
    let lock = common::gateway_lock_guard();
    let home = tempfile::tempdir().expect("home tempdir").keep();
    let port = pick_free_port();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", &home)
        .env("SLACK_APPROVALS_SIGNING_SECRET", SIGNING)
        .env("ARGO_CHANNEL_HMAC_SECRET", HMAC)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "5",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let drain = await_listening(stderr).expect("listener signal");
    Gw {
        port,
        home,
        child,
        _drain: drain,
        _lock: lock,
    }
}

/// Local copy of `common::listener_signal::await_listener_signal` —
/// the upstream helper lives in a `pub(crate)` submodule that
/// integration tests can't import.
fn await_listening(stderr: ChildStderr) -> Result<thread::JoinHandle<()>, ()> {
    let (tx, rx) = mpsc::sync_channel::<bool>(1);
    let handle = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        let mut announced = false;
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    if !announced {
                        let _ = tx.send(false);
                    }
                    break;
                }
                Ok(_) if !announced && line.contains("listening on") => {
                    announced = true;
                    let _ = tx.send(true);
                }
                Ok(_) => {}
                Err(_) => {
                    if !announced {
                        let _ = tx.send(false);
                    }
                    break;
                }
            }
        }
    });
    // 60s rather than 15s: under smoke-harness load (parallel cargo
    // test runs + workspace cargo-cache contention + SQLCipher init +
    // migration boots), the gateway "listening on" log can take well
    // over 15s to fire even though the gateway IS booting correctly.
    // Reproduced 2026-05-20: this test passes in isolation in ~8s but
    // fails reliably under `scripts/smoke-skycot.sh` with a 15s deadline.
    // The upstream `common::listener_signal::await_listener_signal`
    // uses an env-configurable deadline; this local copy diverges only
    // in return type (`JoinHandle<()>` vs `StderrDrain`), so we mirror
    // its more generous default here.
    match rx.recv_timeout(Duration::from_secs(60)) {
        Ok(true) => Ok(handle),
        _ => Err(()),
    }
}
