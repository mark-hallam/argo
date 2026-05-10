//! End-to-end CLI smoke for `argo skill {keygen,sign,verify-sig}`.
//!
//! Walks the cosign-style signing lifecycle through the real `argo`
//! binary: generate keys → publish a SKILL.md to a mock OCI registry
//! → sign it → verify OK → tamper with the published bundle → verify
//! FAIL → also verify MISSING for an unsigned tag.

mod common;

use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

const SKILL_MD: &str = "---\nname: smoke\nversion: \"0.1.0\"\ndescription: \"sign smoke\"\nprovenance:\n  source: test\n  hash: sha256-test\n---\n# Smoke\nOriginal body.\n";

#[test]
fn keygen_sign_verify_and_tamper_roundtrip() {
    let home = tempfile::tempdir().unwrap();
    let keys = tempfile::tempdir().unwrap();
    let registry_dir = tempfile::tempdir().unwrap();
    let manifest = home.path().join("skill.md");
    std::fs::write(&manifest, SKILL_MD).unwrap();
    let registry = format!("mock://{}", registry_dir.path().display());

    // keygen → produces cosign.key + cosign.pub
    let keygen_json = run_json(
        home.path(),
        &[
            "skill",
            "keygen",
            "--out",
            keys.path().to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(keygen_json["ok"], true);
    let private = keys.path().join("cosign.key");
    let public = keys.path().join("cosign.pub");
    assert!(private.exists());
    assert!(public.exists());

    // publish → mock registry has a bundle.tar at oci/smoke/0.1.0/
    let publish_json = run_json(
        home.path(),
        &[
            "skill",
            "publish",
            "smoke",
            "--manifest",
            manifest.to_str().unwrap(),
            "--registry",
            &registry,
            "--tag",
            "0.1.0",
            "--json",
        ],
    );
    assert_eq!(publish_json["ok"], true);
    let bundle = registry_dir
        .path()
        .join("oci")
        .join("smoke")
        .join("0.1.0")
        .join("bundle.tar");
    assert!(bundle.exists());

    // verify-sig before sign → MISSING
    let missing = run_json(
        home.path(),
        &[
            "skill",
            "verify-sig",
            "smoke:0.1.0",
            "--key",
            public.to_str().unwrap(),
            "--registry",
            &registry,
            "--json",
        ],
    );
    assert_eq!(missing["outcome"], "MISSING");
    assert_eq!(missing["ok"], false);

    // sign --manifest
    let sign_json = run_json(
        home.path(),
        &[
            "skill",
            "sign",
            "--manifest",
            manifest.to_str().unwrap(),
            "--key",
            private.to_str().unwrap(),
            "--registry",
            &registry,
            "--tag",
            "0.1.0",
            "--json",
        ],
    );
    assert_eq!(sign_json["ok"], true);
    assert_eq!(sign_json["name"], "smoke");
    let sig_path = registry_dir
        .path()
        .join("oci")
        .join("smoke")
        .join("0.1.0")
        .join("signature.json");
    assert!(sig_path.exists());

    // verify-sig → OK
    let ok = run_json(
        home.path(),
        &[
            "skill",
            "verify-sig",
            "smoke:0.1.0",
            "--key",
            public.to_str().unwrap(),
            "--registry",
            &registry,
            "--json",
        ],
    );
    assert_eq!(ok["outcome"], "OK", "ok json: {ok}");
    assert_eq!(ok["ok"], true);

    // tamper: rewrite the published bundle with mutated SKILL.md
    // (replacement keeps the byte length so the tar header stays valid).
    let tampered_md = SKILL_MD.replace("Original body.", "Tampered body!");
    rewrite_skill_md_in_bundle(&bundle, &tampered_md);

    let failed = run_json(
        home.path(),
        &[
            "skill",
            "verify-sig",
            "smoke:0.1.0",
            "--key",
            public.to_str().unwrap(),
            "--registry",
            &registry,
            "--json",
        ],
    );
    assert_eq!(failed["outcome"], "FAIL", "fail json: {failed}");
    assert_eq!(failed["ok"], false);
    assert!(failed["reason"]
        .as_str()
        .unwrap_or_default()
        .contains("digest"));
}

fn rewrite_skill_md_in_bundle(bundle: &Path, new_md: &str) {
    let mut bytes = std::fs::read(bundle).unwrap();
    let mut cursor = 0usize;
    while cursor + 512 <= bytes.len() {
        let header = &bytes[cursor..cursor + 512];
        if header.iter().all(|b| *b == 0) {
            panic!("SKILL.md not found in bundle");
        }
        let name_end = header[..100].iter().position(|b| *b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&header[..name_end]).unwrap();
        let size_bytes = &header[124..136];
        let size_str = std::str::from_utf8(size_bytes)
            .unwrap()
            .trim_end_matches('\0');
        let size = u64::from_str_radix(size_str.trim(), 8).unwrap() as usize;
        let body_start = cursor + 512;
        let body_end = body_start + size;
        if name == "SKILL.md" {
            assert_eq!(
                new_md.len(),
                size,
                "tampered SKILL.md must keep same byte length (replace only same-width text)"
            );
            bytes[body_start..body_end].copy_from_slice(new_md.as_bytes());
            std::fs::write(bundle, &bytes).unwrap();
            return;
        }
        cursor = body_end + ((512 - size % 512) % 512);
    }
    panic!("SKILL.md not found in bundle");
}

fn run_json(home: &Path, args: &[&str]) -> Value {
    let out = run_ok(home, args);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "non-json stdout from argo {args:?}: {e}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn run_ok(home: &Path, args: &[&str]) -> Output {
    let out = run(home, args);
    assert!(
        out.status.success(),
        "argo {args:?} failed\nstderr: {}\nstdout: {}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    out
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap()
}
