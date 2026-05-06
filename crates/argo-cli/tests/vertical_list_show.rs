mod common;
mod assert_cmd {
    pub use std::process::Command;
}

use assert_cmd::Command;
use common::argo_bin;
use std::fs;
use std::path::Path;

const FIXTURE_TOML: &str = r#"schema_version = "1"

[vertical]
id = "demo-shop"
name = "Demo Shop"
description = "Helps operators prepare marketplace listings safely."
version = "0.2.0"
tags = ["demo"]

[role]
system_prompt = "prompts/system.md"
persona = "prompts/persona.md"

[skills]
bundle = "skills/"

[evaluation]
suites = ["eval/smoke.jsonl"]

[actions.price_check]
risk = "low"
requires_approval = false
description = "Check a proposed listing price."
executor = "human"

[actions.publish_listing]
risk = "high"
requires_approval = true
description = "Publish a listing after operator approval."
executor = "human"
"#;

fn write_fixture(verticals: &Path) {
    let root = verticals.join("demo-shop");
    fs::create_dir_all(root.join("prompts")).unwrap();
    fs::create_dir_all(root.join("skills")).unwrap();
    fs::create_dir_all(root.join("eval")).unwrap();
    fs::create_dir_all(verticals.join("scratch")).unwrap();
    fs::write(root.join("vertical.toml"), FIXTURE_TOML).unwrap();
    fs::write(
        root.join("prompts/system.md"),
        "You are a demo shop worker.",
    )
    .unwrap();
    fs::write(
        root.join("prompts/persona.md"),
        "Warm, concise marketplace operations specialist.",
    )
    .unwrap();
    fs::write(root.join("skills/listings.md"), "# Listings\n").unwrap();
    fs::write(
        root.join("eval/smoke.jsonl"),
        "{\"id\":\"one\",\"input\":\"draft\"}\n{\"id\":\"two\",\"input\":\"price\"}\n",
    )
    .unwrap();
}

fn argo(verticals: &Path) -> Command {
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_VERTICALS_DIR", verticals);
    cmd
}

#[test]
fn vertical_list_show_describe_cover_installed_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_fixture(&verticals);

    let list = argo(&verticals)
        .args(["vertical", "list", "--source", "workspace", "--json"])
        .output()
        .expect("spawn list");
    assert!(
        list.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    let listed: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(listed["ok"], true);
    assert_eq!(listed["loaded"].as_array().unwrap().len(), 1);
    assert_eq!(listed["loaded"][0]["id"], "demo-shop");
    assert_eq!(listed["loaded"][0]["action_count"], 2);
    assert_eq!(listed["loaded"][0]["source"], "workspace");

    let library = argo(&verticals)
        .args(["vertical", "list", "--source", "library"])
        .output()
        .expect("spawn library");
    assert!(library.status.success());
    let library_text = String::from_utf8(library.stdout).unwrap();
    assert!(
        library_text.contains("no library configured"),
        "got: {library_text}"
    );

    let show = argo(&verticals)
        .args(["vertical", "show", "demo-shop", "--json"])
        .output()
        .expect("spawn show");
    assert!(
        show.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&show.stderr)
    );
    let shown: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(shown["id"], "demo-shop");
    assert_eq!(shown["version"], "0.2.0");
    assert_eq!(shown["action_count"], 2);
    assert_eq!(shown["eval_seed_count"], 2);
    assert!(shown["vertical_toml"]
        .as_str()
        .unwrap()
        .contains("[actions.publish_listing]"));
    let actions = shown["actions"].as_array().unwrap();
    let publish = actions
        .iter()
        .find(|a| a["name"] == "publish_listing")
        .unwrap();
    assert_eq!(publish["risk"], "high");
    assert_eq!(publish["requires_approval"], true);
    assert_eq!(
        shown["skills_bundled"].as_array().unwrap()[0],
        "listings.md"
    );

    let describe = argo(&verticals)
        .args(["vertical", "describe", "demo-shop"])
        .output()
        .expect("spawn describe");
    assert!(
        describe.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&describe.stderr)
    );
    let description = String::from_utf8(describe.stdout).unwrap();
    let len = description.trim().chars().count();
    assert!((100..=300).contains(&len), "len={len}: {description}");
    assert!(description.contains("Demo Shop"), "got: {description}");
    assert!(description.contains("approval-gated"), "got: {description}");
}
