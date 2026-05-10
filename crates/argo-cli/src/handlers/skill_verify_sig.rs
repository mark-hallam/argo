//! `argo skill verify-sig` — verify a published skill against its
//! detached cosign-style signature artifact.

use crate::cli::cli_skill_sign_cmds::SkillVerifySigCmd;
use argo_skill::publish::mock_registry_root;
use argo_skill::sign::{
    load_public_key, read_signature_artifact, verify_skill_signature, SkillVerifyOutcome,
};
use argo_types::{ArgoError, ArgoResult};
use std::fs;

pub fn run(cmd: SkillVerifySigCmd) -> ArgoResult<()> {
    let key_path = cmd.key.as_ref().ok_or_else(|| {
        ArgoError::Schema("--key <path> is required (keyless verify is follow-up)".into())
    })?;
    let (name, tag) = split_ref(&cmd.reference)?;
    let verifying = load_public_key(key_path)?;
    let payload = read_published_skill_md(&cmd.registry, &name, &tag)?;
    let outcome = match payload.as_ref() {
        Some(bytes) => verify_skill_signature(&cmd.registry, &name, &tag, &verifying, bytes)?,
        None => match read_signature_artifact(&cmd.registry, &name, &tag)? {
            Some(_) => SkillVerifyOutcome::Fail(
                "no published SKILL.md bundle found at this tag — cannot reconstruct payload"
                    .into(),
            ),
            None => SkillVerifyOutcome::Missing,
        },
    };
    let (label, exit_err) = match &outcome {
        SkillVerifyOutcome::Ok => ("OK", None),
        SkillVerifyOutcome::Fail(reason) => ("FAIL", Some(reason.clone())),
        SkillVerifyOutcome::Missing => ("MISSING", None),
    };
    if cmd.json {
        let payload_present = payload.is_some();
        let reason = exit_err.clone();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": matches!(outcome, SkillVerifyOutcome::Ok),
                "command": "verify-sig",
                "reference": cmd.reference,
                "registry": cmd.registry,
                "outcome": label,
                "payload_present": payload_present,
                "reason": reason,
            }))
            .unwrap_or_default()
        );
    } else {
        match &outcome {
            SkillVerifyOutcome::Ok => {
                println!("OK    {}:{tag} verified against {}", name, key_path.display());
            }
            SkillVerifyOutcome::Fail(reason) => {
                println!("FAIL  {}:{tag} — {reason}", name);
            }
            SkillVerifyOutcome::Missing => {
                println!("MISSING  no signature artifact for {}:{tag}", name);
            }
        }
    }
    Ok(())
}

fn split_ref(reference: &str) -> ArgoResult<(String, String)> {
    let (name, tag) = reference.rsplit_once(':').ok_or_else(|| {
        ArgoError::Schema(format!("skill ref '{reference}' must be <name>:<tag>"))
    })?;
    if name.is_empty() || tag.is_empty() {
        return Err(ArgoError::Schema(format!(
            "skill ref '{reference}' must be <name>:<tag>"
        )));
    }
    Ok((name.to_string(), tag.to_string()))
}

fn read_published_skill_md(registry: &str, name: &str, tag: &str) -> ArgoResult<Option<Vec<u8>>> {
    let root = mock_registry_root(registry)?;
    let bundle = root.join("oci").join(name).join(tag).join("bundle.tar");
    if !bundle.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&bundle)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", bundle.display())))?;
    Ok(extract_skill_md(&bytes))
}

/// Extract the first `SKILL.md` entry from the ustar bundle written by
/// `argo skill publish`. The bundle is the minimal hand-rolled tar
/// produced in `crates/argo-skill/src/publish.rs::build_skill_tar`.
fn extract_skill_md(tar: &[u8]) -> Option<Vec<u8>> {
    let mut cursor = 0usize;
    while cursor + 512 <= tar.len() {
        let header = &tar[cursor..cursor + 512];
        if header.iter().all(|b| *b == 0) {
            return None;
        }
        let name_end = header[..100].iter().position(|b| *b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&header[..name_end]).ok()?;
        let size_bytes = &header[124..136];
        let size_str = std::str::from_utf8(size_bytes).ok()?.trim_end_matches('\0');
        let size = u64::from_str_radix(size_str.trim(), 8).ok()? as usize;
        let body_start = cursor + 512;
        let body_end = body_start + size;
        if body_end > tar.len() {
            return None;
        }
        if name == "SKILL.md" {
            return Some(tar[body_start..body_end].to_vec());
        }
        let padded = body_end + ((512 - size % 512) % 512);
        cursor = padded;
    }
    None
}
