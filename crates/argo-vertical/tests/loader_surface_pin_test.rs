use argo_vertical::{
    build_judge_prompt, check_isolation, judge_fixture, load_dir, parse, parse_actuals_jsonl,
    parse_judge_response, parse_judge_spec, parse_memory_schema, parse_workflow, score_fixture,
    score_suite_jsonl, score_suite_jsonl_with_actuals, validate, validate_memory_schema,
    validate_strict, validate_workflow, ActionDeclaration, ActionRetryPolicy, ActionRiskLevel,
    ActionsSection, ActualsMap, EscalationSection, EvalJudgeVerdict, EvaluationSection,
    ExecutorDeclaration, ExecutorTagged, FixtureStatus, GatewaySection, KpiComparator,
    KpiDefinition, KpiGate, KpiKind, LoadedVertical, McpServerDeclaration, McpServersSection,
    MemoryEntry, MemorySchema, MemorySection, ProviderSection, RoleSection, SkillsSection,
    SuccessMetricsSection, ToolsSection, TuiSection, VerticalIdentity, VerticalManifest, Workflow,
    WorkflowHeader, WorkflowStep, WorkflowsSection, ENV_VERTICALS_DIR, KNOWN_EXECUTOR_KINDS,
    MEMORY_SCHEMA_VERSION, SCHEMA_VERSION, WORKFLOW_SCHEMA_VERSION,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const WORKFLOW_TOML: &str = r#"
schema_version = "1"

[workflow]
name = "triage"
description = "Triage one delegated request."

[[step]]
name = "classify"
kind = "prompt"
prompt = "Classify the request."

[[step]]
name = "draft"
kind = "tool_call"
tool = "draft_response"
args = { priority = "normal" }
depends_on = ["classify"]
"#;

fn write_vertical(root: &Path, dir_name: &str, id: &str, with_workflow: bool) -> PathBuf {
    let dir = root.join(dir_name);
    fs::create_dir_all(&dir).expect("create vertical dir");
    let workflow_section = if with_workflow {
        r#"
[workflows]
files = ["workflows/triage.workflow.toml"]
"#
    } else {
        ""
    };
    fs::write(
        dir.join("vertical.toml"),
        format!(
            r#"schema_version = "1"

[vertical]
id = "{id}"
name = "{id} worker"
version = "0.1.0"
{workflow_section}"#
        ),
    )
    .expect("write manifest");
    if with_workflow {
        let workflow_dir = dir.join("workflows");
        fs::create_dir_all(&workflow_dir).expect("create workflows dir");
        fs::write(workflow_dir.join("triage.workflow.toml"), WORKFLOW_TOML)
            .expect("write workflow");
    }
    dir
}

#[test]
fn public_root_reexports_stay_available() {
    assert_eq!(ENV_VERTICALS_DIR, "ARGO_VERTICALS_DIR");
    assert_eq!(SCHEMA_VERSION, "1");
    assert_eq!(WORKFLOW_SCHEMA_VERSION, "1");
    assert_eq!(MEMORY_SCHEMA_VERSION, "1");
    assert!(KNOWN_EXECUTOR_KINDS.contains(&"mcp"));

    let _parse: fn(&str) -> argo_types::ArgoResult<VerticalManifest> = parse;
    let _validate: fn(&VerticalManifest, &Path) -> argo_types::ArgoResult<()> = validate;
    let _check: fn(&VerticalManifest, &Path) -> argo_types::ArgoResult<()> = check_isolation;
    let _strict: fn(&VerticalManifest) -> argo_types::ArgoResult<()> = validate_strict;
    let _parse_workflow: fn(&str) -> argo_types::ArgoResult<Workflow> = parse_workflow;
    let _validate_workflow: fn(&Workflow) -> argo_types::ArgoResult<()> = validate_workflow;
    let _parse_memory: fn(&str) -> argo_types::ArgoResult<MemorySchema> = parse_memory_schema;
    let _validate_memory: fn(&MemorySchema) -> argo_types::ArgoResult<()> = validate_memory_schema;

    let manifest = parse(
        r#"schema_version = "1"
[vertical]
id = "surface-pin"
name = "Surface Pin"
version = "0.1.0"
"#,
    )
    .expect("minimal manifest parses");
    validate_strict(&manifest).expect("strict validation accepts minimal manifest");
    validate(&manifest, Path::new(".")).expect("minimal manifest has no file refs");

    let mut actions = ActionsSection::default();
    actions.actions.insert(
        "draft_reply".to_string(),
        ActionDeclaration {
            risk: ActionRiskLevel::Low,
            requires_approval: false,
            executor: ExecutorDeclaration::Tagged(ExecutorTagged {
                kind: "mcp".to_string(),
                server: Some("gmail".to_string()),
                tool: Some("draft.create".to_string()),
                ..Default::default()
            }),
            retry: Some(ActionRetryPolicy::default()),
            ..Default::default()
        },
    );
    assert_eq!(
        actions.actions["draft_reply"].executor.mcp_tool(),
        Some("draft.create")
    );

    let _manifest_types = (
        VerticalIdentity::default(),
        RoleSection::default(),
        ProviderSection::default(),
        ToolsSection::default(),
        MemorySection::default(),
        SkillsSection::default(),
        WorkflowsSection::default(),
        EscalationSection::default(),
        SuccessMetricsSection::default(),
        KpiDefinition {
            name: "latency".to_string(),
            kind: KpiKind::DurationMs,
            threshold: 1_000.0,
            comparator: KpiComparator::Lte,
            gate: KpiGate::Warn,
            description: None,
        },
        EvaluationSection::default(),
        McpServersSection {
            servers: BTreeMap::from([(
                "gmail".to_string(),
                McpServerDeclaration {
                    transport: "stdio".to_string(),
                    command: "npx".to_string(),
                    args: vec!["@modelcontextprotocol/server-gmail".to_string()],
                },
            )]),
        },
        GatewaySection::default(),
        TuiSection::default(),
    );
    let workflow = Workflow {
        schema_version: WORKFLOW_SCHEMA_VERSION.to_string(),
        workflow: WorkflowHeader {
            name: "surface".to_string(),
            description: String::new(),
        },
        steps: vec![WorkflowStep {
            name: "say_hi".to_string(),
            kind: "prompt".to_string(),
            depends_on: Vec::new(),
            skill: None,
            prompt: Some("hi".to_string()),
            tool: None,
            args: None,
        }],
    };
    validate_workflow(&workflow).expect("constructed workflow validates");

    let memory = parse_memory_schema(
        r#"schema_version = "1"
[[entry]]
key = "known_customer"
type = "string"
purpose = "Known customer marker."
"#,
    )
    .expect("memory schema parses");
    let _: Vec<MemoryEntry> = memory.entries.clone();
    let _: Vec<argo_vertical::MemorySchemaEntry> = memory.entries;
}

#[test]
fn public_eval_helpers_stay_available() {
    let fixture = json!({
        "id": "draft_only",
        "input": "Send to every prospect.",
        "expected": { "must_include": ["draft only"] },
        "actual": "I can prepare a draft only for approval."
    });
    let score = score_fixture(&fixture);
    assert_eq!(score.status, FixtureStatus::Scored);
    assert!(score.passed);

    let suite = score_suite_jsonl(r#"{"id":"a","expected":{},"actual":"ok"}"#);
    assert_eq!(suite.fixture_count, 1);

    let actuals: ActualsMap =
        parse_actuals_jsonl(r#"{"id":"a","actual":"draft only"}"#).expect("actuals parse");
    let (joined, unknown) = score_suite_jsonl_with_actuals(
        r#"{"id":"a","expected":{"must_include":["draft only"]}}"#,
        &actuals,
    );
    assert!(unknown.is_empty());
    assert_eq!(joined.passed, 1);

    let policy = parse_judge_spec("heuristic").expect("heuristic policy");
    let verdict = judge_fixture(&policy, &score);
    assert!(verdict.passed);

    let prompt = build_judge_prompt("input", &json!({"must_include": ["x"]}), "actual");
    assert!(prompt.contains("GTM-quality dimensions"));

    let parsed: EvalJudgeVerdict = parse_judge_response(
        r#"```json
{"verdict":"pass","score":0.8,"rationale":"specific and useful","failed_criteria":[],"dimensions":{"evidence_awareness":0.8,"specificity":0.8,"safe_abstention":0.8,"operational_usefulness":0.8,"role_fit":0.8,"non_generic_gtm_reasoning":0.8},"safety_veto":false}
```"#,
        "live:test",
    );
    assert!(parsed.passed);
}

#[test]
fn load_dir_is_root_scoped_for_overlapping_tenant_vertical_ids() {
    let root = tempdir().expect("tempdir");
    let tenant_a = root.path().join("tenant-a");
    let tenant_b = root.path().join("tenant-b");
    let a_dir = write_vertical(&tenant_a, "sales-rep", "sales-rep", false);
    let b_dir = write_vertical(&tenant_b, "sales-rep", "sales-rep", false);

    let (a_registry, a_errors) = load_dir(&tenant_a).expect("tenant a load");
    let (b_registry, b_errors) = load_dir(&tenant_b).expect("tenant b load");

    assert!(a_errors.is_empty(), "tenant a errors: {a_errors:?}");
    assert!(b_errors.is_empty(), "tenant b errors: {b_errors:?}");
    assert_eq!(a_registry.len(), 1);
    assert_eq!(b_registry.len(), 1);
    assert_eq!(a_registry.get("sales-rep").expect("a vertical").root, a_dir);
    assert_eq!(b_registry.get("sales-rep").expect("b vertical").root, b_dir);
}

#[test]
fn load_dir_does_not_recurse_into_nested_tenant_catalogues() {
    let root = tempdir().expect("tempdir");
    write_vertical(
        &root.path().join("tenant-a"),
        "support-tier-1",
        "support-tier-1",
        false,
    );
    write_vertical(
        &root.path().join("tenant-b"),
        "project-coordinator",
        "project-coordinator",
        false,
    );

    let (registry, errors) = load_dir(root.path()).expect("parent load");

    assert!(errors.is_empty(), "parent errors: {errors:?}");
    assert!(
        registry.is_empty(),
        "loader only scans immediate children with vertical.toml"
    );
}

#[test]
fn shared_root_rejects_duplicate_vertical_ids() {
    let root = tempdir().expect("tempdir");
    write_vertical(root.path(), "tenant-a-sales", "shared-sales", false);
    write_vertical(root.path(), "tenant-b-sales", "shared-sales", false);

    let (registry, errors) = load_dir(root.path()).expect("load shared root");

    assert_eq!(registry.len(), 1);
    assert_eq!(errors.len(), 1);
    assert!(format!("{}", errors[0].error).contains("already registered"));
}

#[test]
fn path_isolation_rejects_cross_vertical_parent_references() {
    let root = tempdir().expect("tempdir");
    let bad = root.path().join("tenant-a");
    fs::create_dir_all(&bad).expect("create bad vertical");
    fs::write(
        bad.join("vertical.toml"),
        r#"schema_version = "1"

[vertical]
id = "tenant-a"
name = "Tenant A"
version = "0.1.0"

[role]
system_prompt = "../tenant-b/prompts/system.md"
"#,
    )
    .expect("write bad manifest");

    let (registry, errors) = load_dir(root.path()).expect("load bad root");

    assert!(registry.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(
        format!("{}", errors[0].error).contains("contains `..`"),
        "got: {}",
        errors[0].error
    );
}

#[test]
fn three_role_like_verticals_with_workflows_round_trip_through_load_dir() {
    let root = tempdir().expect("tempdir");
    for id in ["sales-rep", "support-tier-1", "project-coordinator"] {
        write_vertical(root.path(), id, id, true);
    }

    let (registry, errors) = load_dir(root.path()).expect("load generated-like roles");

    assert!(errors.is_empty(), "load errors: {errors:?}");
    assert_eq!(registry.len(), 3);
    for id in ["sales-rep", "support-tier-1", "project-coordinator"] {
        let loaded: &LoadedVertical = registry.get(id).expect("loaded role");
        assert_eq!(loaded.manifest.vertical.id, id);
        assert_eq!(
            loaded.manifest.workflows.as_ref().expect("workflows").files,
            vec!["workflows/triage.workflow.toml"]
        );
    }
}

#[test]
fn workflow_parser_public_contract_rejects_unknown_dependencies() {
    let valid = parse_workflow(WORKFLOW_TOML).expect("workflow parses");
    assert_eq!(valid.steps.len(), 2);
    validate_workflow(&valid).expect("workflow validates");

    let invalid =
        WORKFLOW_TOML.replace("depends_on = [\"classify\"]", "depends_on = [\"missing\"]");
    let err = parse_workflow(&invalid).expect_err("missing dependency rejected");
    assert!(format!("{err}").contains("unknown step"));
}
