//! `argo packs` — install and manage vertical template packs.

use argo_types::{ArgoError, ArgoResult};
use argo_vertical_packs::{InstalledPack, PackLoader};
use clap::Subcommand;
use serde_json::json;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum PacksCmd {
    /// Install a pack release tarball into the local cache.
    Install {
        name: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// List cached packs.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Refresh one cached pack version.
    Update {
        name: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Point a pack name at a local checkout.
    DevLink {
        name: String,
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Remove a dev-link and cached pack versions.
    Remove {
        name: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(cmd: PacksCmd) -> ArgoResult<()> {
    let loader = PackLoader::default_loader()?;
    match cmd {
        PacksCmd::Install {
            name,
            version,
            base_url,
            force,
            json,
        } => {
            let row = loader
                .install(&name, &version, base_url.as_deref(), force)
                .await?;
            emit_pack("installed", &row, json)
        }
        PacksCmd::List { json } => emit_list(&loader.list()?, json),
        PacksCmd::Update {
            name,
            version,
            base_url,
            json,
        } => {
            let row = loader
                .install(&name, &version, base_url.as_deref(), true)
                .await?;
            emit_pack("updated", &row, json)
        }
        PacksCmd::DevLink { name, path, json } => {
            let row = loader.dev_link(&name, &path)?;
            emit_pack("dev_linked", &row, json)
        }
        PacksCmd::Remove {
            name,
            version,
            yes,
            json,
        } => {
            if !yes {
                return Err(ArgoError::Schema(
                    "refusing to remove pack without --yes".into(),
                ));
            }
            let removed = loader.remove(&name, version.as_deref())?;
            if json {
                println!("{}", encode(&json!({"removed": removed, "name": name}))?);
            } else {
                println!("removed {removed} pack entry for {name}");
            }
            Ok(())
        }
    }
}

fn emit_pack(action: &str, row: &InstalledPack, json_out: bool) -> ArgoResult<()> {
    if json_out {
        println!("{}", encode(&json!({"action": action, "pack": row}))?);
    } else {
        println!(
            "{action} {} {} at {}",
            row.name,
            row.version,
            row.root.display()
        );
    }
    Ok(())
}

fn emit_list(rows: &[InstalledPack], json_out: bool) -> ArgoResult<()> {
    if json_out {
        println!("{}", encode(&json!({"packs": rows}))?);
    } else if rows.is_empty() {
        println!("(no vertical packs installed)");
    } else {
        for row in rows {
            let marker = if row.dev_link { " dev-link" } else { "" };
            println!(
                "{} {}{} {}",
                row.name,
                row.version,
                marker,
                row.root.display()
            );
        }
    }
    Ok(())
}

fn encode(value: &serde_json::Value) -> ArgoResult<String> {
    serde_json::to_string(value)
        .map_err(|e| ArgoError::Invariant(format!("serialize packs JSON: {e}")))
}
