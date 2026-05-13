//! Environment-level checks for `argo doctor` (cron lock, credential env vars).

use std::path::Path;

pub(super) fn cron_status(cwd: &Path) -> serde_json::Value {
    let lock = cwd.join("cron").join(".tick.lock");
    if !lock.exists() {
        return serde_json::json!({"daemon_started": false});
    }
    let age_s = std::fs::metadata(&lock)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .map(|d| d.as_secs());
    serde_json::json!({
        "daemon_started": true,
        "lock_path": lock.display().to_string(),
        "lock_age_seconds": age_s,
    })
}

pub(super) fn print_cron(v: &serde_json::Value) {
    if !v
        .get("daemon_started")
        .and_then(|b| b.as_bool())
        .unwrap_or(false)
    {
        println!("  cron:       no lock file (daemon not started)");
        return;
    }
    let path = v.get("lock_path").and_then(|s| s.as_str()).unwrap_or("?");
    let age = v.get("lock_age_seconds").and_then(|n| n.as_u64());
    match age {
        Some(s) => println!("  cron:       lock present at {path} (mtime {s}s ago)"),
        None => println!("  cron:       lock present at {path}"),
    }
}

pub(super) fn creds_status() -> serde_json::Value {
    let known = [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "AWS_ACCESS_KEY_ID",
        "ELEVENLABS_API_KEY",
    ];
    let mut found: Vec<&str> = Vec::new();
    let mut missing: Vec<&str> = Vec::new();
    for k in known {
        if std::env::var_os(k).is_some_and(|v| !v.is_empty()) {
            found.push(k);
        } else {
            missing.push(k);
        }
    }
    serde_json::json!({"set": found, "unset": missing})
}

pub(super) fn print_creds(v: &serde_json::Value) {
    let set: Vec<String> = v
        .get("set")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let unset: Vec<String> = v
        .get("unset")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if set.is_empty() {
        println!("  creds:      none of {} are set", unset.join(", "));
    } else {
        println!("  creds:      set: {}", set.join(", "));
        if !unset.is_empty() {
            println!("              unset: {}", unset.join(", "));
        }
    }
}
