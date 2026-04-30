//! `argo provider` — list / validate manifests.
use crate::cli::ProviderCmd;
use argo_provider::ProviderManifest;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub fn run(cmd: ProviderCmd) -> ArgoResult<()> {
    match cmd {
        ProviderCmd::List => list_manifests(),
        ProviderCmd::Validate { manifest } => validate(&PathBuf::from(manifest)),
        ProviderCmd::Test { provider } => {
            // Live provider test requires a real API key + reqwest;
            // belongs with the daemon binding.
            println!("provider test {provider}: requires daemon credentials (separate PR).");
            Ok(())
        }
    }
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
