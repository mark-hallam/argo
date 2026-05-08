mod common;

use argo_skill::{Provenance, SkillArtifact, SkillFrontmatter};
use argo_types::{CapabilitySet, Lock};
use common::argo_bin;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

const BODY: &str = "Legacy import body.\n";

#[test]
fn skill_import_accepts_legacy_three_file_artifact() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let artifact_path = work.path().join("legacy.skill.tar.gz");
    let artifact = artifact("legacy-skill", BODY);
    write_artifact(
        &artifact_path,
        &artifact,
        argo_skill::SKILL_ARTIFACT_BODY_LEGACY,
        BODY.as_bytes(),
    );

    let import = argo(
        home.path(),
        work.path(),
        &["skill", "import", artifact_path.to_str().unwrap(), "--json"],
    );
    assert!(import.status.success(), "{}", stderr(&import));
    let payload = json_stdout(&import);
    assert_eq!(payload["skill"]["name"], "legacy-skill");
    assert_eq!(payload["artifact"]["metadata"], Value::Null);
}

#[test]
fn skill_import_rejects_tampered_legacy_body() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let artifact_path = work.path().join("tampered.skill.tar.gz");
    let artifact = artifact("tampered-skill", BODY);
    write_artifact(
        &artifact_path,
        &artifact,
        argo_skill::SKILL_ARTIFACT_BODY_LEGACY,
        b"tampered body\n",
    );

    let import = argo(
        home.path(),
        work.path(),
        &["skill", "import", artifact_path.to_str().unwrap(), "--json"],
    );
    assert!(!import.status.success());
    assert!(stderr(&import).contains("body hash mismatch"));
}

fn artifact(name: &str, body: &str) -> SkillArtifact {
    SkillArtifact::new(
        SkillFrontmatter {
            name: name.into(),
            version: "1.0".into(),
            description: "legacy".into(),
            capabilities: CapabilitySet::empty(),
            provenance: Provenance {
                source: "user".into(),
                hash: "sha256-seed".into(),
            },
            lock: Lock::None,
            evals: Vec::new(),
        },
        body.into(),
    )
}

fn write_artifact(path: &Path, artifact: &SkillArtifact, body_path: &str, body: &[u8]) {
    let file = std::fs::File::create(path).unwrap();
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    append_bytes(
        &mut builder,
        argo_skill::SKILL_ARTIFACT_MANIFEST,
        &artifact.manifest_json().unwrap(),
    );
    append_bytes(&mut builder, body_path, body);
    let sha = format!("{}  {body_path}\n", artifact.manifest.body_sha256);
    append_bytes(
        &mut builder,
        argo_skill::SKILL_ARTIFACT_SHA256,
        sha.as_bytes(),
    );
    let encoder = builder.into_inner().unwrap();
    encoder.finish().unwrap();
}

fn append_bytes<W: std::io::Write>(builder: &mut tar::Builder<W>, path: &str, bytes: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_cksum();
    builder.append_data(&mut header, path, bytes).unwrap();
}

fn argo(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
