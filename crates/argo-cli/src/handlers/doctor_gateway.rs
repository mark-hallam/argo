//! Gateway-daemon and route checks for `argo doctor`.

use argo_config::ArgoConfig;
use std::path::Path;

/// Report whether the gateway daemon is running. Reads
/// `<ARGO_HOME>/gateway.pid`; if the file exists and the PID
/// is alive, returns `{running: true, pid: N}`. Otherwise
/// `{running: false}` (the file may be present but stale —
/// the live-PID check shells out to `kill -0` so we don't
/// pull in libc just for this one call).
pub(super) fn gateway_status(home: &Path) -> serde_json::Value {
    let pid_path = home.join("gateway.pid");
    let Ok(s) = std::fs::read_to_string(&pid_path) else {
        return serde_json::json!({"running": false});
    };
    let Ok(pid) = s.trim().parse::<u32>() else {
        return serde_json::json!({"running": false});
    };
    // `kill -0 <pid>` exit 0 → live, !=0 → gone or not ours.
    // Safe, side-effect-free probe; works on every Unix.
    let alive = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    serde_json::json!({
        "running": alive,
        "pid": pid,
    })
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
            let body = serde_json::from_str::<serde_json::Value>(
                &r.text().await.unwrap_or_default(),
            )
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
