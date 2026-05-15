//! End-to-end Claim-4 loop smoke for learned skills:
//! teach -> propose -> eval -> review -> publish -> reload -> visible.

use argo_skill::{
    list_mock_skill_tags, publish_mock_skill, ActualOutcome, EvalCase, EvalExecutor, LayeredLookup,
    LifecycleState, Skill, SkillCandidate, SkillLayer, SkillPublishSource, SkillStore,
};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const TRAJECTORY_JSON: &str = include_str!("fixtures/claim4_e2e_trajectory.json");
const MARKETPLACE_SKILL_MD: &str = include_str!("fixtures/claim4_e2e_skill.md");
const MARKETPLACE_BODY_SNIPPET: &str = "return the listing URL plus a short confirmation";
const PRIVATE_BODY_SNIPPET: &str = "tenant-private checkout workaround";

#[derive(Debug, Deserialize)]
struct TrajectoryFixture {
    name: String,
    action_type: String,
    vertical_id: String,
    trajectory: Vec<String>,
    final_text: Option<String>,
    tokens_used: u32,
    success_metric: f64,
}

struct DeterministicEval {
    outputs: Mutex<Vec<Value>>,
}

impl DeterministicEval {
    fn new(outputs: Vec<Value>) -> Self {
        Self {
            outputs: Mutex::new(outputs),
        }
    }
}

#[async_trait]
impl EvalExecutor for DeterministicEval {
    async fn run_one(&self, _skill: &Skill, _case: &EvalCase) -> ArgoResult<ActualOutcome> {
        let mut outputs = self.outputs.lock().unwrap_or_else(|p| p.into_inner());
        assert!(!outputs.is_empty(), "deterministic eval exhausted");
        Ok(ActualOutcome::new(outputs.remove(0)))
    }
}

#[tokio::test]
async fn claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak() {
    let fixture: TrajectoryFixture =
        serde_json::from_str(TRAJECTORY_JSON).expect("trajectory fixture parses");
    let workspace = skill_store().await;
    let (tenant, workspace_id) = workspace
        .store
        .ensure_workspace("claim4-tenant-a", "/tmp/claim4-tenant-a")
        .await
        .expect("workspace");
    let session_id = workspace
        .store
        .create_session(
            tenant,
            workspace_id,
            "mock-model",
            "mock-provider",
            Some("claim4 learning session"),
            Some(&fixture.vertical_id),
        )
        .await
        .expect("session");

    let candidate = SkillCandidate::from_cu_trajectory(
        fixture.name.clone(),
        &fixture.action_type,
        &fixture.vertical_id,
        fixture.trajectory.clone(),
        fixture.final_text.clone(),
        fixture.tokens_used,
        fixture.success_metric,
    );
    let raw_candidate = with_eval_case(&candidate.to_skill_md("claim4-session"));
    assert!(raw_candidate.contains("source: learned:claim4-session"));
    assert!(raw_candidate.contains("kind: contains"));

    let eval = DeterministicEval::new(vec![Value::String(
        "listing claim4-demo verified at USD 42".into(),
    )]);
    let proposed = workspace
        .skills
        .propose_for_review_judged_with_evals(
            &raw_candidate,
            session_id,
            &fixture.vertical_id,
            None,
            Some(&eval),
        )
        .await
        .expect("propose with eval gate");
    assert_eq!(proposed.lifecycle_state, LifecycleState::PendingReview);
    assert_eq!(proposed.frontmatter.evals.len(), 1);

    assert!(
        workspace
            .skills
            .list_skills_for_vertical(&fixture.vertical_id)
            .await
            .expect("visible before review")
            .is_empty(),
        "pending_review proposals must not enter worker prompt"
    );
    let pending = workspace
        .skills
        .list_pending_review()
        .await
        .expect("pending review list");
    assert_eq!(pending.len(), 1);

    let next = workspace
        .skills
        .transition_lifecycle(proposed.id, LifecycleState::Active)
        .await
        .expect("human approval");
    assert_eq!(next, LifecycleState::Active);
    let active = workspace
        .skills
        .find_active_by_name(&fixture.name)
        .await
        .expect("find active")
        .expect("active skill exists");
    assert_eq!(active.frontmatter.name, fixture.name);

    let registry_root = std::env::temp_dir().join(format!(
        "claim4-registry-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    ));
    let publish_source = SkillPublishSource::from_skill(&active).expect("publish source");
    let record = publish_mock_skill(&registry_root, &publish_source, "0.1.0", false)
        .expect("publish mock skill");
    assert!(!record.dry_run);
    assert!(registry_root.join(&record.manifest_path).exists());
    assert!(registry_root.join(&record.blob_path).exists());
    let tags = list_mock_skill_tags(&registry_root, &fixture.name, 10).expect("list tags");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].oci_ref, record.oci_ref);

    let empty_worker_workspace = skill_store().await;
    let marketplace_library = skill_store().await;
    marketplace_library
        .skills
        .install(MARKETPLACE_SKILL_MD, None)
        .await
        .expect("install marketplace reload");
    let prompt = render_worker_prompt(&empty_worker_workspace.skills, &marketplace_library.skills)
        .await
        .expect("render prompt");
    assert!(prompt.contains("claim4_price_check"));
    assert!(prompt.contains(MARKETPLACE_BODY_SNIPPET));
    assert!(prompt.contains("library:claim4_price_check"));

    let other_tenant_workspace = skill_store().await;
    let other_tenant_library = skill_store().await;
    other_tenant_library
        .skills
        .install(private_skill_md(), None)
        .await
        .expect("install tenant-private library row");
    let other_prompt =
        render_worker_prompt(&other_tenant_workspace.skills, &other_tenant_library.skills)
            .await
            .expect("render other tenant prompt");
    assert!(!other_prompt.contains("tenant_private_price_check"));
    assert!(!other_prompt.contains(PRIVATE_BODY_SNIPPET));

    std::fs::remove_dir_all(&registry_root).ok();
}

struct TestStore {
    store: Arc<SessionStore>,
    skills: SkillStore,
}

async fn skill_store() -> TestStore {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate store");
    let store = Arc::new(store);
    let skills = SkillStore::new(Arc::clone(&store));
    TestStore { store, skills }
}

fn with_eval_case(raw: &str) -> String {
    let raw = raw.replacen("capabilities:\n  - cu\n", "capabilities: []\n", 1);
    raw.replacen(
        "lock: none\n---",
        "lock: none\nevals:\n  - input:\n      listing: claim4-demo\n      expected_price: USD 42\n    expected:\n      kind: contains\n      text: USD 42\n---",
        1,
    )
}

async fn render_worker_prompt(workspace: &SkillStore, library: &SkillStore) -> ArgoResult<String> {
    let lookup = LayeredLookup::with_library(workspace, library);
    let mut prompt = String::new();
    for layered in lookup.list_for_action("low").await? {
        if layered.layer == SkillLayer::Library
            && !layered
                .skill
                .frontmatter
                .provenance
                .source
                .starts_with("published:")
        {
            continue;
        }
        let layer = match layered.layer {
            SkillLayer::Workspace => "workspace",
            SkillLayer::Library => "library",
        };
        prompt.push_str(&format!(
            "\n## {layer}:{}\n{}\n",
            layered.skill.frontmatter.name, layered.skill.body
        ));
    }
    Ok(prompt)
}

fn private_skill_md() -> &'static str {
    "---\n\
     name: tenant_private_price_check\n\
     version: 0.1.0\n\
     description: tenant-private skill that must not cross tenant boundaries\n\
     provenance:\n  \
       source: user\n  \
       hash: sha256-private\n\
     lock: none\n\
     ---\n\n\
     # tenant_private_price_check\n\n\
     tenant-private checkout workaround\n"
}
