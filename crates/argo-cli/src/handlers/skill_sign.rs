//! `argo skill sign` — produce a cosign-compatible signature for a
//! local skill manifest.

use crate::cli::cli_skill_sign_cmds::SkillSignCmd;
use argo_skill::sign::{build_signature_artifact, load_private_key, write_signature_artifact};
use argo_skill::{SkillPublishSource, SkillStore};
use argo_types::{ArgoError, ArgoResult};

pub async fn run(skills: &SkillStore, cmd: SkillSignCmd) -> ArgoResult<()> {
    if cmd.keyless {
        return Err(ArgoError::Provider(
            "keyless (Sigstore Fulcio) signing is deferred follow-up; pass --key <path> for v1"
                .into(),
        ));
    }
    let key_path = cmd
        .key
        .as_ref()
        .ok_or_else(|| ArgoError::Schema("--key <path> is required for v1 signing".into()))?;
    let source = load_source(skills, &cmd).await?;
    let signing = load_private_key(key_path)?;
    let tag = cmd.tag.clone().unwrap_or_else(|| source.version.clone());
    let sig =
        build_signature_artifact(&source.name, &source.version, &signing, source.raw.as_bytes());
    let path = write_signature_artifact(&cmd.registry, &sig, &tag)?;
    if cmd.json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "command": "sign",
                "name": sig.name,
                "version": sig.version,
                "tag": tag,
                "registry": cmd.registry,
                "payload_sha256": sig.payload_sha256,
                "signature_path": path.display().to_string(),
                "signed_at": sig.signed_at,
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "signed {} v{} (tag {tag}) → {}",
            sig.name,
            sig.version,
            path.display()
        );
        println!("  digest: {}", sig.payload_sha256);
    }
    Ok(())
}

async fn load_source(
    skills: &SkillStore,
    cmd: &SkillSignCmd,
) -> ArgoResult<SkillPublishSource> {
    if let Some(path) = &cmd.manifest {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
        return SkillPublishSource::from_raw(&raw);
    }
    let name = cmd
        .name
        .as_deref()
        .ok_or_else(|| ArgoError::Schema("either NAME or --manifest <path> is required".into()))?;
    let skill = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    SkillPublishSource::from_skill(&skill)
}
