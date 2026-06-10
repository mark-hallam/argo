//! `argo gateway` command-body helpers split out of `mod.rs` (mechanical,
//! no behaviour change) to keep the gateway module root under the per-file
//! LOC budget. `mod.rs` owns the giant submodule-declaration block, so the
//! `status` reporter and the bind-flag validator live here.
//!
//! LOC headroom: keep new gateway command-body helpers in sibling modules,
//! not back in `mod.rs` (which is dominated by `mod` declarations).

use super::boot;
use super::lifecycle::pid_is_alive;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

pub(super) fn resolve_non_loopback_override(
    bind_non_loopback: bool,
    accept_warning: bool,
    deprecated_insecure_bind: bool,
) -> ArgoResult<bool> {
    if deprecated_insecure_bind {
        return Err(ArgoError::Schema(
            "--insecure-bind is deprecated; use --bind-non-loopback and pass \
             --accept-insecure-bind-warning to confirm"
                .into(),
        ));
    }
    if bind_non_loopback && !accept_warning {
        return Err(ArgoError::Schema(
            "--bind-non-loopback requires --accept-insecure-bind-warning".into(),
        ));
    }
    if accept_warning && !bind_non_loopback {
        return Err(ArgoError::Schema(
            "--accept-insecure-bind-warning requires --bind-non-loopback".into(),
        ));
    }
    Ok(bind_non_loopback)
}

pub(super) fn status(cfg: &ArgoConfig, json: bool) -> ArgoResult<()> {
    // Best-effort: read the PID file so the JSON form can report
    // a real running state, not just the config bits.
    let pid_alive = boot::pid_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .and_then(|s| s.trim().parse::<i32>().ok())
        .map(pid_is_alive)
        .unwrap_or(false);
    if json {
        let payload = serde_json::json!({
            "enabled": cfg.gateway.enabled,
            "adapters": cfg.gateway.adapters,
            "running": pid_alive,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
        return Ok(());
    }
    println!("gateway:");
    println!(
        "  enabled:   {} (set [gateway].enabled in argo.toml)",
        cfg.gateway.enabled
    );
    println!("  running:   {pid_alive}");
    if cfg.gateway.adapters.is_empty() {
        println!("  adapters:  (none configured)");
    } else {
        println!("  adapters:");
        for a in &cfg.gateway.adapters {
            println!("    - {a}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod bind_flag_tests {
    use super::resolve_non_loopback_override;

    #[test]
    fn two_flag_override_required() {
        assert!(resolve_non_loopback_override(false, false, false).is_ok_and(|v| !v));
        assert!(resolve_non_loopback_override(true, true, false).is_ok_and(|v| v));
        assert!(resolve_non_loopback_override(true, false, false).is_err());
        assert!(resolve_non_loopback_override(false, true, false).is_err());
    }

    #[test]
    fn deprecated_insecure_bind_is_rejected() {
        let msg = resolve_non_loopback_override(false, false, true)
            .expect_err("deprecated flag must fail")
            .to_string();
        assert!(msg.contains("--insecure-bind is deprecated"), "{msg}");
        assert!(msg.contains("--bind-non-loopback"), "{msg}");
        assert!(msg.contains("--accept-insecure-bind-warning"), "{msg}");
    }
}
