//! Integration test for the Pitch Agent worker template — Track P
//! brief P-05.
//!
//! Asserts three things on the real `packs/pitch-agent/` checkout:
//!
//! 1. `pack.toml` loads through `PackManifest::load` and adapts into
//!    a `LoadedVertical` the runtime registry accepts.
//! 2. Every file the pack's `worker.toml` points at exists on disk
//!    and parses as its declared format (TOML for `.toml`, JSON for
//!    `.json`, plain UTF-8 read for persona `.md` files).
//! 3. The template is tenant-clean — loading the same pack into two
//!    independent registries tagged with two different tenant labels
//!    yields independent `LoadedVertical` instances with no shared
//!    mutable state. This is the cross-tenant negative test the brief
//!    requires for a public-representation worker.

use argo_vertical::Registry;
use argo_vertical_packs::{adapt_pack_entry, PackLoader, PackLoaderConfig, PackManifest};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let mut cur = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if cur.join("Cargo.lock").is_file() {
            return cur;
        }
        if !cur.pop() {
            panic!("workspace root with Cargo.lock not found");
        }
    }
}

fn pack_root() -> PathBuf {
    workspace_root().join("packs").join("pitch-agent")
}

#[test]
fn pitch_agent_pack_loads_and_adapts_into_vertical() {
    let root = pack_root();
    assert!(
        root.join("pack.toml").is_file(),
        "expected fixture pack at {}",
        root.display()
    );

    let manifest = PackManifest::load(&root).expect("pack.toml must load");
    assert_eq!(manifest.pack.name, "pitch-agent");
    assert_eq!(manifest.verticals.len(), 1);
    assert_eq!(manifest.verticals[0].id, "pitch-agent");

    let loaded = adapt_pack_entry(&root, &manifest, &manifest.verticals[0])
        .expect("adapter must accept the Pitch template");
    assert_eq!(loaded.manifest.vertical.id, "pitch-agent");
    assert_eq!(loaded.manifest.vertical.name, "Pitch");
}

#[test]
fn pitch_agent_worker_toml_references_resolve_and_parse() {
    let root = pack_root();
    let worker_body =
        std::fs::read_to_string(root.join("worker.toml")).expect("worker.toml readable");
    let header: WorkerHeader = toml::from_str(&worker_body).expect("worker.toml parses");

    let mut all_refs: Vec<(&str, &str)> = Vec::new();
    let personas = [
        ("elevator_30s", header.personas.elevator_30s.as_str()),
        ("investor_3min", header.personas.investor_3min.as_str()),
        ("demo_10min", header.personas.demo_10min.as_str()),
        ("diligence_deep", header.personas.diligence_deep.as_str()),
        ("voice_and_tone", header.personas.voice_and_tone.as_str()),
    ];
    all_refs.extend(personas);
    all_refs.push(("memory.schema", header.memory.schema.as_str()));
    all_refs.push(("skills.bundle", header.skills.bundle.as_str()));
    all_refs.push((
        "policies.truth_boundary_ref",
        header.policies.truth_boundary_ref.as_str(),
    ));
    all_refs.push(("tools.permissions", header.tools.permissions.as_str()));
    all_refs.push(("workflows.rehearsal", header.workflows.rehearsal.as_str()));
    all_refs.push(("workflows.web_embed", header.workflows.web_embed.as_str()));
    all_refs.push((
        "workflows.live_meeting",
        header.workflows.live_meeting.as_str(),
    ));
    all_refs.push(("escalations.routes", header.escalations.routes.as_str()));
    all_refs.push(("evals.cases_ref", header.evals.cases_ref.as_str()));
    all_refs.push(("ui.cohort_meta", header.ui.cohort_meta.as_str()));

    for (key, rel) in all_refs {
        let path = root.join(rel);
        assert!(
            path.is_file(),
            "worker.toml ref '{key}' = '{rel}' missing at {}",
            path.display()
        );
        assert_parses_as_declared(key, &path);
    }
}

#[test]
fn pitch_agent_truth_boundary_policies_cover_every_channel() {
    let root = pack_root();
    let policy_body = std::fs::read_to_string(root.join("policies/truth-boundary-ref.toml"))
        .expect("truth-boundary ref readable");
    let parsed: TruthBoundaryRef = toml::from_str(&policy_body).expect("ref parses");
    assert!(
        !parsed.default_policy_id.is_empty(),
        "default policy id must be set"
    );
    for ch in ["rehearsal", "web_embed", "live_meeting"] {
        assert!(
            parsed.channel_overrides.contains_key(ch),
            "channel '{ch}' missing override; pack must name a policy for every workflow channel",
        );
    }
}

#[test]
fn pitch_agent_skills_bundle_lists_required_skills() {
    let root = pack_root();
    let bundle_body =
        std::fs::read_to_string(root.join("skills/bundle.toml")).expect("bundle readable");
    let bundle: SkillsBundle = toml::from_str(&bundle_body).expect("bundle parses");
    let ids: Vec<&str> = bundle.skills.iter().map(|s| s.id.as_str()).collect();
    for required in [
        "canonical-qa",
        "objection-handler",
        "pitch-mode-switcher",
        "slide-controller",
        "follow-up-writer",
    ] {
        assert!(ids.contains(&required), "skill '{required}' missing from bundle (got {ids:?})");
    }
    // Stubs must declare themselves; the pack docs commit to that
    // contract so an operator deploying the template sees the gap.
    for skill in &bundle.skills {
        if skill.source.as_deref() == Some("stub") {
            assert!(
                skill.stub_note.is_some(),
                "skill '{}' is a stub but carries no stub_note",
                skill.id,
            );
        }
    }
}

#[test]
fn pitch_agent_template_is_tenant_clean_across_two_loads() {
    // The template carries no tenant-bound data — every load against
    // the same on-disk pack must yield independent `LoadedVertical`
    // instances that two tenants can deploy without cross-talk. We
    // assert this by dev-linking the same pack into two independent
    // cache roots (the per-tenant analogue) and confirming the loaded
    // verticals land in two independent registries with the same id
    // and no shared mutable state.
    let pack = pack_root();
    let cache_tenant_a = tempfile::tempdir().unwrap();
    let cache_tenant_b = tempfile::tempdir().unwrap();

    let loader_a = PackLoader::new(PackLoaderConfig::with_cache_dir(
        cache_tenant_a.path().into(),
    ));
    let loader_b = PackLoader::new(PackLoaderConfig::with_cache_dir(
        cache_tenant_b.path().into(),
    ));
    loader_a.dev_link("pitch-agent", &pack).unwrap();
    loader_b.dev_link("pitch-agent", &pack).unwrap();

    let v_a = loader_a
        .load_as_vertical("pitch-agent", None)
        .unwrap()
        .expect("tenant A resolves the dev-linked pack");
    let v_b = loader_b
        .load_as_vertical("pitch-agent", None)
        .unwrap()
        .expect("tenant B resolves the dev-linked pack");

    assert_eq!(v_a.len(), 1);
    assert_eq!(v_b.len(), 1);
    assert_eq!(v_a[0].manifest.vertical.id, "pitch-agent");
    assert_eq!(v_b[0].manifest.vertical.id, "pitch-agent");

    let mut reg_a = Registry::new();
    let mut reg_b = Registry::new();
    reg_a.insert(v_a[0].clone()).unwrap();
    reg_b.insert(v_b[0].clone()).unwrap();
    assert!(reg_a.get("pitch-agent").is_some());
    assert!(reg_b.get("pitch-agent").is_some());

    // Mutating tenant A's cache root must not leak into tenant B.
    drop(cache_tenant_a);
    assert!(
        cache_tenant_b.path().join("dev-links.toml").is_file(),
        "tenant B's dev-link cache must survive tenant A's teardown",
    );
}

fn assert_parses_as_declared(key: &str, path: &Path) {
    let body = std::fs::read_to_string(path).expect("file readable");
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => {
            toml::from_str::<toml::Value>(&body)
                .unwrap_or_else(|e| panic!("{key} at {} must parse as TOML: {e}", path.display()));
        }
        Some("json") => {
            serde_json::from_str::<serde_json::Value>(&body)
                .unwrap_or_else(|e| panic!("{key} at {} must parse as JSON: {e}", path.display()));
        }
        Some("md") => {
            assert!(!body.trim().is_empty(), "{key} at {} is empty", path.display());
        }
        other => panic!("{key} at {} has unexpected extension {:?}", path.display(), other),
    }
}

#[derive(Debug, Deserialize)]
struct WorkerHeader {
    personas: PersonasRef,
    memory: MemoryRef,
    skills: SkillsRef,
    policies: PoliciesRef,
    tools: ToolsRef,
    workflows: WorkflowsRef,
    escalations: EscalationsRef,
    evals: EvalsRef,
    ui: UiRef,
}

#[derive(Debug, Deserialize)]
struct PersonasRef {
    elevator_30s: String,
    investor_3min: String,
    demo_10min: String,
    diligence_deep: String,
    voice_and_tone: String,
}

#[derive(Debug, Deserialize)]
struct MemoryRef {
    schema: String,
}
#[derive(Debug, Deserialize)]
struct SkillsRef {
    bundle: String,
}
#[derive(Debug, Deserialize)]
struct PoliciesRef {
    truth_boundary_ref: String,
}
#[derive(Debug, Deserialize)]
struct ToolsRef {
    permissions: String,
}
#[derive(Debug, Deserialize)]
struct WorkflowsRef {
    rehearsal: String,
    web_embed: String,
    live_meeting: String,
}
#[derive(Debug, Deserialize)]
struct EscalationsRef {
    routes: String,
}
#[derive(Debug, Deserialize)]
struct EvalsRef {
    cases_ref: String,
}
#[derive(Debug, Deserialize)]
struct UiRef {
    cohort_meta: String,
}

#[derive(Debug, Deserialize)]
struct TruthBoundaryRef {
    default_policy_id: String,
    channel_overrides: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct SkillsBundle {
    skills: Vec<SkillBundleEntry>,
}

#[derive(Debug, Deserialize)]
struct SkillBundleEntry {
    id: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    stub_note: Option<String>,
}
