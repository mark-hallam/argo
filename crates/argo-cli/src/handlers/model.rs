//! `argo model` — list / show / set / test models from manifests.
use crate::cli::ModelCmd;
use argo_provider::ProviderManifest;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub fn run(cmd: ModelCmd) -> ArgoResult<()> {
    match cmd {
        ModelCmd::List { json, format } => list(json, &format),
        ModelCmd::Set { model } => {
            println!(
                "argo model set {model}: edit \\[provider.<name>\\].model in argo.toml; daemon hot-reloads on SIGHUP."
            );
            Ok(())
        }
        ModelCmd::Test { model } => {
            println!(
                "argo model test {model}: requires daemon credentials + a live provider call (separate PR)."
            );
            Ok(())
        }
    }
}

fn list(json: bool, format: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for dir in [cwd.join("manifests").join("providers")] {
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", dir.display())))?
        {
            let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
            let p = e.path();
            if p.extension().is_some_and(|x| x == "toml") {
                paths.push(p);
            }
        }
    }
    paths.sort();
    let manifests: Vec<ProviderManifest> = paths
        .iter()
        .filter_map(|p| {
            let body = std::fs::read_to_string(p).ok()?;
            ProviderManifest::from_toml_str(&body).ok()
        })
        .collect();
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            let mut all_rows: Vec<serde_json::Value> = Vec::new();
            for m in &manifests {
                for spec in &m.models {
                    all_rows.push(serde_json::json!({
                        "provider": m.provider.id,
                        "family": format!("{:?}", m.provider.family),
                        "id": spec.id,
                        "context_window": spec.context_window,
                        "tier": spec.tier,
                        "price_per_1k_in": spec.price_per_1k_in,
                        "price_per_1k_out": spec.price_per_1k_out,
                    }));
                }
            }
            for row in &all_rows {
                println!("{}", serde_json::to_string(row).unwrap_or_default());
            }
            return Ok(());
        }
        "markdown" => {
            let total: usize = manifests.iter().map(|m| m.models.len()).sum();
            println!("# Models ({total} across {} provider(s))", manifests.len());
            println!();
            if total == 0 {
                println!("_(no manifests / models found)_");
                return Ok(());
            }
            println!("| provider | id | ctx | tier | input/1k | output/1k |");
            println!("| --- | --- | ---: | --- | ---: | ---: |");
            for m in &manifests {
                for spec in &m.models {
                    let ctx = spec
                        .context_window
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".into());
                    let tier = spec.tier.as_deref().unwrap_or("-");
                    let in_p = price_field(&spec.price_per_1k_in);
                    let out_p = price_field(&spec.price_per_1k_out);
                    println!(
                        "| `{}` | `{}` | {} | `{tier}` | {in_p} | {out_p} |",
                        m.provider.id, spec.id, ctx
                    );
                }
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    if paths.is_empty() {
        println!(
            "(no provider manifests under {}/manifests/providers/)",
            cwd.display()
        );
        return Ok(());
    }
    for p in paths {
        let body = std::fs::read_to_string(&p)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
        match ProviderManifest::from_toml_str(&body) {
            Ok(m) => render_provider(&m),
            Err(e) => println!("# {} (INVALID — {e})", p.display()),
        }
    }
    Ok(())
}

fn render_provider(m: &ProviderManifest) {
    println!("# {} ({:?})", m.provider.id, m.provider.family);
    if m.models.is_empty() {
        println!("  (no [[models]] entries)");
    } else {
        println!(
            "  {id:<32} {ctx:<8} {tier:<10} input/1k    output/1k",
            id = "model id",
            ctx = "ctx",
            tier = "tier"
        );
        for spec in &m.models {
            let ctx = spec
                .context_window
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".into());
            let tier = spec.tier.as_deref().unwrap_or("-");
            let in_price = price_field(&spec.price_per_1k_in);
            let out_price = price_field(&spec.price_per_1k_out);
            println!(
                "  {:<32} {:<8} {:<10} {:<11} {}",
                spec.id, ctx, tier, in_price, out_price
            );
        }
    }
    if let Some(disc) = &m.discovery {
        println!(
            "  discovery: endpoint={}  prefer={:?}  refresh={}h",
            disc.endpoint, disc.prefer, disc.refresh_interval_hours
        );
    }
}

fn price_field(value: &Option<f64>) -> String {
    match value {
        Some(price) => format!("${price:.4}"),
        None => "-".into(),
    }
}
