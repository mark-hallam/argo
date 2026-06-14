//! Gateway-daemon and route checks for `argo doctor`.

use argo_config::ArgoConfig;
use std::path::Path;

/// Report whether the gateway daemon is running. Reads
/// `<ARGO_HOME>/gateway.pid`; if the file exists and the PID
/// is alive, returns `{running: true, pid: N}`. Otherwise
/// `{running: false}` (the file may be present but stale).
/// The live-PID probe is platform-specific — see [`pid_is_alive`].
pub(super) fn gateway_status(home: &Path) -> serde_json::Value {
    let pid_path = home.join("gateway.pid");
    let Ok(s) = std::fs::read_to_string(&pid_path) else {
        return serde_json::json!({"running": false});
    };
    let Ok(pid) = s.trim().parse::<u32>() else {
        return serde_json::json!({"running": false});
    };
    serde_json::json!({
        "running": pid_is_alive(pid),
        "pid": pid,
    })
}

/// Side-effect-free liveness probe for `pid`.
///
/// On unix, `kill -0 <pid>` (exit 0 → live, !=0 → gone or not ours) — no `libc`
/// dependency needed for the one call. On Windows there is no `kill`; the
/// built-in `tasklist /FI "PID eq <pid>"` lists a CSV row for a live process and
/// prints `INFO: No tasks...` otherwise, so a live process shows the quoted pid
/// in stdout. Previously this hardcoded `kill`, which fails `NotFound` on
/// Windows whenever a `gateway.pid` exists (latent — #2931's smoke passed only
/// because no pid file was present). See `win-cli-doctor-gateway-kill-portability`.
fn pid_is_alive(pid: u32) -> bool {
    #[cfg(not(windows))]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains(&format!("\"{pid}\""))
            })
            .unwrap_or(false)
    }
}

pub(super) fn print_gateway(v: &serde_json::Value) {
    let running = v["running"].as_bool().unwrap_or(false);
    if running {
        let pid = v["pid"].as_u64().unwrap_or(0);
        println!("  gateway:    RUNNING (pid={pid})");
    } else {
        println!("  gateway:    not running");
    }
}

pub(super) async fn gateway_health_probe(gateway: &serde_json::Value) -> serde_json::Value {
    if !gateway["running"].as_bool().unwrap_or(false) {
        return serde_json::json!({"reachable": false, "reason": "gateway not running"});
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    let url = format!("http://127.0.0.1:{}/health", cfg.gateway.port);
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()
    else {
        return serde_json::json!({"reachable": false, "reason": "client_build_failed"});
    };
    match client.get(&url).send().await {
        Ok(r) => {
            let code = r.status().as_u16();
            let body =
                serde_json::from_str::<serde_json::Value>(&r.text().await.unwrap_or_default())
                    .unwrap_or(serde_json::Value::Null);
            serde_json::json!({"reachable": true, "status_code": code, "body": body})
        }
        Err(e) => serde_json::json!({"reachable": false, "reason": format!("{e}")}),
    }
}

pub(super) fn print_health(v: &serde_json::Value) {
    if v["reachable"].as_bool().unwrap_or(false) {
        let code = v["status_code"].as_u64().unwrap_or(0);
        let s = v["body"]["status"].as_str().unwrap_or("?");
        println!("  /health:    HTTP {code}  status={s}");
    } else {
        println!(
            "  /health:    {}",
            v["reason"].as_str().unwrap_or("unreachable")
        );
    }
}

/// Reads the in-binary route catalog and reports counts by
/// verb. Pure data — no daemon required.
pub(super) fn routes_count() -> serde_json::Value {
    let (_status, body) = crate::handlers::gateway::routes_catalog::routes_response(None);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).unwrap_or(serde_json::Value::Array(Vec::new()));
    let arr = parsed.as_array().cloned().unwrap_or_default();
    let total = arr.len();
    let get = arr
        .iter()
        .filter(|r| r["method"].as_str() == Some("GET"))
        .count();
    let post = arr
        .iter()
        .filter(|r| r["method"].as_str() == Some("POST"))
        .count();
    serde_json::json!({
        "total": total,
        "get": get,
        "post": post,
    })
}

pub(super) fn print_routes(v: &serde_json::Value) {
    let total = v["total"].as_u64().unwrap_or(0);
    let get = v["get"].as_u64().unwrap_or(0);
    let post = v["post"].as_u64().unwrap_or(0);
    println!("  routes:     {total} endpoints ({get} GET, {post} POST)");
}

#[cfg(test)]
mod tests {
    use super::pid_is_alive;

    /// The running test process is, by definition, alive. Exercises the real
    /// liveness probe on BOTH platforms (`tasklist` on Windows, `kill -0` on
    /// unix) — the regression guard for the hardcoded-`kill` Windows break
    /// (`win-cli-doctor-gateway-kill-portability`).
    #[test]
    fn current_process_reads_as_alive() {
        assert!(
            pid_is_alive(std::process::id()),
            "the running test process must read as alive on this platform"
        );
    }

    /// A PID this high is never assigned on either platform, so the probe must
    /// report it dead (not error out / falsely report alive).
    #[test]
    fn improbable_pid_reads_as_dead() {
        assert!(!pid_is_alive(4_294_967_294));
    }
}
