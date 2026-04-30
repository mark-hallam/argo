//! `argo doctor` — config / workspace sanity report.
use crate::home::{db_path, resolve_home};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let toml = cwd.join("argo.toml");
    let toml_status = if toml.exists() { "OK" } else { "MISSING" };
    let cfg = ArgoConfig::load(&toml).unwrap_or_default();
    let home = resolve_home(&cwd).unwrap_or_else(|_| cfg.core.resolved_home());
    let home_status = if home.exists() {
        "OK"
    } else {
        "MISSING (will be created on first run)"
    };
    println!("argo doctor — workspace check");
    println!("  cwd:        {}", cwd.display());
    println!("  argo.toml:  {} ({})", toml.display(), toml_status);
    println!("  ARGO_HOME:  {} ({})", home.display(), home_status);
    println!("  profile:    {}", cfg.core.profile);
    println!(
        "  tenant:     {} (mode={})",
        cfg.tenant.default_id, cfg.tenant.mode
    );
    let bundle_dirs = ["skills", "personas", "cron", "rubrics"];
    let mut missing = Vec::new();
    for dir in bundle_dirs {
        if !cwd.join(dir).exists() {
            missing.push(dir);
        }
    }
    if missing.is_empty() {
        println!("  bundle dirs: all present");
    } else {
        println!(
            "  bundle dirs missing: {} (run `argo workspace init` to create)",
            missing.join(", ")
        );
    }
    let memfiles = ["MEMORY.md", "USER.md", "SOUL.md", "AGENTS.md"];
    let mut missing_m = Vec::new();
    for f in memfiles {
        if !cwd.join(f).exists() {
            missing_m.push(f);
        }
    }
    if missing_m.is_empty() {
        println!("  memory files: all present");
    } else {
        println!(
            "  memory files missing: {} (run `argo workspace init`)",
            missing_m.join(", ")
        );
    }
    db_health(&home).await;
    cron_health(&cwd);
    creds_health();
    Ok(())
}

fn cron_health(cwd: &std::path::Path) {
    let lock = cwd.join("cron").join(".tick.lock");
    if !lock.exists() {
        println!("  cron:       no lock file (daemon not started)");
        return;
    }
    // The lock is fs2-exclusive; if we can take it, no daemon is
    // running. We don't try to take it here (it would race with a
    // live daemon). Just report the file's presence + mtime.
    match std::fs::metadata(&lock) {
        Ok(m) => match m.modified() {
            Ok(mtime) => {
                let age = mtime
                    .elapsed()
                    .map(|d| format!("{}s ago", d.as_secs()))
                    .unwrap_or_else(|_| "unknown".into());
                println!(
                    "  cron:       lock present at {} (mtime {age})",
                    lock.display()
                );
            }
            Err(_) => {
                println!("  cron:       lock present at {}", lock.display());
            }
        },
        Err(_) => {
            println!("  cron:       lock metadata unreadable");
        }
    }
}

fn creds_health() {
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
    if found.is_empty() {
        println!("  creds:      none of {} are set", known.join(", "));
    } else {
        println!("  creds:      set: {}", found.join(", "));
        if !missing.is_empty() {
            println!("              unset: {}", missing.join(", "));
        }
    }
}

async fn db_health(home: &std::path::Path) {
    let db = db_path(home);
    if !db.exists() {
        println!("  db:         (no sessions.db yet — runs `argo workspace init`)");
        return;
    }
    let store = match SessionStore::open(&db).await {
        Ok(s) => s,
        Err(e) => {
            println!("  db:         OPEN FAILED ({e})");
            return;
        }
    };
    if let Err(e) = store.migrate().await {
        println!("  db:         MIGRATE FAILED ({e})");
        return;
    }
    let version = store.schema_version().await.unwrap_or(0);
    let stats = match store.db_stats().await {
        Ok(s) => s,
        Err(e) => {
            println!("  db:         schema v{version}, stats failed ({e})");
            return;
        }
    };
    let ws_status = match store.first_workspace().await {
        Ok(Some(_)) => "registered",
        Ok(None) => "MISSING — run `argo workspace init`",
        Err(_) => "lookup failed",
    };
    println!("  db:         schema v{version}  workspace={ws_status}");
    println!(
        "  rows:       sessions={} messages={} audit={} memory={} skills={}",
        stats.sessions, stats.messages, stats.audit_rows, stats.memory_entries, stats.skills
    );
}
