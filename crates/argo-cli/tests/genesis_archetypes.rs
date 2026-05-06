mod common;

use common::argo_bin;
use std::process::Command;

#[test]
fn genesis_archetype_discovery_commands_work() {
    let list = Command::new(argo_bin())
        .args(["genesis", "list-archetypes", "--json"])
        .output()
        .expect("spawn list");
    assert!(list.status.success(), "stderr: {}", String::from_utf8_lossy(&list.stderr));
    let body = String::from_utf8(list.stdout).unwrap();
    let listed: serde_json::Value = serde_json::from_str(&body).unwrap();
    let archetypes = listed["archetypes"].as_array().unwrap();
    assert_eq!(archetypes.len(), 12);
    assert_eq!(archetypes[0]["name"], "Bookkeeping Assistant");

    let show = Command::new(argo_bin())
        .args(["genesis", "show-archetype", "marketing-intern", "--json"])
        .output()
        .expect("spawn show");
    assert!(show.status.success(), "stderr: {}", String::from_utf8_lossy(&show.stderr));
    let shown: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(shown["name"], "Marketing Intern");
    assert!(shown["seed_actions"].as_array().unwrap().iter().any(|a| a["name"] == "post_campaign"));

    let suggest = Command::new(argo_bin())
        .args([
            "genesis",
            "suggest",
            "--charter",
            "be my marketing intern and run weekly campaigns to cold leads",
            "--json",
        ])
        .output()
        .expect("spawn suggest");
    assert!(suggest.status.success(), "stderr: {}", String::from_utf8_lossy(&suggest.stderr));
    let suggested: serde_json::Value = serde_json::from_slice(&suggest.stdout).unwrap();
    let matches = suggested["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0]["name"], "Marketing Intern");
    for item in matches {
        let confidence = item["confidence"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&confidence));
    }
}

#[test]
fn genesis_show_unknown_archetype_fails_loudly() {
    let out = Command::new(argo_bin())
        .args(["genesis", "show-archetype", "not-real"])
        .output()
        .expect("spawn show missing");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("not-real"), "got: {stderr}");
    assert!(stderr.contains("not found"), "got: {stderr}");
}
