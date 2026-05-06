mod common;
mod assert_cmd {
    pub use std::process::Command;
}

use argo_skill::{PrincipalOverride, Skill, SkillStore};
use argo_store::SessionStore;
use argo_types::Lock;
use assert_cmd::Command;
use common::argo_bin;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;

const BODY: &str = "# Offline Skill\n\nPreserve this body exactly.\n";

fn argo(home: &Path, workspace: &Path) -> Command {
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home).current_dir(workspace);
    cmd
}

async fn seed_locked_skill(home: &Path) -> String {
    let store = open_store(home).await;
    let skills = SkillStore::new(store);
    let raw = format!(
        "---\nname: offline-skill\nversion: \"1.2.3\"\ndescription: \"Offline skill\"\n\
         lock: none\nprovenance:\n  source: seed\n  hash: {}\n---\n{}",
        body_hash(BODY),
        BODY
    );
    let skill = skills.install(&raw, None).await.unwrap();
    skills
        .set_lock(skill.id, Lock::ContentLocked, PrincipalOverride::Force)
        .await
        .unwrap();
    skill.id.0.to_string()
}

async fn active_skills(home: &Path) -> Vec<Skill> {
    let store = open_store(home).await;
    SkillStore::new(store).list_active().await.unwrap()
}

async fn open_store(home: &Path) -> Arc<SessionStore> {
    let store = SessionStore::open(&home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

fn body_hash(body: &str) -> String {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    format!("sha256-{}", hex::encode(h.finalize()))
}

#[test]
fn skill_review_export_import_round_trips_body_frontmatter_and_lock() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let source_home = tempfile::tempdir().unwrap();
    let import_home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let skill_id = rt.block_on(seed_locked_skill(source_home.path()));

    let md_path = workspace.path().join("offline-skill.md");
    let md_out = argo(source_home.path(), workspace.path())
        .args([
            "skill",
            "review",
            "export",
            &skill_id,
            "--output",
            md_path.to_str().unwrap(),
        ])
        .output()
        .expect("export md");
    assert!(
        md_out.status.success(),
        "{}",
        String::from_utf8_lossy(&md_out.stderr)
    );
    let md = std::fs::read_to_string(&md_path).unwrap();
    assert!(md.contains("name: \"offline-skill\""));
    assert!(md.contains("lock: content-locked"));
    assert!(md.ends_with(BODY));

    let json_path = workspace.path().join("offline-skill.json");
    let json_out = argo(source_home.path(), workspace.path())
        .args([
            "skill",
            "review",
            "export",
            &skill_id,
            "--output",
            json_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("export json");
    assert!(json_out.status.success());
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(json["frontmatter"]["name"], "offline-skill");
    assert_eq!(json["body"], BODY);

    let toml_path = workspace.path().join("offline-skill.toml");
    let toml_out = argo(source_home.path(), workspace.path())
        .args([
            "skill",
            "review",
            "export",
            &skill_id,
            "--output",
            toml_path.to_str().unwrap(),
            "--format",
            "toml",
        ])
        .output()
        .expect("export toml");
    assert!(toml_out.status.success());
    let toml: toml::Value = toml::from_str(&std::fs::read_to_string(&toml_path).unwrap()).unwrap();
    assert_eq!(
        toml["frontmatter"]["name"].as_str().unwrap(),
        "offline-skill"
    );
    assert_eq!(toml["body"].as_str().unwrap(), BODY);

    let import = argo(import_home.path(), workspace.path())
        .args([
            "skill",
            "review",
            "import",
            md_path.to_str().unwrap(),
            "--lock",
            "content_locked",
        ])
        .output()
        .expect("import md");
    assert!(
        import.status.success(),
        "{}",
        String::from_utf8_lossy(&import.stderr)
    );
    let imported = rt.block_on(active_skills(import_home.path()));
    assert_eq!(imported.len(), 1);
    let skill = &imported[0];
    assert_eq!(skill.frontmatter.name, "offline-skill");
    assert_eq!(skill.frontmatter.version, "1.2.3");
    assert_eq!(skill.frontmatter.description, "Offline skill");
    assert_eq!(skill.frontmatter.lock, Lock::ContentLocked);
    assert_eq!(skill.frontmatter.provenance.hash, body_hash(BODY));
    assert!(skill.frontmatter.provenance.source.starts_with("imported:"));
    assert_eq!(skill.body, BODY);

    let reexport_path = workspace.path().join("offline-skill-reexport.md");
    let reexport = argo(import_home.path(), workspace.path())
        .args([
            "skill",
            "review",
            "export",
            &skill.id.0.to_string(),
            "--output",
            reexport_path.to_str().unwrap(),
        ])
        .output()
        .expect("re-export md");
    assert!(reexport.status.success());
    let reexported = std::fs::read_to_string(&reexport_path).unwrap();
    assert!(reexported.contains("lock: content-locked"));
    assert!(reexported.ends_with(BODY));

    let duplicate = argo(import_home.path(), workspace.path())
        .args(["skill", "review", "import", md_path.to_str().unwrap()])
        .output()
        .expect("duplicate import");
    assert!(!duplicate.status.success());
    let stderr = String::from_utf8_lossy(&duplicate.stderr);
    assert!(stderr.contains("import hash collision"), "got: {stderr}");
}
