//! `argo skill verify` — verify provenance hashes of active skills.
use argo_skill::SkillStore;
use argo_types::{ArgoError, ArgoResult};

pub(super) async fn verify_all(skills: &SkillStore) -> ArgoResult<()> {
    use sha2::{Digest, Sha256};
    let active = skills.list_active().await?;
    if active.is_empty() {
        println!("(no active skills)");
        return Ok(());
    }
    let mut failures: Vec<String> = Vec::new();
    for s in &active {
        let mut h = Sha256::new();
        h.update(s.body.as_bytes());
        let computed = format!("sha256-{}", hex::encode(h.finalize()));
        if computed == s.frontmatter.provenance.hash {
            println!(
                "  ok       {} v{}",
                s.frontmatter.name, s.frontmatter.version
            );
        } else {
            failures.push(format!(
                "{} v{} stored={} computed={}",
                s.frontmatter.name, s.frontmatter.version, s.frontmatter.provenance.hash, computed
            ));
            println!(
                "  MISMATCH {} v{}",
                s.frontmatter.name, s.frontmatter.version
            );
        }
    }
    if failures.is_empty() {
        println!(
            "all {} skill(s) verified — provenance hashes match",
            active.len()
        );
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "{} of {} skill(s) failed verification:\n  {}",
            failures.len(),
            active.len(),
            failures.join("\n  ")
        )))
    }
}

pub(super) async fn verify_hash(skills: &SkillStore, name: &str) -> ArgoResult<()> {
    use sha2::{Digest, Sha256};
    let s = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    let mut h = Sha256::new();
    h.update(s.body.as_bytes());
    let computed = format!("sha256-{}", hex::encode(h.finalize()));
    let stored = &s.frontmatter.provenance.hash;
    if &computed == stored {
        println!(
            "skill '{name}' v{} — provenance hash ok ({})",
            s.frontmatter.version, computed
        );
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "skill '{name}' v{} — provenance hash MISMATCH\n  stored:   {stored}\n  computed: {computed}",
            s.frontmatter.version
        )))
    }
}
