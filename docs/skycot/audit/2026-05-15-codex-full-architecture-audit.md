# Codex full-architecture audit — 2026-05-15

**Run by**: codex. **Read-only.** Authored against origin/main `abceee8f` at `2026-05-15T11:05:41Z`.

> **NOTE 2026-05-16:** File restored into main repo from conductor session context after the original codex temp worktree was cleaned up. Content is the verbatim audit codex produced. The Claude validation pass + operator decisions are captured in `docs/build-plan/RESUME.md` "Right-now snapshot" section.

## Executive summary

- 18 findings total: P0=1, P1=6, P2=10, P3=1.
- Top 5 ship-blockers / near-blockers: F-001, F-002, F-003, F-004, F-005.
- Per-mission coverage: Claim 1 PARTIAL, Claim 2 PARTIAL with one P0, Claim 3 PARTIAL, Claim 4 PARTIAL, Claim 5 PARTIAL, Claim 6 PARTIAL.
- Cross-cutting headline: the Phase A.5 skill-library worker-load P0 is SHIPPED, and the migration-slot P0 is mitigated by the Rust registry; current high-risk drift is concentrated in tenant/auth boundaries (`x-tenant-id`, legacy websocket broadcast, nil tenant dispatch, default audit tenant), plus MCP-first/tool-stub drift.
- The current checkout exposes 87 crate manifests and 83 Rust-active crates under `crates/`; the prior Phase A baseline reported 88 crates / 520,814 LOC, while this checkout has 2,355 Rust files / 429,730 Rust LOC.
- Tests/builds were not run because the directive was read-only and Cargo would write `target/`; all verification below is source inspection plus read-only grep/scans.

## Methodology

I read the 11 requested source-of-truth docs first. Success criteria are anchored to the six mission claims in `docs/skycot/MISSION.md:26`, the v1 gate in `docs/skycot/V1_DEFINITION.md:5`, the hard invariants in `CLAUDE.md:42`, and drift-prevention rules in `docs/skycot/DRIFT_PREVENTION.md:16`.

I then ran the requested LOC scan, identified the top 50 Rust-active crates, inspected the mission runtime paths, and expanded repeated patterns across the workspace with read-only `rg`, `find`, `wc`, and small read-only scripts. The unused-public-API appendix is approximate: `cargo-machete` was not run because it may create build/cache artifacts.

Delta against 2026-05-14 audits:

- SHIPPED: the Phase A.5 skill-library worker-load blocker is now wired. The agent runner opens `ARGO_SKILL_LIBRARY_DB` in `crates/argo-agent-runner/src/turn_helpers.rs:104`, merges workspace/library lookup in `crates/argo-agent-runner/src/turn_helpers.rs:61`, filters non-marketplace library skills in `crates/argo-agent-runner/src/turn_helpers.rs:65`, and renders loaded skills into the prompt via `crates/argo-agent-runner/src/turn.rs:375`.
- SHIPPED/MITIGATED: migration slot collisions are no longer a production-runner P0 because the Rust registry validates registered versions in `crates/argo-store/src/migrations/slot_validator.rs:126`, but raw duplicate filenames remain preserved historical state in `crates/argo-store/src/migrations/slot_validator.rs:60`.
- STILL LIVE: duplicate type surfaces, file/function size drift, global mutable state, and MCP-first tool-surface drift remain live.

## Mission-claim sections

### Claim 1: One AI dev environment, polyglot and provider-agnostic

**Current state**: The turn loop loads a provider manifest, builds a transport, records spend context, and runs tools in `crates/argo-agent-runner/src/turn.rs:193` and `crates/argo-agent-runner/src/turn.rs:235`. Provider families include OpenAI-compatible, OpenAI Responses, Anthropic, and Bedrock via `crates/argo-agent-runner/src/transport.rs:20`. Tool dispatch uses a registry and capability engine in `crates/argo-agent-runner/src/dispatch.rs:71`.

**Required by MISSION.md**: the project must be a unified AI dev environment, not a single wrapper, with provider portability and capability/RBAC invariants (`docs/skycot/MISSION.md:26`, `CLAUDE.md:42`, `CLAUDE.md:47`, `CLAUDE.md:49`).

**Gaps found**:

- F-005 (P1): Audit rows can be stamped with `ws-default` even when the turn already knows the real session tenant. `AuditSink::new` defaults to `DEFAULT_AUDIT_TENANT_ID` in `crates/argo-audit/src/sink.rs:52` and `crates/argo-audit/src/sink.rs:121`; the turn records `session.tenant_id` in `crates/argo-agent-runner/src/turn.rs:173` but constructs `AuditSink::new` in `crates/argo-agent-runner/src/turn.rs:214`. Remediation: construct tenant-aware audit sinks at session/gateway edges, then deprecate `AuditSink::new` outside tests.
- F-006 (P1): The default `user` role still grants workspace-unscoped `FsRead` and `FsWrite` over `"**"` in `crates/argo-capability/src/lib.rs:285`. Remediation: interpolate a workspace-root glob before binding the role, and make the placeholder unconstructable in production.
- F-018 (P3): A stale provider-manifest comment says only `openai-compatible` is wired in `crates/argo-provider/src/manifest/mod.rs:97`, while runtime transport dispatch supports other families. Remediation: update the comment so future agents do not reimplement already-shipped transports.

**Coverage verdict**: PARTIAL. The core path exists, but tenant/audit and capability defaults are not enterprise-safe.

### Claim 2: Real work in real apps and SaaS

**Current state**: The manifest supports action executors including `mcp`, `computer_use`, `api`, and platform-native variants in `crates/argo-vertical/src/manifest_actions.rs:86`. CLI executor resolution maps `computer_use_macos` to a concrete executor in `crates/argo-cli/src/handlers/worker_actions_executor.rs:137` and `crates/argo-cli/src/handlers/worker_actions_executor.rs:272`.

**Required by MISSION.md**: both MCP SaaS tools and computer-use backends are required (`docs/skycot/MISSION.md:82`, `docs/skycot/MISSION.md:87`, `docs/skycot/MISSION.md:101`), and the locked v1 decision is macOS-first computer-use (`CLAUDE.md:103`).

**Gaps found**:

- F-001 (P0): Native macOS computer-use resolves but does not run the Anthropic computer-use loop. The adapter explicitly says the macOS crate has step primitives but no `run()` orchestrator in `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:9`, returns `ActionOutcome::HumanHandoff` after preflight in `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:91`, and writes instructions telling the operator to carry the action out manually in `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:157`. Remediation: port the shared CU run loop into the macOS executor or extract a platform-neutral loop trait, then return `Completed` on `stop_success`.
- F-008 (P2): The in-core tool surface drifted from the MCP-first invariant. `argo-tool` documents four schema-only stubs and Hermes-parity tools in `crates/argo-tool/src/tools/mod.rs:1`, returns 15 built-ins in `crates/argo-tool/src/tools/mod.rs:63`, and tests that exact 15-name surface in `crates/argo-tool/src/tools/mod.rs:120`; `CLAUDE.md:50` still described a smaller fixed core.
  - RESOLVED 2026-05-16: operator decision keeps the 15-tool core and updates `CLAUDE.md`; new tools still default to MCP unless they need direct AgentDeps/runtime access.
- F-017 (P2): Dynamic SQL identifier interpolation is present in 42 scanned instances. Most inspected cases use static allowlists, for example `crates/argo-store/src/tenant_query.rs:86` and `crates/argo-store/src/gdpr_apply.rs:98`, but the pattern is ad hoc. Remediation: introduce a typed `SqlIdent`/allowlist helper so new executor/connectors do not copy raw `format!("SELECT ... {table}")` patterns.

**Coverage verdict**: PARTIAL. MCP/API/browser executors are broad, but the macOS-first CU success path remains manual handoff.

### Claim 3: Hire and manage workers

**Current state**: Genesis has a charter-to-plan pipeline in `crates/argo-genesis/src/pipeline.rs:1`, writes vertical and worker config in `crates/argo-genesis/src/pipeline.rs:196`, and the duty scheduler fires worker duties into sessions in `crates/argo-orchestrator/src/duty_scheduler.rs:158` and `crates/argo-orchestrator/src/duty_scheduler.rs:218`.

**Required by MISSION.md**: workers, personas, roles, and verticals must not be conflated (`docs/skycot/MISSION.md:56`), and operators must be able to define roles/workers in plain English (`docs/skycot/MISSION.md:32`, `docs/skycot/V1_DEFINITION.md:41`).

**Gaps found**:

- F-010 (P2): Job-description ingestion is only an identity charter synthesis stub. The module describes JD/PDF/URL ingestion, then says charter synthesis is an identity-fn stub in `crates/argo-genesis/src/job_description.rs:31`; `synthesize_charter` returns raw text unchanged in `crates/argo-genesis/src/job_description.rs:171`. Remediation: wire the LLM classifier/summarizer into this path and add an e2e JD-to-worker smoke.
- F-011 (P2): Type canonicalization drift remains visible across worker/persona/role-adjacent surfaces. Current scan found 51 duplicate public type names, including `AuditRow`, `Principal`, `ComputerUseExecutor`, `ToolSchema`, `TrustGrade`, and `Workflow`; examples include `crates/argo-store/src/models.rs:153`, `crates/argo-cli/src/handlers/gateway/routes_worker_overview_kpis.rs:23`, `crates/argo-types/src/capability.rs:238`, and `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs:14`. Remediation: centralize canonical structs in `argo-types`/owning crates and replace local shadows.

**Coverage verdict**: PARTIAL. Worker identity and duty machinery exist; plain-English onboarding still has a stubbed JD synthesis branch and type drift.

### Claim 4: Workers learn skills and share them

**Current state**: The prior skill-library blocker is fixed: the turn path calls `turn_helpers::load_prompt_skills` in `crates/argo-agent-runner/src/turn.rs:375`, library lookup is layered in `crates/argo-agent-runner/src/turn_helpers.rs:61`, and non-marketplace library bodies are filtered in `crates/argo-agent-runner/src/turn_helpers.rs:65`. Cross-tenant skill sharing exists through `crates/argo-store/src/skill_share.rs:71`.

**Required by MISSION.md**: workers should learn skills from successful trajectories and share reusable skills across workers/customers (`docs/skycot/MISSION.md:36`, `docs/skycot/V1_DEFINITION.md:217`).

**Gaps found**:

- F-002 (P1): Marketplace install is mounted before bearer auth and trusts a spoofable `x-tenant-id` header as the principal. The route bypass is explicit in `crates/argo-cli/src/handlers/gateway/routes.rs:93` and `crates/argo-cli/src/handlers/gateway/marketplace.rs:6`; install derives authn from `x-tenant-id` in `crates/argo-cli/src/handlers/gateway/marketplace.rs:202`, checks only that target equals that same header in `crates/argo-cli/src/handlers/gateway/marketplace.rs:218`, then globally toggles a skill row via `set_shared` in `crates/argo-cli/src/handlers/gateway/marketplace.rs:243`. The store notes the `skill` table has no owning tenant in `crates/argo-store/src/skill_share.rs:22`, and `set_shared` updates by `(name, version)` only in `crates/argo-store/src/skill_share.rs:71`. Remediation: put install POST behind bearer/JWT auth, derive tenant from authenticated claims, add owner/provenance checks, and audit every install.

**Coverage verdict**: PARTIAL. Learning/prompt loading is materially better than the May 14 audit, but marketplace sharing has a real tenant/authz gap.

### Claim 5: Teams of workers collaborate

**Current state**: `delegate_task` emits orchestrator handoffs when the agent runner supplies an `OrchestratorStore` in `crates/argo-agent-runner/src/dispatch.rs:92`; the tool persists handoffs in `crates/argo-tool/src/tools/delegate_task.rs:184`.

**Required by MISSION.md**: workers should delegate, collaborate, and preserve context/escalation chains (`docs/skycot/MISSION.md:40`, `docs/skycot/V1_DEFINITION.md:229`).

**Gaps found**:

- F-004 (P1): The tool/MCP dispatch path still uses a nil tenant sentinel. `dispatch_tenant()` returns `TenantId(Uuid::nil())` in `crates/argo-agent-runner/src/dispatch.rs:21`, and MCP pool acquisition is keyed with that tenant in `crates/argo-agent-runner/src/dispatch.rs:57`. `delegate_task` repeats the nil tenant comment in `crates/argo-tool/src/tools/delegate_task.rs:22` and inserts the handoff under `dispatch_tenant()` in `crates/argo-tool/src/tools/delegate_task.rs:184`. Remediation: add `tenant_id` to `ToolContext`, plumb it from `session.tenant_id`, and key MCP/handoff state by the real tenant.
- F-009 (P2): Several public built-in tools still have default "not wired" paths. Examples: `memory` errors when no store is threaded in `crates/argo-tool/src/tools/memory.rs:206`, `delegate_task` errors without an orchestrator store in `crates/argo-tool/src/tools/delegate_task.rs:143`, `session_search` errors without a session store in `crates/argo-tool/src/tools/session_search.rs:139`, and `send_message` has an unwired dispatcher in `crates/argo-tool/src/tools/send_message.rs:92`. Remediation: split schema-only tools from runtime-registered tools, and make the agent runner the only surface that advertises tools requiring injected dependencies.

**Coverage verdict**: PARTIAL. Collaboration primitives exist, but tenant scoping and some runtime wires are incomplete.

### Claim 6: Approvals, channels, and inboxes

**Current state**: Approval asks notify channels in `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:268`; Slack, iMessage, push, and voice routes exist, including Slack signature verification in `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs:97`.

**Required by MISSION.md**: approval gates must work across web/iMessage/voice/Slack/push with auditability and safe authorization (`docs/skycot/MISSION.md:115`, `docs/skycot/V1_DEFINITION.md:241`).

**Gaps found**:

- F-003 (P1): Legacy web-inbox websocket auth intentionally has no tenant filtering. `WsAuth::Legacy` is documented as receiving every tenant's events in `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs:158`, falls back from JWT to bearer in `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs:184`, and accepts the legacy bearer path in `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs:203`. Remediation: require JWT for tenant-scoped websocket subscriptions, or bind the legacy bearer to one configured tenant and remove the global `"*"` broadcast path.
- F-007 (P1): `send_message` models high-risk outbound channels (`imessage`, `sms`) in `crates/argo-tool/src/tools/send_message.rs:16`, but `required_capabilities` returns no extra caps in `crates/argo-tool/src/tools/send_message.rs:206`; channel auth is deferred to config. Remediation: require channel-specific capabilities (for example `MessageSend{channel, recipient_scope}`) and route high-risk channels through approval policy before dispatch.

**Coverage verdict**: PARTIAL. Channel surfaces exist, but websocket tenant isolation and outbound-message RBAC need tightening.

## Cross-cutting concerns

### Security

- F-002: unauthenticated marketplace install trusts `x-tenant-id`; see `crates/argo-cli/src/handlers/gateway/marketplace.rs:202`.
- F-003: legacy websocket listeners can receive cross-tenant events; see `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs:160`.
- F-006: default user role has `"**"` filesystem access; see `crates/argo-capability/src/lib.rs:289`.
- F-007: high-risk outbound channels have no per-channel capability; see `crates/argo-tool/src/tools/send_message.rs:206`.
- No confirmed hardcoded secrets were found in the inspected top paths. SQL string formatting exists, but inspected production cases are static/allowlisted rather than direct user interpolation.

### Multi-tenancy

- F-002: `skill_share::set_shared` updates a skill row by name/version without tenant ownership.
- F-003: websocket legacy auth uses a global channel by design.
- F-004: tool/MCP/handoff dispatch uses nil tenant sentinels.
- F-005: audit events often use the default audit tenant even when a real session tenant exists.

### Observability

- Spend context is present in `crates/argo-agent-runner/src/turn.rs:235`.
- Audit chain stamping exists in `crates/argo-audit/src/sink.rs:215`, but F-005 means tenant dimensions can be wrong at write time.
- Global telemetry registries are process-wide statics; see F-015.

### Capability-RBAC

- F-006 and F-007 are the main RBAC gaps.
- Tool dispatch has a capability engine in `crates/argo-agent-runner/src/dispatch.rs:103`, but some tools rely only on generic `ToolDispatch` and do not declare risk-specific caps.

### MCP-first

- F-008 and F-009 show drift from the old fixed-count core-tool rule. This is the largest architectural drift from `CLAUDE.md:50`.

### Migration ordering

- F-016: production registry validation is in place, but raw historical duplicate filenames remain and raw-file tests still sort by numeric prefix in `crates/argo-store/tests/migration_chain.rs:126`.

## Pattern catalog

- **P-01: Tenant identity derived from unauthenticated headers or sentinels** - found in 6 files / 18 `x-tenant-id` hits plus 33 files / 51 nil-UUID hits. Examples: `crates/argo-cli/src/handlers/gateway/marketplace.rs:202`, `crates/argo-cli/src/handlers/gateway/rate_limit.rs:251`, `crates/argo-agent-runner/src/dispatch.rs:21`, `crates/argo-tool/src/tools/delegate_task.rs:22`. Expansion grep: `rg -n 'x-tenant-id|Uuid::nil\(\)|TenantId\(Uuid::nil\(\)' crates --type rust`. Recommended fix shape: authenticated `TenantId` on request/session/tool context; forbid tenant-bearing headers as principals.
- **P-02: Runtime-advertised tools with "not wired" defaults** - found in 20 files / 30 instances. Examples: `crates/argo-tool/src/tools/send_message.rs:104`, `crates/argo-tool/src/tools/memory.rs:206`, `crates/argo-tool/src/tools/delegate_task.rs:143`, `crates/argo-tool/src/tools/session_search.rs:139`. Expansion grep: `rg -n 'not wired' crates --type rust`. Recommended fix shape: separate schema registry from executable registry; only advertise executable tools in agent turns.
- **P-03: MCP-first drift / too many in-core tools** - 15 built-ins currently asserted. Examples: `crates/argo-tool/src/tools/mod.rs:63`, `crates/argo-tool/src/tools/mod.rs:120`. Expansion grep: `rg -n 'builtin_tools|vec!\[' crates/argo-tool/src/tools/mod.rs`. Recommended fix shape: update the invariant to the operator-approved 15-tool core and keep MCP as the default for new tools.
- **P-04: Duplicate public type names** - 51 duplicate names. Examples: `AuditRow` at `crates/argo-store/src/models.rs:153` and `crates/argo-cli/src/handlers/gateway/routes_worker_overview_kpis.rs:23`; `Principal` at `crates/argo-types/src/capability.rs:238` and `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs:14`; `ComputerUseExecutor` at `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs:78` and `crates/argo-executor-computer-use/src/executor.rs:41`. Expansion grep: `rg '^\s*pub .* (struct|enum|trait)' crates --type rust`. Recommended fix shape: type-canonicalization briefs per cluster. **CLAUDE VALIDATION 2026-05-15: count inflated — pattern catches pub(super)/pub(crate) shadows alongside truly-public; real cross-crate duplicates ~5-15.**
- **P-05: Size-budget drift** - 3 files over 500 LOC, 217 functions over 80 LOC, 19 impl blocks over 200 LOC. Examples: `crates/argo-audit/src/sink.rs:1`, `crates/argo-tui/src/input.rs:27`, `crates/argo-agent-runner/src/turn_helpers.rs:122`, `crates/argo-identity/src/store.rs:35`. Recommended fix shape: split by already visible helper boundaries; add CI hard gates.
- **P-06: Missing public API rustdoc/examples** - approximate scan found 4,260 missing docs among 6,893 public API items. Examples: `crates/argo-capability/src/lib.rs:35`, `crates/argo-skill/src/audit_reader.rs:29`, `crates/argo-skill/src/ab_test_stats.rs:32`, `crates/argo-mcp-connector-jira/src/client.rs:10`. Recommended fix shape: enforce doc lint for new/changed public APIs, then backfill crate by crate.
- **P-07: Global mutable state** - 60 files / 81 instances of `OnceLock`/`Mutex`/`RwLock` statics. Examples: `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs:138`, `crates/argo-cli/src/handlers/gateway/rate_limit.rs:183`, `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs:50`, `crates/argo-telemetry/src/prometheus_registry.rs:51`, `crates/argo-runtime/src/pool_elasticity.rs:303`. Recommended fix shape: move runtime state into gateway/app deps or narrowly document/test immutable caches.
- **P-08: Dynamic SQL identifier formatting** - 22 files / 42 broad hits, with 17 explicit `audit-allow` comments. Examples: `crates/argo-store/src/tenant_query.rs:86`, `crates/argo-store/src/gdpr.rs:234`, `crates/argo-store/src/gdpr_apply.rs:137`, `crates/argo-compliance/src/delete.rs:169`. Recommended fix shape: typed allowlisted identifiers and helper constructors.
- **P-09: Raw migration duplicate prefixes** - 11 historical slots / 27 files remain. Examples: slots 19, 22, 23, 24, 26, 27, 28, 30, 45, 51, 75 in `crates/argo-store/src/migrations/slot_validator.rs:60`. Recommended fix shape: keep registry canonical, but delete raw-prefix assumptions from tests/tools and add a preflight explaining the historical mapping. **OPERATOR DECISION 2026-05-15 Q4: rename to canonical (best practice).**

## Per-finding remediation table

| ID | Sev | Claim | File:line | Description | Remediation sketch | Est LOC | Brief-id proposal | Deps |
|---|---|---|---|---|---|---:|---|---|
| F-001 | P0 | 2 | `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:9` | macOS CU dispatch is bundle-and-handoff, not autonomous loop. | Extract/port CU run loop to macOS executor and return `Completed` on `stop_success`. | 350 (Claude bumped: 600-900, split into 2 briefs) | F-001a `codex-cu-platform-neutral-loop-trait` (MERGED #1719) + F-001b `codex-cu-macos-run-loop-wiring` (CLAIMED) | none |
| F-002 | P1 | 4 | `crates/argo-cli/src/handlers/gateway/marketplace.rs:202` | Marketplace install bypasses auth and trusts `x-tenant-id`; skill share has no owning tenant. | Gate POST by bearer/JWT, derive tenant from claims, add owner/provenance checks. | 220 (bumped from 180) | `codex-marketplace-install-authz` (MERGED #1721) | none |
| F-003 | P1 | 6 | `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs:158` | Legacy websocket can receive every tenant's events. | Require JWT or bind legacy bearer to one tenant; remove global broadcast. | 120 | `codex-webinbox-tenant-auth` (MERGED #1720) | none |
| F-004 | P1 | 5 | `crates/argo-agent-runner/src/dispatch.rs:21` | Tool/MCP/delegate dispatch uses nil tenant sentinel. | Add tenant to `ToolContext`; key MCP/handoff by real tenant. | 350 (additive-constructor pattern after 3 BLOCKs) | `codex-toolcontext-tenant` (TODO, 3rd-unblocked) | none |
| F-005 | P1 | 1 | `crates/argo-agent-runner/src/turn.rs:214` | AuditSink default tenant used where session tenant is known. | Prefer `new_with_tenant`; deprecate default outside tests. | DEFERRED — 106 callers (not 25); needs re-sizing into 2-3 sub-briefs | `codex-audit-tenant-plumb-*` | F-004 MERGED first |
| F-006 | P1 | 1/6 | `crates/argo-capability/src/lib.rs:285` | Default user role grants FsRead/FsWrite `"**"`. | Interpolate workspace-scoped globs or remove fs defaults. | 120 (bumped from 80) | `codex-rbac-workspace-scope` (CLAIMED) | none |
| F-007 | P1 | 6 | `crates/argo-tool/src/tools/send_message.rs:206` | High-risk outbound messaging has no channel-specific capability. | Add message/channel capabilities and approval integration. | 220 (bumped from 120 — needs new Capability::MessageSend variant) | `codex-send-message-channel-rbac` (CLAIMED) | F-009 |
| F-008 | P2 | 2 | `crates/argo-tool/src/tools/mod.rs:63` | 15 in-core tools outgrew the stale fixed-count MCP-first invariant. | **OPERATOR Q-decision 2026-05-15: update CLAUDE.md invariant, don't migrate. ~40 LOC doc edit. RESOLVED 2026-05-16 by invariant update; MCP remains default for new tools.** | 40 (was 250) | `codex-claude-md-tool-count-invariant-update` (TODO) | none |
| F-009 | P2 | 5 | `crates/argo-tool/src/tools/memory.rs:206` | Default built-ins advertise tools that error as "not wired". | **OPERATOR reframe 2026-05-15: WIRE them, don't hide. Build builtin_tools_with_runtime(deps).** | 280 | `codex-tool-runtime-wiring` (TODO) | F-008 OK to parallel |
| F-010 | P2 | 3 | `crates/argo-genesis/src/job_description.rs:171` | JD charter synthesis is identity stub. | Wire LLM classifier/summarizer and e2e JD onboarding test. | 180 | `codex-genesis-jd-synthesis` (unauthored) | none |
| F-011 | P2 | 3 | `crates/argo-store/src/models.rs:153` | 51 duplicate public type names remain. | **CLAUDE AMENDMENT 2026-05-15: count inflated to ~5-15 truly-public cross-crate; LOC 120 not 300.** | 120 (was 300) | `codex-type-canonicalization-wave-2` (unauthored) | none |
| F-012 | P2 | all | `crates/argo-audit/src/sink.rs:1` | 3 Rust files exceed 500 LOC. | Split files and add CI guard. | 120 | `codex-file-size-budget-split` (unauthored) | none |
| F-013 | P2 | all | `crates/argo-tui/src/input.rs:27` | 217 functions >80 LOC and 19 impls >200 LOC. | Refactor largest production functions first; enforce CI. | 500 | `codex-fn-impl-size-budget` (unauthored) | F-012 |
| F-014 | P2 | all | `crates/argo-capability/src/lib.rs:35` | Approx. 4,260 public API items lack rustdoc/examples. | Backfill by crate; lint new public APIs. | 600 | `codex-public-api-rustdoc-wave` (unauthored, Phase 4) | none |
| F-015 | P2 | all | `crates/argo-cli/src/handlers/gateway/rate_limit.rs:183` | 81 global mutable/static state instances. | Move runtime state into deps or document immutable caches. | 300 | `codex-global-state-audit` (unauthored) | none |
| F-016 | P2 | all | `crates/argo-store/src/migrations/slot_validator.rs:60` | Raw duplicate migration prefixes remain; raw test sorts by prefix. | **OPERATOR Q4 decision 2026-05-15: rename to canonical (best practice).** | 250 (bumped from 80) | `codex-migration-raw-filename-rename` (unauthored) | none |
| F-017 | P2 | all | `crates/argo-store/src/gdpr_apply.rs:137` | Dynamic SQL identifier formatting is repeated ad hoc. | Add typed allowlisted identifier helper. | 120 | `codex-sql-ident-helper` (unauthored) | none |
| F-018 | P3 | 1 | `crates/argo-provider/src/manifest/mod.rs:97` | Stale provider comment says other families are stubs. | Update comment. | 5 | `codex-provider-comment-refresh` (unauthored, Phase 4) | none |

## Phased execution roadmap

- **Phase 1 — ship-blockers** (target: v1 demo gate met): F-001. F-001a MERGED #1719; F-001b CLAIMED. Estimated 600-900 LOC total.
- **Phase 2 — security/data-integrity** (P1): F-002, F-003, F-004, F-005, F-006, F-007. F-002 + F-003 MERGED; F-004 + F-006 + F-007 in flight; F-005 deferred. Estimated ~1100 LOC.
- **Phase 3 — drift cleanup** (P2): F-008, F-009, F-010, F-011, F-012, F-013, F-015, F-016, F-017. F-008 + F-009 authored TODO; 7 unauthored. Estimated ~1700 LOC (post-amendments).
- **Phase 4 — polish** (P2/P3): F-014, F-018. Estimated 605 LOC.

Phases are sequential gates: Phase N+1 should begin after Phase N is merged. The exception is F-018, which can be bundled into any touched provider-doc cleanup.

## Appendix A — Top-50-LOC crates (sorted, with counts)

| Rank | Crate | Rust LOC | Rust files |
|---:|---|---:|---:|
| 1 | argo-cli | 203851 | 1106 |
| 2 | argo-store | 35638 | 204 |
| 3 | argo-skill | 18292 | 79 |
| 4 | argo-tool | 10188 | 45 |
| 5 | argo-tui | 9563 | 50 |
| 6 | argo-orchestrator | 9436 | 41 |
| 7 | argo-channels | 9265 | 41 |
| 8 | argo-provider | 9182 | 40 |
| 9 | argo-genesis | 8092 | 42 |
| 10 | argo-identity | 7753 | 37 |
| 11 | argo-executor-computer-use | 6958 | 32 |
| 12 | argo-runtime | 6426 | 29 |
| 13 | argo-vertical | 5834 | 26 |
| 14 | argo-agent-runner | 5769 | 30 |
| 15 | argo-config | 5146 | 44 |
| 16 | argo-audit | 4112 | 19 |
| 17 | argo-memory | 3457 | 18 |
| 18 | argo-executor-document | 3093 | 18 |
| 19 | argo-telemetry | 2680 | 17 |
| 20 | argo-executor-computer-use-windows | 2422 | 10 |
| 21 | argo-marketplace | 2388 | 10 |
| 22 | argo-eval | 2301 | 12 |
| 23 | argo-compliance | 2204 | 8 |
| 24 | argo-forge | 2178 | 12 |
| 25 | argo-types | 2151 | 12 |
| 26 | argo-mcp | 2059 | 13 |
| 27 | argo-workflow | 1942 | 9 |
| 28 | argo-executor-computer-use-macos | 1930 | 10 |
| 29 | argo-executor-computer-use-linux | 1635 | 16 |
| 30 | argo-executor-document-gsuite | 1552 | 6 |
| 31 | argo-judge | 1543 | 8 |
| 32 | argo-voice-tui | 1493 | 11 |
| 33 | argo-core | 1346 | 9 |
| 34 | argo-siem | 1289 | 10 |
| 35 | argo-cron | 1197 | 8 |
| 36 | argo-mcp-connector-onedrive | 1093 | 5 |
| 37 | argo-acp | 1092 | 7 |
| 38 | argo-executor-filesystem | 1076 | 6 |
| 39 | argo-redact | 1072 | 8 |
| 40 | argo-routing | 1050 | 9 |
| 41 | argo-executor-document-csv | 1004 | 6 |
| 42 | argo-plugin | 993 | 8 |
| 43 | argo-mcp-connector-clickup | 946 | 7 |
| 44 | argo-team-headline | 908 | 5 |
| 45 | argo-mcp-connector-gmail | 896 | 9 |
| 46 | argo-vertical-packs | 890 | 5 |
| 47 | argo-mcp-connector-github-issues | 857 | 7 |
| 48 | argo-skill-sdk | 850 | 4 |
| 49 | argo-mcp-connector-salesforce | 845 | 6 |
| 50 | argo-mcp-connector-pagerduty | 811 | 6 |

## Appendix B — Files >500 LOC (invariant violations)

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs` - 634 LOC.
- `crates/argo-tool/src/tools/memory_tests.rs` - 585 LOC.
- `crates/argo-audit/src/sink.rs` - 520 LOC.

Source invariant: `CLAUDE.md:54`.

## Appendix C — Functions >80 LOC (sample)

Approximate scan found 217 functions over 80 LOC. Largest samples:

- `crates/argo-cli/tests/gateway_smoke.rs:40` - `gateway_start_serves_health_and_version_then_exits_on_max_requests`, 441 LOC.
- `crates/argo-tui/src/input.rs:27` - `run`, 329 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs:136` - `dispatch_post_inner`, 329 LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs:73` - `start`, 322 LOC.
- `crates/argo-agent-runner/src/turn_helpers.rs:122` - `run_turn_with_tools`, 311 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_rollups.rs:14` - `dispatch_rollups`, 290 LOC.
- `crates/argo-cli/src/handlers/worker_actions.rs:35` - `run`, 289 LOC.
- `crates/argo-executor-computer-use/src/executor_step_helpers.rs:36` - `run_loop`, 193 LOC.

Approximate scan found 19 impl blocks over 200 LOC. Largest samples:

- `crates/argo-identity/src/store.rs:35` - 400 LOC.
- `crates/argo-skill/src/store.rs:48` - 374 LOC.
- `crates/argo-mcp-connector-onedrive/src/client.rs:34` - 333 LOC.
- `crates/argo-skill-sdk/src/builders.rs:139` - 304 LOC.
- `crates/argo-orchestrator/src/walker.rs:122` - 298 LOC.

Source invariants: `CLAUDE.md:55`, `CLAUDE.md:56`.

## Appendix D — Unused pub APIs (dead-code candidates)

`cargo-machete` was not run because this audit was read-only. These are grep-based candidates, not definitive dead-code findings:

- Prior Phase B orphan `FallbackChain::with_operator_preferred()` appears removed; `rg 'with_operator_preferred' crates --type rust` returned no hits.
- `crates/argo-runtime/src/planner_templates.rs:53` - `PlanTemplateCatalog::with_learned_templates` has no non-definition hit in the workspace scan; docs say this is the seam for Dreams output, so it is either a queued integration or dead seam.
- `crates/argo-routing/src/tiered.rs:70` - `TieredRouter::with_cache` has no non-definition hit in the scanned runtime crates; docs say it is for tests but no matching test use was found in the targeted grep.
- `crates/argo-provider/src/spend_gate.rs:151` - `with_anomaly_dispatch` is used by tests only (`crates/argo-provider/src/spend_gate_tests.rs:207`), so production anomaly dispatch wiring should be verified before relying on it.
- Duplicate public type clusters in F-011 are also dead-code candidates because local shadows often survive after canonical types ship.

## Open questions for operator — ALL ANSWERED 2026-05-15

- ~~Should the v1 demo gate require native macOS CU automation specifically, or may E2B `computer_use` satisfy Claim 2~~ — **Q1 answered: both paths stay; macOS is the v1 demo target. F-001 stays P0.**
- ~~Is the marketplace UI allowed to remain pre-bearer for GET-only browsing?~~ — **Q2 answered: POST behind auth, GET anonymous. F-002 SHIPPED #1721.**
- ~~Should `AuditSink::new` be made `#[cfg(test)]`/deprecated immediately~~ — **Q3 answered: rip the band-aid (cfg(test) immediately). F-005 needs re-sizing into 2-3 sub-briefs due to 106 callers.**
- ~~Do raw SQL migration files need to keep duplicate prefixes indefinitely~~ — **Q4 answered: rename to canonical (best practice). F-016 LOC bumped to ~250.**

Additional decisions:
- **F-008** (core-tool count): update CLAUDE.md to match reality, no MCP migration.
- **F-009** (unwired tools): WIRE them via `builtin_tools_with_runtime(deps)`, don't hide via registry split.
- **F-011** (type canon): re-scope to truly-public cross-crate `pub struct` duplicates only (~5-15, not 51).
