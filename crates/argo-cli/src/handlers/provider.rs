//! `argo provider` — list / validate manifests.
use crate::cli::ProviderCmd;
use argo_provider::ProviderManifest;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub fn run(cmd: ProviderCmd) -> ArgoResult<()> {
    match cmd {
        ProviderCmd::List => list_manifests(),
        ProviderCmd::Show { id, json } => show(&id, json),
        ProviderCmd::Validate { manifest } => validate(&PathBuf::from(manifest)),
        ProviderCmd::ValidateAll => validate_all(),
        ProviderCmd::Test { provider } => {
            // Live provider test requires a real API key + reqwest;
            // belongs with the daemon binding.
            println!("provider test {provider}: requires daemon credentials (separate PR).");
            Ok(())
        }
    }
}

fn validate_all() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let dir = cwd.join("manifests").join("providers");
    if !dir.exists() {
        return Err(ArgoError::Provider(format!(
            "no manifests at {} (run from a workspace with manifests/providers/)",
            dir.display()
        )));
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", dir.display())))?
    {
        let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
        let p = e.path();
        if p.extension().is_some_and(|x| x == "toml") {
            paths.push(p);
        }
    }
    paths.sort();
    if paths.is_empty() {
        println!("(no manifests under {})", dir.display());
        return Ok(());
    }
    let mut failures: Vec<(PathBuf, String)> = Vec::new();
    for p in &paths {
        let body = std::fs::read_to_string(p)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
        match ProviderManifest::from_toml_str(&body) {
            Ok(m) => println!(
                "  {:<24} ok ({:?})",
                p.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
                m.provider.family
            ),
            Err(e) => failures.push((p.clone(), e.to_string())),
        }
    }
    if failures.is_empty() {
        println!(
            "{} manifest(s) validated under {}",
            paths.len(),
            dir.display()
        );
        Ok(())
    } else {
        for (p, msg) in &failures {
            eprintln!("  {} INVALID — {msg}", p.display());
        }
        Err(ArgoError::Provider(format!(
            "{} of {} manifest(s) failed validation",
            failures.len(),
            paths.len()
        )))
    }
}

fn show(provider_id: &str, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let manifests_dir = cwd.join("manifests").join("providers");
    if !manifests_dir.exists() {
        return Err(ArgoError::Provider(format!(
            "no manifests at {} (run from a workspace with manifests/providers/)",
            manifests_dir.display()
        )));
    }
    for entry in std::fs::read_dir(&manifests_dir)
        .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", manifests_dir.display())))?
    {
        let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
        let p = e.path();
        if p.extension().is_none_or(|x| x != "toml") {
            continue;
        }
        let body = std::fs::read_to_string(&p)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
        let m = match ProviderManifest::from_toml_str(&body) {
            Ok(m) => m,
            Err(_) => continue, // skip invalid manifests in show-by-id flow
        };
        if m.provider.id == provider_id {
            if json {
                let serialized = serde_json::to_string(&m)
                    .map_err(|e| ArgoError::Provider(format!("serialize manifest: {e}")))?;
                println!("{serialized}");
                return Ok(());
            }
            return render(&m);
        }
    }
    Err(ArgoError::Provider(format!(
        "no provider with id '{provider_id}' under {}",
        manifests_dir.display()
    )))
}

fn render(m: &ProviderManifest) -> ArgoResult<()> {
    println!("# {} ({:?})", m.provider.id, m.provider.family);
    println!("  base_url:   {}", m.provider.base_url);
    println!(
        "  auth:       header={} format={} env={}",
        m.auth.header_name, m.auth.header_format, m.auth.api_key_env
    );
    println!("  streaming:  {:?}", m.transport.streaming);
    println!(
        "  timeouts:   connect={}ms read={}ms max_retries={}",
        m.transport.connect_timeout_ms, m.transport.read_timeout_ms, m.transport.max_retries
    );
    println!("  tool_calls: {:?}", m.tool_calls.format);
    if !m.models.is_empty() {
        println!("  models:");
        for spec in &m.models {
            let tier = spec.tier.as_deref().unwrap_or("-");
            println!(
                "    {:<32} ctx={:<8} tier={tier}",
                spec.id,
                spec.context_window
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".into())
            );
        }
    }
    if let Some(d) = m.discovery.as_ref() {
        println!(
            "  discovery:  endpoint={} prefer={:?} refresh={}h",
            d.endpoint, d.prefer, d.refresh_interval_hours
        );
    }
    if let Some(r) = m.reasoning.as_ref() {
        println!(
            "  reasoning:  field={} preserve_across_turns={}",
            r.content_field, r.preserve_across_turns
        );
    }
    Ok(())
}

fn list_manifests() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    // Spec layout: workspace `manifests/providers/*.toml` for tenant
    // overrides; the bundled v1 set lives in the source repo at
    // `manifests/providers/`. Look in both, the workspace dir taking
    // precedence.
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
            Ok(m) => println!(
                "  {:<24} {:?}  base={}",
                m.provider.id, m.provider.family, m.provider.base_url
            ),
            Err(e) => println!("  {:<24} INVALID  ({e})", p.display()),
        }
    }
    Ok(())
}

fn validate(path: &std::path::Path) -> ArgoResult<()> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let m = ProviderManifest::from_toml_str(&body)?;
    println!("manifest ok: {} ({:?})", m.provider.id, m.provider.family);
    println!("  base_url:  {}", m.provider.base_url);
    println!("  streaming: {:?}", m.transport.streaming);
    Ok(())
}
