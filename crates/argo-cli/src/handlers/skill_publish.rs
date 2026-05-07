//! `argo skill publish` — publish a skill to the mock OCI marketplace.

use crate::cli::SkillPublishCmd;
use argo_skill::{mock_registry_root, publish_mock_skill, SkillPublishSource, SkillStore};
use argo_types::{ArgoError, ArgoResult};

pub async fn run(skills: &SkillStore, args: SkillPublishCmd) -> ArgoResult<()> {
    let root = mock_registry_root(&args.registry)?;
    let source = load_source(skills, &args).await?;
    if source.name != args.name {
        return Err(ArgoError::Schema(format!(
            "manifest name '{}' does not match requested skill '{}'",
            source.name, args.name
        )));
    }
    let tag = args.tag.clone().unwrap_or_else(|| source.version.clone());
    let record = publish_mock_skill(&root, &source, &tag, args.dry_run)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "command": "publish",
                "registry": args.registry,
                "record": record,
            }))
            .unwrap_or_default()
        );
    } else if args.dry_run {
        println!(
            "would publish {}:{} to {}",
            record.name, record.tag, args.registry
        );
        println!("  digest: {}", record.digest);
        println!("  bytes:  {}", record.bytes);
    } else {
        println!(
            "published {}:{} to {}",
            record.name, record.tag, args.registry
        );
        println!("  manifest: {}", record.manifest_path);
        println!("  blob:     {}", record.blob_path);
        println!("  digest:   {}", record.digest);
    }
    Ok(())
}

async fn load_source(
    skills: &SkillStore,
    args: &SkillPublishCmd,
) -> ArgoResult<SkillPublishSource> {
    if let Some(path) = &args.manifest {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
        return SkillPublishSource::from_raw(&raw);
    }
    let skill = skills
        .find_active_by_name(&args.name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{}'", args.name)))?;
    SkillPublishSource::from_skill(&skill)
}
