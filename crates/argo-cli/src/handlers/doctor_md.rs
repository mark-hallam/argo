//! Markdown render of the `argo doctor` payload — extracted from
//! `doctor.rs` so that file stays under the 500-LOC budget. Same
//! fields as the text view but as a shareable status doc with
//! checkbox-style indicators.

pub(super) fn print_markdown(payload: &serde_json::Value) {
    print_workspace(payload);
    print_db_section(payload);
    print_audit_section(payload);
    print_runtime_section(payload);
    print_creds_section(payload);
}

fn print_workspace(payload: &serde_json::Value) {
    let cwd = payload["cwd"].as_str().unwrap_or("?");
    let toml_path = payload["argo_toml"]["path"].as_str().unwrap_or("?");
    let toml_ok = payload["argo_toml"]["ok"].as_bool().unwrap_or(false);
    let home_path = payload["argo_home"]["path"].as_str().unwrap_or("?");
    let home_ok = payload["argo_home"]["ok"].as_bool().unwrap_or(false);
    let profile = payload["profile"].as_str().unwrap_or("?");
    let tenant_id = payload["tenant"]["id"].as_str().unwrap_or("?");
    let tenant_mode = payload["tenant"]["mode"].as_str().unwrap_or("?");
    println!("# argo doctor");
    println!();
    println!("## Workspace");
    println!();
    println!("- **cwd**: `{cwd}`");
    println!(
        "- **argo.toml**: `{toml_path}` {}",
        if toml_ok { "✓" } else { "_(missing)_" }
    );
    println!(
        "- **ARGO_HOME**: `{home_path}` {}",
        if home_ok {
            "✓"
        } else {
            "_(missing — will be created on first run)_"
        }
    );
    println!("- **profile**: `{profile}`");
    println!("- **tenant**: `{tenant_id}` (mode=`{tenant_mode}`)");
    let bundle_missing: Vec<&str> = payload["bundle_dirs_missing"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    if bundle_missing.is_empty() {
        println!("- **bundle dirs**: all present ✓");
    } else {
        println!(
            "- **bundle dirs missing**: {} _(run `argo workspace init`)_",
            bundle_missing.join(", ")
        );
    }
    let memfiles_missing: Vec<&str> = payload["memory_files_missing"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    if memfiles_missing.is_empty() {
        println!("- **memory files**: all present ✓");
    } else {
        println!(
            "- **memory files missing**: {} _(run `argo workspace init`)_",
            memfiles_missing.join(", ")
        );
    }
}

fn print_db_section(payload: &serde_json::Value) {
    println!();
    println!("## Database");
    println!();
    let db = &payload["db"];
    let present = db.get("present").and_then(|b| b.as_bool()).unwrap_or(false);
    if !present {
        println!("- _(no sessions.db yet — run `argo workspace init`)_");
        return;
    }
    let version = db.get("schema_version").and_then(|n| n.as_u64()).unwrap_or(0);
    let ws = db
        .get("workspace_registered")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    println!(
        "- **schema**: v{version}, workspace {}",
        if ws { "registered ✓" } else { "**MISSING**" }
    );
    if let Some(rows) = db.get("rows") {
        let n = |k| rows.get(k).and_then(|n| n.as_u64()).unwrap_or(0);
        println!(
            "- **rows**: sessions={} messages={} audit={} memory={} skills={}",
            n("sessions"),
            n("messages"),
            n("audit"),
            n("memory"),
            n("skills")
        );
    }
}

fn print_audit_section(payload: &serde_json::Value) {
    println!();
    println!("## Audit chain");
    println!();
    let audit = &payload["audit"];
    if !audit["available"].as_bool().unwrap_or(false) {
        println!("- _(store not available)_");
        return;
    }
    let ok = audit["ok"].as_bool().unwrap_or(false);
    let rows = audit["rows"].as_u64().unwrap_or(0);
    let breaks = audit["breaks"].as_u64().unwrap_or(0);
    println!(
        "- **status**: {}",
        if ok { "OK ✓" } else { "**BROKEN**" }
    );
    println!("- **rows**: {rows}");
    println!("- **breaks**: {breaks}");
}

fn print_runtime_section(payload: &serde_json::Value) {
    println!();
    println!("## Daemon / runtime");
    println!();
    let gateway = &payload["gateway"];
    let running = gateway["running"].as_bool().unwrap_or(false);
    if running {
        let pid = gateway["pid"].as_u64().unwrap_or(0);
        println!("- **gateway**: RUNNING (pid=`{pid}`) ✓");
    } else {
        println!("- **gateway**: not running");
    }
    let routes = &payload["routes"];
    let total = routes["total"].as_u64().unwrap_or(0);
    let get = routes["get"].as_u64().unwrap_or(0);
    let post = routes["post"].as_u64().unwrap_or(0);
    println!("- **routes**: {total} endpoints ({get} GET, {post} POST)");
    let cron = &payload["cron"];
    if cron
        .get("daemon_started")
        .and_then(|b| b.as_bool())
        .unwrap_or(false)
    {
        println!("- **cron**: lock present ✓");
    } else {
        println!("- **cron**: no lock file (daemon not started)");
    }
}

fn print_creds_section(payload: &serde_json::Value) {
    println!();
    println!("## Credentials");
    println!();
    let creds = &payload["creds"];
    let set: Vec<String> = creds["set"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let unset: Vec<String> = creds["unset"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if set.is_empty() {
        println!("- **set**: _(none)_");
    } else {
        println!("- **set**: {}", set.join(", "));
    }
    if !unset.is_empty() {
        println!("- **unset**: {}", unset.join(", "));
    }
}
