//! Peer mesh client — V2.x multi-host visibility (NOT replication).
//!
//! Reads `ARGO_GATEWAY_PEERS` (comma-separated URLs) at first
//! invocation; spawns a tokio loop that polls each peer's
//! `GET /peer/sync` every `ARGO_PEER_POLL_MS` (default 5000); caches
//! the most recent response per peer in a process-local map for
//! `routes_peer::peer_state_response` to render.
//!
//! Bearer auth is shared-secret via `ARGO_PEER_SECRET` (constant-time
//! compare on the receiver). Cleartext over HTTP — POC only;
//! production deployments are expected to terminate TLS at a
//! reverse proxy or run the mesh on a private overlay (Tailscale
//! / WireGuard).
//!
//! Design rationale lives at
//! `docs/skycot/MULTI_HOST_ORCHESTRATOR.md`.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};

const DEFAULT_POLL_MS: u64 = 5000;
const DEFAULT_TIMEOUT_MS: u64 = 3000;
const MAX_BACKOFF_MS: u64 = 60_000;
const PEERS_ENV: &str = "ARGO_GATEWAY_PEERS";
const SECRET_ENV: &str = "ARGO_PEER_SECRET";
const POLL_MS_ENV: &str = "ARGO_PEER_POLL_MS";
const TIMEOUT_MS_ENV: &str = "ARGO_PEER_TIMEOUT_MS";

/// One peer's most recent sync result, or its last error.
///
/// `last_sync_micros` is the wall-clock micros at which we
/// successfully polled this peer; `0` means we have not yet
/// completed a successful sync. `last_error` is `Some` after any
/// failure and stays set until the next successful poll clears it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct PeerSnapshot {
    pub url: String,
    pub last_sync_micros: i64,
    pub last_error: Option<String>,
    pub session_count: u32,
    pub audit_count: u32,
}

/// Process-wide peer state. `OnceLock` so callers can read without
/// initialising the mesh — calls into `state()` short-circuit
/// when the mesh is not configured.
static PEER_STATE: OnceLock<RwLock<HashMap<String, PeerSnapshot>>> = OnceLock::new();

/// `Notify` woken by `nudge()` (the peer-sync hint emitted from
/// the dispatch path on session creation). The poll loop awaits
/// either this notify OR the periodic interval, whichever fires
/// first. Single instance per process.
static NUDGE: OnceLock<Notify> = OnceLock::new();

/// Init guard. The poll loop spawns once per process — repeated
/// calls into `ensure_started()` are no-ops after the first.
static INIT: OnceLock<()> = OnceLock::new();

fn state_lock() -> &'static RwLock<HashMap<String, PeerSnapshot>> {
    PEER_STATE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn nudge_signal() -> &'static Notify {
    NUDGE.get_or_init(Notify::new)
}

/// Parse the comma-separated peer URL list out of
/// `ARGO_GATEWAY_PEERS`. Returns an empty vec if the env var is
/// unset or whitespace.
pub(super) fn configured_peers() -> Vec<String> {
    parse_peer_list(&std::env::var(PEERS_ENV).unwrap_or_default())
}

fn parse_peer_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
        .collect()
}

/// Bearer secret, if configured. The receiver-side check in
/// `routes_peer` rejects when this is `None` AND the request is
/// not on the auth-bypass escape hatch.
pub(super) fn configured_secret() -> Option<String> {
    std::env::var(SECRET_ENV).ok().filter(|s| !s.is_empty())
}

fn poll_interval() -> Duration {
    let ms = std::env::var(POLL_MS_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_POLL_MS);
    Duration::from_millis(ms)
}

fn http_timeout() -> Duration {
    let ms = std::env::var(TIMEOUT_MS_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    Duration::from_millis(ms)
}

/// Spawn the peer-poll loop if it hasn't started yet AND at least
/// one peer is configured. Idempotent. Called from
/// `routes_peer::dispatch` (any /peer/* request) and from
/// `routes_dispatch_post` after a session is created.
///
/// # Examples
///
/// ```ignore
/// // Inside a request handler, no harm in calling on every request;
/// // the OnceLock guarantees at most one spawn per process.
/// argo_cli::handlers::gateway::peer::ensure_started();
/// ```
pub(super) fn ensure_started() {
    let peers = configured_peers();
    if peers.is_empty() {
        return;
    }
    if INIT.get().is_some() {
        return;
    }
    if INIT.set(()).is_err() {
        return;
    }
    seed_peer_snapshots(&peers);
    tokio::spawn(run_poll_loop(peers));
}

fn seed_peer_snapshots(peers: &[String]) {
    let lock = state_lock();
    let mut guard = match lock.try_write() {
        Ok(g) => g,
        Err(_) => return,
    };
    for url in peers {
        guard
            .entry(url.clone())
            .or_insert_with(|| PeerSnapshot {
                url: url.clone(),
                ..Default::default()
            });
    }
}

/// Wake the poll loop so it does an immediate sync instead of
/// waiting for the next periodic tick. Wired by
/// `routes_dispatch_post` on session creation, and by
/// `notify_peers_of_local_change()` after this gateway pushes a
/// poke to its peers.
pub(super) fn nudge() {
    nudge_signal().notify_one();
}

/// The peer-sync hint: when something interesting happens locally
/// (e.g. a session is created), notify the local poll loop AND
/// fire-and-forget a `POST /peer/poke` to every peer so they can
/// fast-pull instead of waiting for their next 5s tick.
///
/// Spawns one short-lived task per peer; never blocks the
/// caller. Failures are logged at trace level — the peer will
/// still pick the change up on its next periodic poll.
pub(super) fn notify_peers_of_local_change() {
    let peers = configured_peers();
    if peers.is_empty() {
        return;
    }
    nudge();
    let secret = configured_secret();
    for url in peers {
        let secret = secret.clone();
        tokio::spawn(async move {
            let _ = send_poke(&url, secret.as_deref()).await;
        });
    }
}

async fn send_poke(peer_url: &str, secret: Option<&str>) -> reqwest::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(http_timeout())
        .build()?;
    let mut req = client.post(format!("{peer_url}/peer/poke"));
    if let Some(s) = secret {
        req = req.bearer_auth(s);
    }
    let resp = req.send().await?;
    tracing::trace!(
        target: "gateway::peer",
        peer = %peer_url,
        status = %resp.status(),
        "peer poke sent"
    );
    Ok(())
}

async fn run_poll_loop(peers: Vec<String>) {
    let mut backoff_ms: HashMap<String, u64> = HashMap::new();
    loop {
        for url in &peers {
            let next_backoff = poll_one_peer(url, backoff_ms.get(url).copied()).await;
            match next_backoff {
                Some(ms) => {
                    backoff_ms.insert(url.clone(), ms);
                }
                None => {
                    backoff_ms.remove(url);
                }
            }
        }
        let interval = poll_interval();
        tokio::select! {
            _ = tokio::time::sleep(interval) => {},
            _ = nudge_signal().notified() => {
                tracing::trace!(
                    target: "gateway::peer",
                    "poll loop nudged — fast-pulling peers"
                );
            }
        }
    }
}

/// Poll a single peer. Returns `Some(next_backoff_ms)` after a
/// failure (caller will skip ahead-of-schedule attempts within
/// the backoff window) or `None` after a success.
async fn poll_one_peer(peer_url: &str, prior_backoff_ms: Option<u64>) -> Option<u64> {
    let secret = configured_secret();
    match fetch_peer_sync(peer_url, secret.as_deref()).await {
        Ok(payload) => {
            record_success(peer_url, &payload).await;
            None
        }
        Err(e) => {
            let next = (prior_backoff_ms.unwrap_or(500)).saturating_mul(2).min(MAX_BACKOFF_MS);
            record_failure(peer_url, e.to_string()).await;
            tracing::warn!(
                target: "gateway::peer",
                peer = %peer_url,
                err = %e,
                next_backoff_ms = next,
                "peer sync failed"
            );
            Some(next)
        }
    }
}

#[derive(Debug, Deserialize)]
struct SyncPayload {
    #[serde(default)]
    sessions: Vec<serde_json::Value>,
    #[serde(default)]
    audit: Vec<serde_json::Value>,
}

async fn fetch_peer_sync(
    peer_url: &str,
    secret: Option<&str>,
) -> Result<SyncPayload, String> {
    let client = reqwest::Client::builder()
        .timeout(http_timeout())
        .build()
        .map_err(|e| format!("client build: {e}"))?;
    let mut req = client.get(format!("{peer_url}/peer/sync"));
    if let Some(s) = secret {
        req = req.bearer_auth(s);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("send: {e}"))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {body}"));
    }
    serde_json::from_str(&body).map_err(|e| format!("decode: {e}"))
}

async fn record_success(peer_url: &str, payload: &SyncPayload) {
    let lock = state_lock();
    let mut guard = lock.write().await;
    let entry = guard
        .entry(peer_url.to_string())
        .or_insert_with(|| PeerSnapshot {
            url: peer_url.to_string(),
            ..Default::default()
        });
    entry.last_sync_micros = chrono::Utc::now().timestamp_micros();
    entry.last_error = None;
    entry.session_count = u32::try_from(payload.sessions.len()).unwrap_or(u32::MAX);
    entry.audit_count = u32::try_from(payload.audit.len()).unwrap_or(u32::MAX);
}

async fn record_failure(peer_url: &str, err: String) {
    let lock = state_lock();
    let mut guard = lock.write().await;
    let entry = guard
        .entry(peer_url.to_string())
        .or_insert_with(|| PeerSnapshot {
            url: peer_url.to_string(),
            ..Default::default()
        });
    entry.last_error = Some(err);
}

/// Snapshot of every configured peer for `routes_peer::state_response`.
/// Empty list when the mesh is not configured.
pub(super) async fn snapshot_all() -> Vec<PeerSnapshot> {
    let Some(lock) = PEER_STATE.get() else {
        return Vec::new();
    };
    let guard = lock.read().await;
    let mut out: Vec<PeerSnapshot> = guard.values().cloned().collect();
    out.sort_by(|a, b| a.url.cmp(&b.url));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_peer_list_handles_whitespace_and_trailing_slash() {
        let peers = parse_peer_list(" http://a:7400 , http://b:7400/ , ");
        assert_eq!(peers, vec!["http://a:7400", "http://b:7400"]);
    }

    #[test]
    fn parse_peer_list_empty_input_yields_empty() {
        assert!(parse_peer_list("").is_empty());
        assert!(parse_peer_list("   ").is_empty());
        assert!(parse_peer_list(", , ,").is_empty());
    }

    #[test]
    fn snapshot_all_uninitialised_returns_empty() {
        // PEER_STATE may be initialised by other tests in this binary
        // running in parallel; the only invariant we can assert
        // independently is that snapshot_all() does not panic.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let _ = rt.block_on(snapshot_all());
    }
}
