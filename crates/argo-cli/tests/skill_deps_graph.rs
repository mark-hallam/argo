//! End-to-end coverage for `argo skill deps`.
//!
//! Builds a small fixture skill set with known references, runs
//! the CLI, and asserts the markdown tree + JSON envelope. Cycle
//! detection is exercised by adding a self-referential skill.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

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

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn create(home: &Path, cwd: &Path, name: &str, body: &str) {
    let out = argo(
        home,
        cwd,
        &[
            "skill",
            "create",
            name,
            "--description",
            "fixture",
            "--body",
            body,
        ],
    );
    assert!(out.status.success(), "create {name}: {}", stderr(&out));
}

#[test]
fn deps_json_lists_calls_edge() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create(home.path(), work.path(), "leaf-a", "no refs.");
    create(
        home.path(),
        work.path(),
        "root-a",
        "step 1: argo skill run leaf-a",
    );

    let out = argo(
        home.path(),
        work.path(),
        &["skill", "deps", "root-a", "--json"],
    );
    assert!(out.status.success(), "deps json: {}", stderr(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["root"], "root-a");
    let edges = v["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["from"], "root-a");
    assert_eq!(edges[0]["to"], "leaf-a");
    assert_eq!(edges[0]["kind"], "calls");
    assert_eq!(v["cycles"].as_array().unwrap().len(), 0);
}

#[test]
fn deps_markdown_renders_tree_with_kind_label() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create(home.path(), work.path(), "child-1", "leaf");
    create(
        home.path(),
        work.path(),
        "parent-1",
        "see `skill:child-1` for details",
    );

    let out = argo(home.path(), work.path(), &["skill", "deps", "parent-1"]);
    assert!(out.status.success(), "deps md: {}", stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("# Dependencies of `parent-1`"), "header: {s}");
    assert!(
        s.contains("`child-1` [references]"),
        "edge with kind: {s}"
    );
}

#[test]
fn deps_handles_branching_chain() {
    // root → a → leaf-x
    //      └→ b
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create(home.path(), work.path(), "leaf-x", "leaf");
    create(home.path(), work.path(), "branch-a", "argo skill run leaf-x");
    create(home.path(), work.path(), "branch-b", "no refs");
    create(
        home.path(),
        work.path(),
        "branch-root",
        "argo skill run branch-a then argo skill run branch-b",
    );

    let out = argo(
        home.path(),
        work.path(),
        &["skill", "deps", "branch-root", "--json"],
    );
    assert!(out.status.success(), "branching: {}", stderr(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let edges = v["edges"].as_array().unwrap();
    let pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e["from"].as_str().unwrap().into(), e["to"].as_str().unwrap().into()))
        .collect();
    assert!(pairs.contains(&("branch-root".into(), "branch-a".into())));
    assert!(pairs.contains(&("branch-root".into(), "branch-b".into())));
    assert!(pairs.contains(&("branch-a".into(), "leaf-x".into())));
}

#[test]
fn deps_detects_cycles_without_infinite_loop() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create(home.path(), work.path(), "loop-a", "argo skill run loop-b");
    create(home.path(), work.path(), "loop-b", "argo skill run loop-a");

    let out = argo(
        home.path(),
        work.path(),
        &["skill", "deps", "loop-a", "--json"],
    );
    assert!(out.status.success(), "cycle: {}", stderr(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let cycles: Vec<&str> = v["cycles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        cycles.contains(&"loop-a"),
        "expected loop-a in cycles, got {cycles:?}"
    );
}

#[test]
fn deps_max_depth_is_honored() {
    // root → a → b → c. With max-depth=1 we should see only a.
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create(home.path(), work.path(), "depth-c", "leaf");
    create(home.path(), work.path(), "depth-b", "argo skill run depth-c");
    create(home.path(), work.path(), "depth-a", "argo skill run depth-b");
    create(
        home.path(),
        work.path(),
        "depth-root",
        "argo skill run depth-a",
    );

    let out = argo(
        home.path(),
        work.path(),
        &["skill", "deps", "depth-root", "--json", "--max-depth", "1"],
    );
    assert!(out.status.success(), "depth cap: {}", stderr(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let edges = v["edges"].as_array().unwrap();
    // depth=1 means we only walk root→ depth-a; depth-a's body
    // is parsed but its edges don't recurse further.
    let froms: Vec<&str> = edges.iter().map(|e| e["from"].as_str().unwrap()).collect();
    assert!(froms.contains(&"depth-root"));
    assert!(!froms.contains(&"depth-a"), "depth-1 must not recurse: {froms:?}");
}

#[test]
fn deps_reverse_finds_parents() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create(home.path(), work.path(), "rev-shared", "leaf shared by many");
    create(
        home.path(),
        work.path(),
        "rev-parent-1",
        "argo skill run rev-shared",
    );
    create(
        home.path(),
        work.path(),
        "rev-parent-2",
        "see `skill:rev-shared`",
    );

    let out = argo(
        home.path(),
        work.path(),
        &["skill", "deps", "rev-shared", "--reverse", "--json"],
    );
    assert!(out.status.success(), "reverse: {}", stderr(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let froms: Vec<&str> = v["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["from"].as_str().unwrap())
        .collect();
    assert!(froms.contains(&"rev-parent-1"), "p1: {froms:?}");
    assert!(froms.contains(&"rev-parent-2"), "p2: {froms:?}");
}

#[test]
fn deps_no_refs_renders_clean_message() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create(home.path(), work.path(), "loner", "no refs at all");

    let out = argo(home.path(), work.path(), &["skill", "deps", "loner"]);
    assert!(out.status.success(), "loner: {}", stderr(&out));
    assert!(stdout(&out).contains("(no forward dependencies)"));

    let out_json = argo(
        home.path(),
        work.path(),
        &["skill", "deps", "loner", "--json"],
    );
    let v: Value = serde_json::from_slice(&out_json.stdout).unwrap();
    assert_eq!(v["edges"].as_array().unwrap().len(), 0);
    assert_eq!(v["cycles"].as_array().unwrap().len(), 0);
}

#[test]
fn deps_unknown_skill_errors_clearly() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let out = argo(
        home.path(),
        work.path(),
        &["skill", "deps", "does-not-exist"],
    );
    assert!(!out.status.success(), "expected non-zero exit");
    let err = stderr(&out);
    assert!(
        err.contains("does-not-exist"),
        "error must name the missing skill: {err}"
    );
}
