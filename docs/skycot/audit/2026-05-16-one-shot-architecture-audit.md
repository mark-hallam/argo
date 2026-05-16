# Skycot one-shot architecture audit, 2026-05-16

Scope: read-only audit of the Rust workspace at commit `a22c9adc` (`status: #1731 MERGED (codex-smoke-script-target-dir-inheritance)`). This report is the only write produced by the audit.

The audit compares current code against the source-of-truth documents named in the audit request: `MISSION.md`, `CLAUDE.md`, `V1_DEFINITION.md`, `BUILD_PLAN_V11.md`, `MISSION_BACKLOG.md`, the 2026-05-14 Phase A-D audits, and `DRIFT_PREVENTION.md`.

I did not run the full cargo test suite because the request was read-only and cargo builds/tests write to `target/`. I used read-only source inspection and read-only shell queries.

> **NOTE 2026-05-16:** File restored into main repo from conductor session paste after the codex temp worktree was cleaned up (same pattern as the 2026-05-15 audit). Content is verbatim from the codex audit output. F-05 and F-08 corrected from initial partial paste; F-06 and F-07 added.

## Source-of-truth frame

The six load-bearing mission claims are: persistent teammate identity, delegation as the unit, platform-built workers, shared/learned skills, MCP plus computer-use execution, and multi-channel approvals (`docs/skycot/MISSION.md:26`, `docs/skycot/MISSION.md:29`, `docs/skycot/MISSION.md:32`, `docs/skycot/MISSION.md:36`, `docs/skycot/MISSION.md:40`, `docs/skycot/MISSION.md:45`). The worker contract specifically includes `memory_namespace`, `auto_approval_policy`, and channel preferences (`docs/skycot/MISSION.md:56`).

The v1 gate is all five MISSION success scenarios plus Tracks L and M plus the demo recording (`docs/skycot/V1_DEFINITION.md:5`). The success scenarios require a generated worker running over Slack and computer-use, skill propagation across workers, handoff with shared memory, budget/time-bound auto-approval, and a complete audit trail (`docs/skycot/MISSION.md:203`, `docs/skycot/MISSION.md:207`, `docs/skycot/MISSION.md:214`, `docs/skycot/MISSION.md:219`, `docs/skycot/MISSION.md:223`, `docs/skycot/MISSION.md:227`; repeated at `docs/skycot/V1_DEFINITION.md:41`).

The core invariants are single-binary Rust, tenant-aware primitives, RBAC, provider manifests, MCP-first integrations, and CI rejection of files over 500 LOC, functions over 80 LOC, unmarked `unsafe`, unwrap/expect outside tests, global mutable state, and undocumented public APIs (`CLAUDE.md:33`, `CLAUDE.md:42`, `CLAUDE.md:54`). The operator-locked v1 decisions also make macOS the v1 computer-use path, enterprise the connector priority, type canonicalization mandatory, skill-library runtime load a v1 ship-blocker, and migration slot collisions P0 (`CLAUDE.md:105`).

The operative plan says the current v1 stop condition is the mission-MVP demo recording plus extended smoke, including BrowserExecutor and Slack approval (`docs/build-plan/BUILD_PLAN_V11.md:263`). Its sequencing says to stop credibility bleed first with CI, headline honesty, gateway auth, and load-bearing wires, then production perimeter, then mission-claim closure (`docs/build-plan/BUILD_PLAN_V11.md:357`).

## Delta since the 2026-05-14 audits

Shipped or substantially improved:

- The Phase A zero-LOC connector-stub finding is no longer live. The current workspace has 83 crate manifests and the prior zero-LOC stub crates `apollo`, `mailgun`, `resend`, `signal`, `telegram`, `zoom`, and `zoominfo` are absent from `crates/`.
- The Phase D `version = "0.0.1"` Cargo drift is no longer live; `rg 'version = "0\.0\.1"' crates/*/Cargo.toml` returns no matches.
- The v1 skill-library runtime load ship-blocker is shipped. `load_prompt_skills` opens `ARGO_SKILL_LIBRARY_DB` and calls `load_prompt_skills_with_library` (`crates/argo-agent-runner/src/turn_helpers.rs:34`), the runtime path constructs `LayeredLookup::with_library` (`crates/argo-agent-runner/src/turn_helpers.rs:61`), and `LayeredLookup::with_library` exists in the skill crate (`crates/argo-skill/src/library.rs:84`). The integration test verifies a published library skill reaches the worker prompt while a private library skill does not (`crates/argo-agent-runner/src/turn_library_integration_tests.rs:23`, `crates/argo-agent-runner/src/turn_library_integration_tests.rs:65`, `crates/argo-agent-runner/src/turn_library_integration_tests.rs:71`).
- Claim 4 has a real end-to-end smoke test: teach -> propose -> eval -> review -> publish -> reload -> visible (`crates/argo-skill/tests/end_to_end_claim4_loop.rs:1`). It covers publication and reload into a worker prompt (`crates/argo-skill/tests/end_to_end_claim4_loop.rs:144`, `crates/argo-skill/tests/end_to_end_claim4_loop.rs:161`) and rejects tenant-private library leakage (`crates/argo-skill/tests/end_to_end_claim4_loop.rs:168`).
- Genesis eval seed coverage improved. Generated verticals now assert at least five eval seeds (`crates/argo-genesis/tests/eval_seed_generation.rs:22`, `crates/argo-genesis/tests/eval_seed_generation.rs:32`) and the legacy warning path still emits deterministic seeds (`crates/argo-genesis/tests/eval_seed_generation.rs:68`, `crates/argo-genesis/tests/eval_seed_generation.rs:86`).
- The extended smoke script now includes BrowserExecutor, macOS computer-use, Slack approval, multi-window fallback, multi-worker handoff memory, cross-tenant skill discovery, and Claim 4 skill marketplace loop checks (`scripts/smoke-skycot.sh:247`, `scripts/smoke-skycot.sh:265`, `scripts/smoke-skycot.sh:275`, `scripts/smoke-skycot.sh:285`, `scripts/smoke-skycot.sh:292`, `scripts/smoke-skycot.sh:299`, `scripts/smoke-skycot.sh:306`).
- Per-worker prompt memory hydration is partially shipped: the agent runner composes worker/self or shared/team memory into the system prompt using `MemoryNs` (`crates/argo-agent-runner/src/turn_memory.rs:41`, `crates/argo-agent-runner/src/turn_memory.rs:74`, `crates/argo-agent-runner/src/turn_memory.rs:84`).

Still live or newly regressed:

- Migration slot collisions remain live despite being identified as P0 in `CLAUDE.md` (`CLAUDE.md:113`) and the v1 checklist (`docs/skycot/V1_DEFINITION.md:163`).
- CI and production gates are still not active as GitHub workflows; this contradicts the backlog row that says CI re-enable is `[SHIPPED]` (`docs/build-plan/MISSION_BACKLOG.md:75`) and the plan's Phase A production-perimeter sequencing (`docs/build-plan/BUILD_PLAN_V11.md:320`).
- Type canonicalization drift is worse than the Phase A baseline: the current lint reports 33 duplicate public type names, while `CLAUDE.md` says not to introduce new parallel public types (`CLAUDE.md:111`) and `DRIFT_PREVENTION.md` describes this as a known failure mode (`docs/skycot/DRIFT_PREVENTION.md:59`).
- Several new surfaces are present but not wired all the way into runtime loops, especially per-worker learning cron, memory tool namespaces, policy-as-code, and multi-channel dispatch.

## Findings

### F-01 P0: CI, deny, audit, and deploy-smoke gates are disabled by filename

Source-of-truth: `CLAUDE.md` says CI rejects files over 500 LOC, functions over 80 LOC, unmarked `unsafe`, unwrap/expect outside tests, global mutable state, and undocumented APIs (`CLAUDE.md:54`). The v1 checklist requires `scripts/smoke-skycot.sh` to pass (`docs/skycot/V1_DEFINITION.md:165`). The plan says the operative stop condition includes extended smoke with BrowserExecutor and Slack approval (`docs/build-plan/BUILD_PLAN_V11.md:263`).

Current state: the main CI definition is checked in as `.github/workflows/ci.yml.disabled`, so its triggers at `.github/workflows/ci.yml.disabled:1` through `.github/workflows/ci.yml.disabled:6` do not run. The `loc-budget` job would fail files over 500 LOC if active (`.github/workflows/ci.yml.disabled:167`). The duplicate public type lint is warning-only even inside the disabled workflow (`.github/workflows/ci.yml.disabled:27`). Deploy smoke is also disabled by filename despite having push, schedule, and workflow_dispatch triggers (`.github/workflows/deploy-smoke.yml.disabled:10`, `.github/workflows/deploy-smoke.yml.disabled:12`, `.github/workflows/deploy-smoke.yml.disabled:22`). Deny and audit are disabled too (`.github/workflows/deny.yml.disabled:1`, `.github/workflows/audit.yml.disabled:1`).

Impact: every other quality finding below can land unnoticed. This is the largest delta between the ledger/backlog and reality: `MISSION_BACKLOG.md` marks CI re-enable shipped (`docs/build-plan/MISSION_BACKLOG.md:75`), but the executable gates are not active.

Remediation: restore active workflow filenames or document and wire a self-hosted runner path. Make the extended smoke script a required gate. Make parallel public type lint enforce after triage. Keep disabled workflow files only as archived history or delete them.

### F-02 P0: migration slot collisions are still live

Source-of-truth: migration slot collisions are explicitly P0 operational risk (`CLAUDE.md:113`) and a v1 ship-blocker (`docs/skycot/V1_DEFINITION.md:163`). Additive migrations are a core maintainability rule (`CLAUDE.md:37`).

Current duplicate slots:

- slot 19: `crates/argo-store/migrations/19__anomaly.sql:1`, `crates/argo-store/migrations/19__worker_fs_root.sql:1`
- slot 22: `crates/argo-store/migrations/22__tenant_archived.sql:1`, `crates/argo-store/migrations/22__roster.sql:1`
- slot 23: `crates/argo-store/migrations/23__handoff_routing.sql:1`, `crates/argo-store/migrations/23__workflow_checkpoints.sql:1`
- slot 24: `crates/argo-store/migrations/24__skill_quality_score.sql:1`, `crates/argo-store/migrations/24__operator_feedback.sql:1`, `crates/argo-store/migrations/24__vertical_archived.sql:1`
- slot 26: `crates/argo-store/migrations/26__worker_budget_alert.sql:1`, `crates/argo-store/migrations/26__spend_baseline.sql:1`, `crates/argo-store/migrations/26__data_classification.sql:1`
- slot 27: `crates/argo-store/migrations/27__handoff_auctions.sql:1`, `crates/argo-store/migrations/27__spend_anomaly.sql:1`
- slot 28: `crates/argo-store/migrations/28__tenant_quota.sql:1`, `crates/argo-store/migrations/28__recovery_cache.sql:1`, `crates/argo-store/migrations/28__role_duties.sql:1`
- slot 30: `crates/argo-store/migrations/30__learned_plan_templates.sql:1`, `crates/argo-store/migrations/30__worker_charter.sql:1`
- slot 45: `crates/argo-store/migrations/45__cu_screenshot_index.sql:1`, `crates/argo-store/migrations/45__worker_dependency.sql:1`, `crates/argo-store/migrations/45__worker_level.sql:1`
- slot 51: `crates/argo-store/migrations/51__audit_worker_deleted_at.sql:1`, `crates/argo-store/migrations/51__worker_skill.sql:1`
- slot 75: `crates/argo-store/migrations/75__scim_provisioned_tombstones.sql:1`, `crates/argo-store/migrations/75__siem_dispatch_cursor.sql:1`

The migration-chain test also has special duplicate-column suppressions (`crates/argo-store/tests/migration_chain.rs:35`, `crates/argo-store/tests/migration_chain.rs:41`, `crates/argo-store/tests/migration_chain.rs:42`), with explicit duplicate handlers for `session.worker_id` and `skill.shared` (`crates/argo-store/tests/migration_chain.rs:320`, `crates/argo-store/tests/migration_chain.rs:325`).

Impact: schema application order is ambiguous and can differ between fresh databases, upgraded databases, and developer machines. Duplicate suppression turns schema drift into a tolerated warning.

Remediation: renumber migrations into a strictly monotonic chain, add a pre-sqlx validator that fails duplicate slots before applying any SQL, and delete duplicate-column suppressions after reconciliation tests prove real legacy upgrade paths.

### F-03 P1: runtime memory tools are not bound to worker/team namespaces

Source-of-truth: workers must have a persistent `memory_namespace` (`docs/skycot/MISSION.md:56`) and handoff must allow Worker B to continue from shared memory without re-asking (`docs/skycot/MISSION.md:219`). The backlog calls out `query_scored` and worker/shared-team memory tool addressing as open claim-1 work (`docs/build-plan/MISSION_BACKLOG.md:151`, `docs/build-plan/MISSION_BACKLOG.md:152`).

Current state: the prompt hydration path uses `MemoryNs` and is partially correct (`crates/argo-agent-runner/src/turn_memory.rs:84`). The tool path is not. `Memory` documents a namespace field required by `query_scored`, but says old `Memory::new` callers leave it `None` (`crates/argo-tool/src/tools/memory.rs:37`). The tool description claims private actions use the worker namespace and team actions address tenant-shared memory (`crates/argo-tool/src/tools/memory.rs:127`). `query_scored` rejects explicit namespaces (`crates/argo-tool/src/tools/memory_query.rs:70`) and then errors unless the tool is bound to a worker namespace (`crates/argo-tool/src/tools/memory_query.rs:79`).

The agent runner constructs the runtime memory tool with `Memory::new`, not `Memory::with_namespace` (`crates/argo-tool/src/tools/mod.rs:130`, `crates/argo-tool/src/tools/mod.rs:136`, `crates/argo-tool/src/tools/mod.rs:138`). The dispatch context only accepts `(MemoryStore, WorkspaceId, Option<String>)`, not tenant/worker namespace (`crates/argo-agent-runner/src/dispatch.rs:69`). `turn_helpers` populates that tuple from `first_workspace()` and vertical scope only (`crates/argo-agent-runner/src/turn_helpers.rs:154`, `crates/argo-agent-runner/src/turn_helpers.rs:159`).

Impact: `query_scored`, `list_team`, `read_team`, and `write_team` can be tested in isolation, but the real agent runner cannot use them as documented. Legacy `list/read/write` stay workspace/vertical-global, while the tool schema tells the model they are worker-private.

Remediation: thread `tenant_id`, `worker_id`, and `MemoryNs` through `build_dispatch_ctx` and `builtin_tools_with_runtime`; construct `Memory::with_namespace` for the agent runner; add a runtime integration test where the model/tool call successfully executes `query_scored` and cannot read another worker's private memory.

### F-04 P1: per-worker skill learning cron is present but not scheduled by runtime boot

Source-of-truth: Claim 4 requires shared and learned skills with a learning loop (`docs/skycot/MISSION.md:36`), and v1 success scenario #2 requires a learned skill to propagate to another worker (`docs/skycot/MISSION.md:214`). Drift prevention calls "land scaffold now, wire later" the failure mode (`docs/skycot/DRIFT_PREVENTION.md:45`, `docs/skycot/DRIFT_PREVENTION.md:54`).

Current state: `per_worker_learning_tick` and `spawn_per_worker_learning_cron` exist (`crates/argo-skill/src/per_worker_learning_cron.rs:62`, `crates/argo-skill/src/per_worker_learning_cron.rs:119`). The cron is exported from `argo-skill` (`crates/argo-skill/src/lib.rs:94`). The only non-test/code references found are the export and the build ledger note (`docs/build-plan/BUILD_STATUS.md:648`); there is no production caller in gateway boot, worker supervisor, duty scheduler, or orchestrator.

Impact: the substrate can propose worker overlays from ratings, but deployed workers will not learn autonomously unless an external caller manually invokes the tick. The Claim 4 loop is therefore testable but not yet a runtime loop.

Remediation: spawn the cron from the worker/gateway runtime that owns worker lifecycle, pass tenant/worker config, retain shutdown handles, and add an end-to-end test that ratings in the store cause an overlay proposal without directly calling `per_worker_learning_tick`.

### F-05 P1: fallback notification routing bypasses per-risk channel filters

Source-of-truth: multi-channel approval requires per-action manifest channels at risk levels and per-operator preferences (`docs/skycot/MISSION.md:115`, `docs/skycot/MISSION.md:117`). Claim 6 is not merely "send somewhere"; it is channel-appropriate approval gates (`docs/skycot/MISSION.md:108`).

Current state: normal fan-out has a `matches_risk` filter (`crates/argo-cli/src/handlers/worker_actions_notify.rs:169`) and applies it before sending (`crates/argo-cli/src/handlers/worker_actions_notify.rs:228`). The fallback path in `try_dispatch_on_ask` builds a notification and passes `config.channels` directly into fallback dispatch (`crates/argo-cli/src/handlers/worker_actions_notify.rs:277`, `crates/argo-cli/src/handlers/worker_actions_notify.rs:283`, `crates/argo-cli/src/handlers/worker_actions_notify.rs:285`, `crates/argo-cli/src/handlers/worker_actions_notify.rs:288`). The fallback module explicitly says risk filtering happens in the caller (`crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs:35`), but `run_one_attempt` picks the first channel kind match without checking `risk_level` (`crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs:74`, `crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs:81`).

Tests cover risk filtering for normal dispatch (`crates/argo-cli/src/handlers/worker_actions_notify_tests.rs:236`) but the fallback fixture has no risk filters (`crates/argo-cli/src/handlers/worker_actions_notify_fallback_tests.rs:89`).

Impact: a low-risk action can be sent to a high-risk-only fallback channel, or a high-risk action can skip the intended high-risk channel if the first configured kind mismatch wins.

Remediation: filter the `channel_refs` by `matches_risk` before fallback dispatch, or move risk matching into `dispatch_with_fallback`; add a regression test with two channels of the same kind and mismatched risk filters.

### F-06 P1: policy-as-code parses enterprise policy fields but approval gating ignores them

Source-of-truth: Claim 2 and Claim 6 require delegation with policies and approval routing; Claim 1 says each worker has an `auto_approval_policy` (`docs/skycot/MISSION.md:56`). The v1 success scenario requires weekday business hours and daily budget constraints to be observed (`docs/skycot/MISSION.md:223`). The backlog explicitly calls for roster lookup before `resolve_effective_policy` (`docs/build-plan/MISSION_BACKLOG.md:172`). `CLAUDE.md` requires multi-tenant primitives and RBAC (`CLAUDE.md:46`, `CLAUDE.md:48`).

Current state: `argo-identity::policy` documents a resolution order that includes action-specific rules, worker policy, roster policy, workspace policy, and workspace defaults (`crates/argo-identity/src/policy.rs:17`). It also has role and tenant selectors (`crates/argo-identity/src/policy.rs:83`) and effective fields for signatures, quiet hours, and audit-chain requirements (`crates/argo-identity/src/policy.rs:187`). But `worker_actions_auto_approval` calls `resolve_effective_policy` with empty role and tenant strings (`crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:192`, `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:195`, `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:196`), with a comment saying role and tenant context are not threaded through (`crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:179`, `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:183`). The gate then consumes only `effective.auto_approval` (`crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:199`).

Impact: non-wildcard tenant/role policy files, roster policy inheritance, multi-signature requirements, quiet hours, and audit-chain-required flags are either inert or only partially represented. This undercuts the enterprise approval story and the budget/time-window success scenario.

Remediation: resolve actual tenant, role, worker, and roster before evaluating approvals; enforce `require_signatures`, `quiet_hours`, and `audit_chain_required` in the gate or fail closed when present but unsupported; add tests with a non-wildcard tenant policy, role policy, roster policy, quiet hours, and N-of-M signatures.

### F-07 P1: channel implementations exist but the auto-approval dispatcher cannot reach several mission channels

Source-of-truth: Claim 6 names email, Slack, SMS, push, voice, and web (`docs/skycot/MISSION.md:45`), and the approval spec says each `NotifyChannel` implementation handles outbound and inbound where applicable (`docs/skycot/MISSION.md:115`). The backlog still marks iMessage dispatcher and voice route consistency open (`docs/build-plan/MISSION_BACKLOG.md:285`, `docs/build-plan/MISSION_BACKLOG.md:286`) and Teams/Matrix as channel work (`docs/build-plan/MISSION_BACKLOG.md:298`, `docs/build-plan/MISSION_BACKLOG.md:300`).

Current state: `EnvChannelSender` supports `push`, `webinbox`, `slack`, `email`, `sms`, `discord`, `chat`, and `sse` only (`crates/argo-cli/src/handlers/worker_actions_notify.rs:309`, `crates/argo-cli/src/handlers/worker_actions_notify.rs:310`). Channel implementations exist for voice (`crates/argo-channels/src/voice.rs:154`), iMessage (`crates/argo-channels/src/imessage.rs:151`), WhatsApp (`crates/argo-channels/src/whatsapp.rs:126`), Teams (`crates/argo-channels/src/teams.rs:389`), and Matrix (`crates/argo-channels/src/matrix.rs:243`), but those names fall into the unsupported-channel error path (`crates/argo-cli/src/handlers/worker_actions_notify.rs:319`).

Impact: the codebase can appear to have many channel implementations while the auto-approval ask path cannot dispatch to them. This is exactly the drift pattern of infrastructure shipped without runtime wiring.

Remediation: add dispatcher arms and config/credential loading for each intended v1 channel; for deferred channels, remove them from v1 claims and docs or mark them explicitly unreachable; add outbound and inbound round-trip tests per supported channel.

### F-08 P1: duplicate public type drift remains broad and unenforced

Source-of-truth: type canonicalization is an operator-locked v1 decision (`CLAUDE.md:111`). Drift prevention says duplicate overlapping public types are a bug (`CLAUDE.md:173`) and documents the parallel-type lint (`docs/skycot/DRIFT_PREVENTION.md:91`).

Current state: `python3 scripts/lint-parallel-pub-types.py` reports 33 duplicate public type names. High-risk examples include:

- `AuditEvent`: `crates/argo-audit/src/sink.rs:71`, `crates/argo-siem/src/sink.rs:13`
- `AuditRow`: `crates/argo-cli/src/handlers/gateway/routes_worker_overview_kpis.rs:23`, `crates/argo-store/src/models.rs:153`
- `AuditSink`: `crates/argo-audit/src/sink.rs:59`, `crates/argo-mcp-image-gen/src/server.rs:35`
- `BackendKind`: `crates/argo-cli/src/handlers/worker_actions_executor_code.rs:112`, `crates/argo-mcp-image-gen/src/config.rs:8`
- `BudgetSnapshot`: `crates/argo-executor-computer-use/src/budget.rs:150`, `crates/argo-tui/src/spend_monitor.rs:26`
- `ComputerUseExecutor`: `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs:78`, `crates/argo-executor-computer-use/src/executor.rs:41`
- `MemoryLayer`: `crates/argo-genesis/src/memory_schema.rs:30`, `crates/argo-memory/src/layers.rs:24`
- `PendingApproval`: `crates/argo-cli/src/handlers/audit_report_aggregate.rs:33`, `crates/argo-tui/src/approval_tray.rs:13`, `crates/argo-voice-tui/src/runner.rs:24`
- `Principal`: `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs:14`, `crates/argo-types/src/capability.rs:255`
- `ProviderRequest`: `crates/argo-executor-document/src/extract/provider.rs:36`, `crates/argo-provider/src/transport.rs:24`
- `ProviderResponse`: `crates/argo-executor-document/src/extract/provider.rs:67`, `crates/argo-provider/src/transport.rs:64`
- `StopReason`: `crates/argo-cli/src/handlers/work_loop.rs:45`, `crates/argo-executor-computer-use/src/anthropic.rs:174`, `crates/argo-provider/src/transport.rs:132`
- `ToolSchema`: `crates/argo-provider/src/transport.rs:52`, `crates/argo-tool/src/tool.rs:156`
- `TrustGrade`: `crates/argo-skill/src/trust.rs:33`, `crates/argo-store/src/learned_plan_template.rs:72`
- `Workflow`: `crates/argo-vertical/src/workflow.rs:60`, `crates/argo-workflow/src/dag.rs:100`

Impact: duplicate public types split contracts across crates and make it easy to wire the wrong data shape into RBAC, audit, policy, provider, and workflow paths.

Remediation: triage duplicates into canonical homes, starting with security/audit/provider/workflow types; re-export canonical types from `argo-types` or the owning domain crate; change the lint to enforce in active CI after the allowlist is explicit and reviewed.

### F-09 P1: LOC and function-size invariants are violated and the function gate is missing

Source-of-truth: no file over 500 LOC and no function over 80 LOC are CI rejection rules (`CLAUDE.md:56`, `CLAUDE.md:57`).

Current state: two Rust files exceed 500 LOC: `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs:635` and `crates/argo-tool/src/tools/memory_tests.rs:585`. The local CI script only runs a function-size gate if `scripts/check-fn-loc.sh` exists; otherwise it prints a skip and returns success (`scripts/local-ci-check.sh:145`, `scripts/local-ci-check.sh:147`, `scripts/local-ci-check.sh:152`). That script is absent. A read-only approximate scan found many functions over 80 LOC, including `crates/argo-tui/src/input.rs:27`, `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs:136`, `crates/argo-cli/src/handlers/worker_actions.rs:35`, `crates/argo-cli/src/handlers/audit.rs:9`, `crates/argo-cli/tests/e2e_v1_demo.rs:45`, and `crates/argo-skill/tests/end_to_end_claim4_loop.rs:54`.

Impact: the architectural budgeting rules have no reliable enforcement. Large functions and test files are already accumulating in mission-critical routes.

Remediation: split the two >500 LOC files immediately; add `scripts/check-fn-loc.sh`; run it in local CI and active CI; decide whether tests get a separate budget, but encode that decision explicitly rather than silently violating the invariant.

### F-10 P1: macOS computer-use has an adapter, but the v1 evidence path is still too weak

Source-of-truth: macOS is the v1 demo path for computer-use (`CLAUDE.md:109`; `docs/skycot/V1_DEFINITION.md:7`), and v1 success scenario #1 requires the worker to post via computer-use without the operator opening a terminal (`docs/skycot/MISSION.md:207`).

Current state: the macOS executor adapter does reach the macOS crate on macOS (`crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:56`, `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:80`, `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:87`) and `ExecutorKind::ComputerUseMacOS` resolves from vertical manifests (`crates/argo-cli/src/handlers/worker_actions_executor.rs:132`, `crates/argo-cli/src/handlers/worker_actions_executor.rs:137`, `crates/argo-cli/src/handlers/worker_actions_executor.rs:272`). However, the dedicated `cu_macos_dispatch_smoke` test says it verifies the runtime reaches the dispatch arm (`crates/argo-cli/tests/cu_macos_dispatch_smoke.rs:3`) but the actual test only executes `argo --help` (`crates/argo-cli/tests/cu_macos_dispatch_smoke.rs:29`, `crates/argo-cli/tests/cu_macos_dispatch_smoke.rs:31`). The Notes smoke test returns success without exercising native automation unless `ARGO_NATIVE_MACOS_INTEGRATION=1` and target OS is macOS (`crates/argo-executor-computer-use-macos/tests/notes_smoke.rs:23`, `crates/argo-executor-computer-use-macos/tests/notes_smoke.rs:24`, `crates/argo-executor-computer-use-macos/tests/notes_smoke.rs:28`).

Impact: the code path exists, but the automated proof is easy to green without proving the v1 "human-on-a-Mac task" flow. With CI disabled, even the skip-heavy smoke is not a required gate.

Remediation: replace `cu_macos_dispatch_smoke` with a real action execution against a manifest using `computer_use_macos`; require a dedicated macOS runner or operator demo machine for `ARGO_NATIVE_MACOS_INTEGRATION=1`; keep non-macOS clean-error tests, but do not count them as v1 computer-use proof.

### F-11 P2: skill library runtime load is shipped, but prompt selection is too broad

Source-of-truth: Claim 4 requires workers to draw from a shared learned skill library with provenance and trust (`docs/skycot/MISSION.md:36`). The drift rule says public surfaces need a named consumer and should avoid parallel or dead scaffolds (`CLAUDE.md:169`).

Current state: when no session-attached skill exists and a library is configured, `load_prompt_skills_with_library` lists library/workspace skills for action `"low"` (`crates/argo-agent-runner/src/turn_helpers.rs:61`, `crates/argo-agent-runner/src/turn_helpers.rs:62`). It filters private library rows by provenance (`crates/argo-agent-runner/src/turn_helpers.rs:65`) and filters workspace skills by vertical only (`crates/argo-agent-runner/src/turn_helpers.rs:68`). There is no action-type, vertical, tenant trust-policy, prompt budget, or relevance cap for marketplace library skills in this path.

Impact: the old ship-blocker is fixed, but the next failure mode is prompt pollution or over-broad skill exposure as the marketplace grows. This is correctness and trust-risk drift, not a reason to redesign the skill architecture.

Remediation: add a relevance selector keyed by action type, vertical, tags, and trust policy; cap rendered marketplace skills; add tests where an unrelated published marketplace skill does not enter the prompt for a different vertical/action.

### F-12 P2: code executor still advertises E2B while falling back to local or "not yet wired"

Source-of-truth: real-world execution requires computer-use and MCP for human-on-PC tasks (`docs/skycot/MISSION.md:80`), and security requires capability/RBAC and tenant-aware boundaries (`CLAUDE.md:46`, `CLAUDE.md:48`). For code execution specifically, `CLAUDE.md` also rejects unbounded/unsafe execution patterns by implication of the sandbox/security rules (`CLAUDE.md:59`).

Current state: `BackendKind::E2b` in the code executor says v1 returns a "not yet wired" error when selected and the HTTP client lands in a follow-up (`crates/argo-cli/src/handlers/worker_actions_executor_code.rs:112`, `crates/argo-cli/src/handlers/worker_actions_executor_code.rs:113`). `BackendKind::Local` is explicitly "bash -c in the parent OS namespace. No isolation" (`crates/argo-cli/src/handlers/worker_actions_executor_code.rs:117`).

Impact: any vertical or action relying on isolated code execution is either not actually isolated or not executable. This is not a blocker for the macOS-first computer-use path, but it is a production perimeter risk for enterprise deployments.

Remediation: keep local execution dev-only and fail closed in production profiles; wire E2B or another sandbox before advertising safe code execution; add policy checks that reject local backend when action risk/tenant policy forbids it.

### F-13 P2: v1 smoke grew broader, but it is not a complete proof of the five success scenarios

Source-of-truth: v1 requires all five MISSION success scenarios (`docs/skycot/V1_DEFINITION.md:41`) and `scripts/smoke-skycot.sh` passing (`docs/skycot/V1_DEFINITION.md:165`).

Current state: the smoke script now includes many mission checks (`scripts/smoke-skycot.sh:247`, `scripts/smoke-skycot.sh:275`, `scripts/smoke-skycot.sh:292`, `scripts/smoke-skycot.sh:306`). However, the macOS check can pass via the test's skip path (`crates/argo-executor-computer-use-macos/tests/notes_smoke.rs:24`), and the script itself is not active in a workflow because deploy smoke is disabled (`.github/workflows/deploy-smoke.yml.disabled:10`).

Missing proof points from the code walk:

- runtime memory tool namespace use through the agent dispatch loop, not just prompt hydration (`crates/argo-agent-runner/src/dispatch.rs:69`, `crates/argo-tool/src/tools/mod.rs:138`);
- fallback risk filtering (`crates/argo-cli/src/handlers/worker_actions_notify.rs:283`, `crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs:35`);
- policy-as-code with actual tenant/role/roster context (`crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:192`);
- per-worker learning cron scheduled by runtime boot (`crates/argo-skill/src/per_worker_learning_cron.rs:123`);
- real macOS action execution on a macOS machine, not `argo --help` (`crates/argo-cli/tests/cu_macos_dispatch_smoke.rs:31`).

Remediation: promote these proof points into the extended smoke gate. Treat skip-heavy tests as "compatibility checks," not mission success checks.

### F-14 P2: small unsafe shims lack the explicit marking expected by the invariant

Source-of-truth: CI should reject `unsafe` outside marked FFI shims (`CLAUDE.md:59`).

Current state: Unix signal lifecycle code uses `unsafe { libc::kill(...) }` (`crates/argo-cli/src/handlers/gateway/lifecycle.rs:16`) and self-update install uses a similar signal path (`crates/argo-cli/src/handlers/self_update_install.rs:174`). A different process-registry shim has an explicit `SAFETY:` comment (`crates/argo-tool/src/process_registry_tail.rs:94`), so the local pattern exists.

Impact: this is not the highest mission-risk item, but it shows the CI invariant is not consistently enforced.

Remediation: mark OS FFI shims with explicit `SAFETY:` comments and/or wrapper modules, then enforce the unsafe check in active CI.

## Coverage by mission claim

Claim 1, teammate not tool: identity, worker memory prompt hydration, worker portraits, roster and duty surfaces are present, but runtime memory tools are not namespace-bound (`crates/argo-tool/src/tools/mod.rs:138`) and policy resolution lacks tenant/role worker context (`crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:195`). The claim is partially wired, not production-complete.

Claim 2, delegation is the unit: duty scheduler, handoff, auctions, and worker lifecycle surfaces exist, but roster policy lookup before approval is still backlog work (`docs/build-plan/MISSION_BACKLOG.md:172`), and enterprise policy fields are not enforced (`crates/argo-identity/src/policy.rs:187`, `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:199`).

Claim 3, platform builds workers: Genesis has stronger eval seed tests (`crates/argo-genesis/tests/eval_seed_generation.rs:22`), but v1 docs still list pack/canvas/SDK distribution gaps (`docs/skycot/V1_DEFINITION.md:206`, `docs/skycot/V1_DEFINITION.md:215`). No new critical code blocker was found in this audit beyond CI/evidence gates.

Claim 4, skills are shared and learned: skill-library runtime load and Claim 4 E2E are shipped (`crates/argo-agent-runner/src/turn_helpers.rs:61`, `crates/argo-skill/tests/end_to_end_claim4_loop.rs:53`), but per-worker learning cron is not scheduled (`crates/argo-skill/src/per_worker_learning_cron.rs:123`) and library prompt selection is broad (`crates/argo-agent-runner/src/turn_helpers.rs:62`).

Claim 5, real-world execution on PC/Mac: MCP/API/browser/macOS paths are present, but v1 macOS proof is weak (`crates/argo-cli/tests/cu_macos_dispatch_smoke.rs:31`; `crates/argo-executor-computer-use-macos/tests/notes_smoke.rs:24`). Code execution sandboxing remains local-or-unwired (`crates/argo-cli/src/handlers/worker_actions_executor_code.rs:112`, `crates/argo-cli/src/handlers/worker_actions_executor_code.rs:117`).

Claim 6, multi-channel approvals: Slack/email/SMS/push/webinbox dispatch exists, but fallback risk filtering is broken (`crates/argo-cli/src/handlers/worker_actions_notify.rs:283`, `crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs:35`), several channel implementations are unreachable from the ask dispatcher (`crates/argo-cli/src/handlers/worker_actions_notify.rs:310`, `crates/argo-channels/src/voice.rs:154`), and policy-as-code fields are not enforced (`crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:199`).

## Phased execution roadmap

### Phase 0: restore the safety net and schema foundation

1. Re-enable active CI, deny, audit, and deploy-smoke workflows. Required findings: F-01.
2. Deduplicate and validate migrations before runtime SQL application. Required findings: F-02.
3. Enforce file/function budgets in local and GitHub CI. Required findings: F-09.
4. Turn duplicate-public-type lint from warning to enforced after a reviewed allowlist. Required findings: F-08.

Exit criteria: active CI rejects >500 LOC files, >80 LOC functions, duplicate migration slots, and duplicate public types outside allowlist; `scripts/smoke-skycot.sh` is a required gate on an appropriate runner matrix.

### Phase 1: close mission runtime loops

1. Bind runtime memory tools to tenant/worker/team namespaces and prove `query_scored` through real agent dispatch. Required findings: F-03.
2. Spawn per-worker learning cron from worker/gateway lifecycle. Required findings: F-04.
3. Enforce policy-as-code with tenant/role/roster context, quiet hours, audit-chain requirement, and N-of-M signatures or fail closed. Required findings: F-06.
4. Fix notification fallback risk filtering and channel reachability. Required findings: F-05 and F-07.

Exit criteria: a generated worker can remember privately, share team memory, learn a reusable skill from ratings, and request approval through the configured risk-appropriate channel with policy enforcement.

### Phase 2: prove v1 end-to-end on the actual demo path

1. Replace weak macOS dispatch smoke with real action execution through `computer_use_macos`. Required findings: F-10 and F-13.
2. Run extended smoke on a macOS runner or operator demo machine with `ARGO_NATIVE_MACOS_INTEGRATION=1`.
3. Add mission smoke assertions for policy-as-code, memory `query_scored`, fallback risk filtering, and per-worker learning.
4. Record the v1 mission demo after the smoke is green, matching `V1_DEFINITION.md` operator artifacts (`docs/skycot/V1_DEFINITION.md:153`, `docs/skycot/V1_DEFINITION.md:157`, `docs/skycot/V1_DEFINITION.md:158`).

Exit criteria: all five v1 success scenarios pass in one operator-visible flow, including Slack approval, macOS computer-use, shared memory handoff, auto-approval limits, and complete audit artifacts.

### Phase 3: harden enterprise deployment

1. Fail closed for local code execution in production profiles; wire a real sandbox backend. Required finding: F-12.
2. Complete channel dispatcher consistency for voice/iMessage/WhatsApp/Teams/Matrix or mark channels deferred in product docs. Required finding: F-07.
3. Finish type canonicalization for audit, provider, workflow, RBAC, and approval surfaces. Required finding: F-08.
4. Add explicit unsafe shim marking and enforcement. Required finding: F-14.

Exit criteria: enterprise deployment does not rely on paper-only policies, unreachable channels, local execution escape hatches, or parallel security/audit type contracts.

### Phase 4: scale the differentiated mission

1. Add relevance-aware skill marketplace selection and prompt-budget caps. Required finding: F-11.
2. Complete cross-tenant trajectory aggregation, RTBF propagation, and trust graph hardening from the backlog (`docs/build-plan/MISSION_BACKLOG.md:281` and `docs/skycot/V1_DEFINITION.md:217`).
3. Expand vertical proof beyond the v1 demo, following the plan's demo-vertical breadth gap (`docs/build-plan/BUILD_PLAN_V11.md:139`).
4. Continue the Build Plan v1.1-v2.2 order: production hardening, capability breadth, multi-worker orchestration, self-improvement, and scale (`docs/build-plan/BUILD_PLAN_V11.md:147`, `docs/build-plan/BUILD_PLAN_V11.md:159`, `docs/build-plan/BUILD_PLAN_V11.md:173`, `docs/build-plan/BUILD_PLAN_V11.md:336`).

Exit criteria: the platform can honestly claim workers are teammates, not a demo stack: persistent identity, memory, delegation, learned/shared skills, real PC/Mac execution, and multi-channel approval all work in production loops.
