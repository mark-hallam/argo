//! Unit tests for `turn_skills` — Stream
//! `mission-skill-session-boot-wire` (M6 precursor 3/3).
//!
//! Six tests, one per acceptance bullet in the brief:
//! 1. empty list ⇒ block omitted entirely
//! 2. populated list ⇒ renders the documented markdown
//! 3. injected position ⇒ between persona / body-skill section and
//!    the memory composition appended after
//! 4. lookup error ⇒ `tracing::warn!` + omit (session boot succeeds)
//! 5. legacy worker (`trust_grade = 0`) ⇒ only unrestricted skills
//! 6. markdown format pinned byte-for-byte
#![cfg(test)]

use super::turn_skills::{
    compose_available_skills_block, inject_skills_block, render_available_skills_block,
};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_skill::{Skill, SkillStore, TrustGrade};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

fn skill_md(name: &str, version: &str, description: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: \"{description}\"\n\
         provenance:\n  source: builtin\n  hash: sha256-x\n---\nbody",
    )
}

async fn install_skill(skills: &SkillStore, name: &str, version: &str, desc: &str) -> Skill {
    skills.install(&skill_md(name, version, desc), None).await.unwrap()
}

async fn fresh_skill_store() -> SkillStore {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    SkillStore::new(store)
}

#[tokio::test]
async fn empty_list_omits_block() {
    assert!(render_available_skills_block(&[]).is_none());
}

#[tokio::test]
async fn populated_list_renders_correctly() {
    let skills = fresh_skill_store().await;
    let s1 = install_skill(&skills, "alpha", "0.1.0", "first skill").await;
    let s2 = install_skill(&skills, "beta", "1.2.3", "second skill").await;
    let block = render_available_skills_block(&[s1, s2]).expect("two skills ⇒ block");
    assert!(block.starts_with("## Available skills\n"));
    assert!(block.contains("- **alpha** (v0.1.0): first skill\n"));
    assert!(block.contains("- **beta** (v1.2.3): second skill\n"));
}

#[tokio::test]
async fn injected_position_is_after_persona_before_memory() {
    // Simulate the buffer that `build_system_prompt_with_ab` hands to
    // `inject_skills_block`: persona (vertical prompt is prepended
    // downstream) + workspace-skill body section. Memory composition
    // is appended AFTER inject_skills_block. The assertion: persona
    // < block < memory in the final string.
    let mut buf = String::from("PERSONA\n\n---\n\n## body-skill (v1)\nguidance\n\n---\n\n");
    let block = "## Available skills\n- **x** (v0.1.0): pinned\n";
    inject_skills_block(&mut buf, block);
    buf.push_str("\nMEMORY");

    let persona_idx = buf.find("PERSONA").unwrap();
    let body_idx = buf.find("## body-skill").unwrap();
    let block_idx = buf.find("## Available skills").unwrap();
    let memory_idx = buf.find("MEMORY").unwrap();
    assert!(persona_idx < body_idx);
    assert!(body_idx < block_idx);
    assert!(block_idx < memory_idx);
}

#[tokio::test]
async fn error_from_lookup_warns_and_skips() {
    // Worker id with NO matching row in IdentityStore: trust_grade
    // returns Ok(None), available_for_worker converts that to a hard
    // error, compose_available_skills_block must log + return None
    // rather than propagate the error.
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let identity = IdentityStore::new(&store);
    identity.ensure_schema().await.unwrap();

    let tenant = TenantId::new();
    let stray = argo_store::WorkerId(uuid::Uuid::now_v7());
    let block = compose_available_skills_block(&store, tenant, Some(stray)).await;
    assert!(block.is_none(), "missing worker ⇒ None, got {block:?}");
}

#[tokio::test]
async fn legacy_worker_treated_as_lowest_grade() {
    // Worker.draft() defaults trust_grade to 0 (legacy / fresh ladder
    // floor — migration 50). With one NULL-floor and one Tenant-floor
    // skill installed, only the NULL-floor skill should surface.
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let identity = IdentityStore::new(&store);
    let tenant = TenantId::new();
    let worker = Worker::draft(
        tenant,
        "gtm",
        Persona::named("Maya", "op"),
        Role::new("Sell."),
    );
    identity.create_worker(&worker).await.unwrap();

    let skills = SkillStore::new(Arc::clone(&store));
    skills.install(&skill_md("open", "0.1.0", "no floor"), None).await.unwrap();
    skills.install(&skill_md("locked", "0.1.0", "tenant only"), None).await.unwrap();
    skills
        .set_min_trust_grade("locked", Some(TrustGrade::Tenant))
        .await
        .unwrap();

    let store_worker_id = argo_store::WorkerId(worker.id.0);
    let block = compose_available_skills_block(&store, tenant, Some(store_worker_id))
        .await
        .expect("legacy worker still has the NULL-floor skill");
    assert!(block.contains("- **open** (v0.1.0): no floor\n"));
    assert!(
        !block.contains("**locked**"),
        "legacy worker (trust_grade=0) must NOT see Tenant-locked skill, got:\n{block}",
    );
}

#[tokio::test]
async fn markdown_format_stable() {
    // Pin the exact rendered bytes for a known input so prompt-format
    // drift surfaces loudly. Two skills, one with a non-empty
    // description, one with an empty description.
    let skills = fresh_skill_store().await;
    let s_full = install_skill(&skills, "named", "0.1.0", "has description").await;
    // Empty-description path: install with a single space then strip
    // to empty in the rendered output.
    let mut s_bare = install_skill(&skills, "bare", "2.0.0", " ").await;
    s_bare.frontmatter.description = String::new();

    let block = render_available_skills_block(&[s_full, s_bare]).unwrap();
    let expected = "## Available skills\n\
                    - **named** (v0.1.0): has description\n\
                    - **bare** (v2.0.0)\n";
    assert_eq!(block, expected);
}
