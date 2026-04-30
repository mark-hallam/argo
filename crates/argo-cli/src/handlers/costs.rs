//! `argo costs` — inspect or set hard $$$ ceilings.
use crate::cli::CostsCmd;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

pub fn run(cmd: Option<CostsCmd>) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml"))?;
    match cmd {
        None => show(&cfg, false),
        Some(CostsCmd::Show { json }) => show(&cfg, json),
        Some(CostsCmd::Cap {
            session,
            day,
            month,
            usd,
        }) => {
            // No flag selected → print help-ish message instead of
            // silently picking a scope.
            if !session && !day && !month {
                return Err(ArgoError::Provider(
                    "specify exactly one of --session / --day / --month".into(),
                ));
            }
            let micros = (usd * 1_000_000.0).round() as u64;
            let scope = if session {
                "session"
            } else if day {
                "day"
            } else {
                "month"
            };
            println!("would set per-{scope} cap to ${usd} ({micros} usd-micros);");
            println!(
                "  edit `argo.toml` directly under [budget]; the daemon hot-reloads on SIGHUP."
            );
            Ok(())
        }
    }
}

fn show(cfg: &ArgoConfig, json: bool) -> ArgoResult<()> {
    let ceilings = cfg.budget.to_cost_ceilings();
    if json {
        let payload = serde_json::json!({
            "per_session_usd_micros": ceilings.per_session_usd_micros,
            "per_day_usd_micros":     ceilings.per_day_usd_micros,
            "per_month_usd_micros":   ceilings.per_month_usd_micros,
            "notify_at_pct":          ceilings.notify_at_pct,
        });
        let serialized = serde_json::to_string(&payload)
            .map_err(|e| ArgoError::Provider(format!("serialize costs: {e}")))?;
        println!("{serialized}");
        return Ok(());
    }
    println!("cost ceilings (USD):");
    for (label, val) in [
        ("session", ceilings.per_session_usd_micros),
        ("day", ceilings.per_day_usd_micros),
        ("month", ceilings.per_month_usd_micros),
    ] {
        match val {
            Some(v) => println!("  per-{label:<7} ${:>10.2}", v as f64 / 1_000_000.0),
            None => println!("  per-{label:<7} (unlimited)"),
        }
    }
    println!(
        "notify_at_pct: {}",
        ceilings
            .notify_at_pct
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}
