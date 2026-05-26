# argo-vertical loader surface + tenant-isolation audit

Date: 2026-05-25
Brief: `eval-argo-vertical-loader-tenant-isolation`
Verdict: P2 audit pinned with tests; no production code changed.

## Surface inventory

Root exports are declared in `crates/argo-vertical/src/lib.rs:75-133`.

| Area | Public functions/constants | Public structs/enums/types |
|---|---|---|
| Discovery | `ENV_VERTICALS_DIR`, `default_dir()` | - |
| Eval scoring | `score_fixture`, `score_suite_jsonl` | `FixtureScore`, `FixtureStatus`, `ScoreCheck`, `SuiteScore` |
| Eval actuals | `parse_actuals_jsonl`, `score_suite_jsonl_with_actuals` | `ActualsMap`, `ActualsParseError` |
| Eval judge | `build_judge_prompt`, `judge_fixture`, `parse_judge_response`, `parse_judge_spec` | `EvalJudgeVerdict`, `JudgePolicy` |
| Loader + isolation | `load_dir`, `check_isolation` | `LoadError` |
| Manifest parser | `parse`, `SCHEMA_VERSION` | `VerticalManifest`, `VerticalIdentity`, `RoleSection`, `ProviderSection`, `ToolsSection`, `MemorySection`, `SkillsSection`, `WorkflowsSection`, `EscalationSection`, `SuccessMetricsSection`, `KpiDefinition`, `KpiKind`, `KpiComparator`, `KpiGate`, `EvaluationSection`, `GatewaySection`, `TuiSection` |
| Action/MCP schema | - | `ActionsSection`, `ActionDeclaration`, `ActionRetryPolicy`, `ActionRiskLevel`, `ExecutorDeclaration`, `ExecutorTagged`, `McpServersSection`, `McpServerDeclaration` |
| Strict validation | `validate_strict`, `KNOWN_EXECUTOR_KINDS` | `ManifestValidationError` |
| Memory schema | `parse_memory_schema`, `validate_memory_schema`, `MEMORY_SCHEMA_VERSION` | `MemorySchema`, `MemorySchemaEntry`, `MemoryEntry` |
| Registry | `Registry::new`, `insert`, `get`, `list`, `len`, `is_empty` | `Registry`, `LoadedVertical` |
| Structural validation | `validate` | - |
| Versioning | `archive_existing_vertical`, `next_version_dir` | - |
| Workflow schema | `parse_workflow`, `validate_workflow`, `WORKFLOW_SCHEMA_VERSION` | `Workflow`, `WorkflowHeader`, `WorkflowStep` |

The new `loader_surface_pin_test.rs` compiles through these root exports rather than private modules, so future public API drift fails in `argo-vertical` itself.

## Loader entry points

`load_dir` is the main catalogue entry point. It scans only immediate subdirectories of the supplied root, skips children without `vertical.toml`, parses each manifest, validates file references, and inserts by `vertical.id` into an in-memory `Registry` (`loader.rs:35-68`, `registry.rs:40-63`).

Workspace call sites cluster into seven groups:

1. Gateway catalogue and detail routes: `routes_verticals.rs`, `routes_verticals_list_get.rs`, `routes_verticals_doctor.rs`, `routes_verticals_lint.rs`, `routes_verticals_run.rs`, `routes_workers_hire.rs`, `routes_worker_detail.rs`, `routes_worker_policies.rs`, `routes_onboarding_helpers.rs`, `onboarding_provision.rs`, and `routes_post_sessions.rs`. These read the directory from `handlers::vertical::resolve_verticals_dir()`, which delegates to `argo_vertical::default_dir()`.
2. CLI vertical/workflow commands: `handlers/vertical.rs`, `vertical_show.rs`, `vertical_lint.rs`, `vertical_eval.rs`, `vertical_run.rs`, `vertical_workflow.rs`, `vertical_publish.rs`, `vertical_archive.rs`, `new.rs`, and `eval_run.rs`. These also read the default verticals root or an operator-provided equivalent.
3. Worker/action CLI surfaces: `worker_actions_*`, `work.rs`, `work_steps.rs`, `work_jobs.rs`, `worker_portrait.rs`, `worker_doctor.rs`, `mcp_tools.rs`, `mcp_health.rs`, and `mcp_registry.rs`. These use the loaded registry to resolve action vocabulary, worker display data, policy data, or MCP declarations.
4. Runtime runner surfaces: `crates/argo-agent-runner/src/dispatch.rs` and `turn_vertical.rs` call `load_dir(default_dir or resolved dir)` to resolve action dispatch and vertical context at turn time.
5. Genesis surfaces: `routes_genesis_stream.rs`, `genesis_export.rs`, `genesis_refine.rs`, `crates/argo-genesis/src/pipeline.rs`, and `pipeline_seed.rs`. The streaming route and Genesis tests write generated output under a root and round-trip it with `load_dir`.
6. Workflow parsing call sites: `routes_workflow_execute.rs`, `vertical_workflow.rs`, and `crates/argo-store/src/workflow_definition_write.rs` call `parse_workflow` directly on stored or on-disk TOML before execution or DB insert.
7. Asset/inventory tests: vertical asset smoke tests, `claim_3_live_genesis_to_run`, `e2e_v1_demo`, and `loader_surface_pin_test.rs` use `load_dir` to pin catalogues and generated trees.

The broad `parse(...)` symbol is ambiguous across the workspace; for this audit it means `argo_vertical::parse`, whose in-crate use is validation/tests and whose higher-level consumers usually arrive through `load_dir`.

## Tenant isolation answer

Does any loader path bind `tenant_id`? **No.** `load_dir` accepts only `&Path` (`loader.rs:35`), and `Registry` is keyed only by `vertical.id` (`registry.rs:23-28`). There is no tenant identifier in `LoadedVertical`, `Registry`, or the public loader signature.

Can two tenants see overlapping vertical IDs? **Only if the caller scopes each tenant to a separate root.** The loader itself is filesystem-root scoped: calling `load_dir(root/tenant-a)` and `load_dir(root/tenant-b)` can load the same `vertical.id` independently. Calling `load_dir(shared_root)` with two immediate child manifests using the same id produces a duplicate-id error through `Registry::insert` (`registry.rs:40-49`). Calling `load_dir(parent)` where tenants are nested one level deeper loads nothing, because the loader does not recurse (`loader.rs:60-64`). Static path isolation is present for manifest file references: parent traversal and symlink escape are rejected by `check_isolation` (`isolation.rs:44-91`).

## Genesis round-trip

Direct `argo-vertical` tests cannot call `argo_genesis::plan` without adding a dev-dependency cycle, because `argo-genesis` already depends on `argo-vertical`. The existing canonical Genesis round-trip is already pinned in `crates/argo-genesis/tests/five_charters_round_trip.rs`: five role descriptions are planned, written to tempdirs, and loaded via `argo_vertical::load_dir`; a second test writes all five into one parent directory and loads the catalogue together. This audit adds an `argo-vertical`-side consumer-contract pin for three generated-role-shaped directories (`sales-rep`, `support-tier-1`, `project-coordinator`) with workflow references, plus surface and isolation pins.

## Test pins added

New file: `crates/argo-vertical/tests/loader_surface_pin_test.rs`

Pins:

1. Root re-export surface compiles through public APIs.
2. Eval helper public surface compiles and returns expected verdicts.
3. Separate tenant roots can independently load overlapping vertical IDs.
4. Parent roots do not recurse into nested tenant catalogues.
5. Shared roots reject duplicate vertical IDs.
6. Parent-directory manifest references are rejected by static isolation.
7. Three role-shaped verticals with workflow files round-trip through `load_dir`.
8. `parse_workflow` rejects dangling step dependencies.

## Follow-up briefs

1. `mission-vertical-loader-tenant-bound-registry`: add an optional tenant-bound registry wrapper above `argo-vertical` so gateway callers cannot accidentally share one root across tenants.
2. `eval-vertical-loader-callsite-root-audit`: classify every CLI/gateway call site as global catalogue, tenant-scoped catalogue, or test-only, then pin the expected root source.
3. `mission-genesis-vertical-roundtrip-shared-helper`: move the Genesis round-trip helper into a shared test-support crate or non-cyclic public helper so both `argo-genesis` and `argo-vertical` can reuse the same fixture matrix.
4. `mission-vertical-loader-recursion-policy-doc`: document the non-recursive immediate-child loader contract in operator docs and add an error-mode option if nested tenant roots become product-supported.
5. `eval-workflow-parser-runtime-parity`: compare `parse_workflow` validation with the gateway workflow execution path and add a test for each stored workflow shape accepted by `routes_workflow_execute`.
