# Mission-vs-Code Gap Manifest

**Authored**: 2026-05-23 (evaluator round 8)
**Status**: 🔴 LIVING DOCUMENT — extend, never truncate.
**Purpose**: Single canonical map of "what MISSION.md says SKYCOT must do" vs "what code currently exists." Each gap row has a citation + priority + suggested brief shape the conductor turns into ledger work.

## How to extend this document

Every eval session that reads new code adds rows. The format:

```
| Crate or surface | Mission claim | Audited? | Known gap | Cite | Priority | Brief shape |
```

The conductor + auditor read this doc each tick and pick up unactioned rows. The aim is to drive every `🔴` / `🟡` row to `🟢` by feeding briefs through the swarm.

---

## The 6 load-bearing mission claims (verbatim from MISSION.md)

1. **Teammate, not tool.** Worker has persistent identity, voice, memory, role-membership over time.
2. **Delegation is the unit.** "Be my marketing intern" — duties, KPIs, charter. Not "run this one job."
3. **The platform builds workers.** Genesis turns plain-English role → vertical (system prompt, action vocabulary, memory schema, eval seeds).
4. **Skills are shared and learned.** Library + learning loop, cross-tenant sharing with provenance + trust.
5. **Any task a human can do on a PC.** TWO execution backends: MCP (well-known APIs) AND computer-use (Anthropic-style desktop control).
6. **Multi-channel approval + multi-tenant by construction.** Email, Slack, SMS, push, voice, web. Tenant isolation at every primitive.

---

## Crate map by mission claim

89 crates total in the workspace. Each gets bucketed below.

### Claim 1 — Teammate, not tool

| Crate | LOC | Audited? | Known gap | Cite | Priority | Brief shape |
|---|---|---|---|---|---|---|
| `argo-identity` | ~600 | 🟢 round 2 + 7 | None at substrate level. Channels + KPIs intentionally deferred to post-hire flows. | `store_hire.rs:151` `worker-downstream-state-after-hire.md` | — | — |
| `argo-memory` | ~3.3K | 🟢 AUDITED 2026-05-23 (eval-memory-per-worker-scope-honored) | Per-worker `MemoryNs` scope IS honored at SQL bind: `layer_rows` binds `(tenant_id, worker_id, memory_scope, layer)` (layers.rs:258-278); `append_in_namespace_with_layer` writes the same triple (layers.rs:110-128); `summary_namespaces` GROUP BY `(tenant_id, worker_id, memory_scope)` (summary.rs:176-211). 7 contract-pinning tests added (`crates/argo-memory/tests/per_worker_scope_test.rs`): same-tenant cross-worker, cross-tenant same-WorkerId, WorkerSelf-vs-SharedTeam, WorkerSelf-vs-Role, shared-team tenant-only isolation, summary per-worker grouping. Legacy `append`/`list_active`/`get`/`supersede`/`revert`/`set_lock` bind only `workspace_id` — pre-MemoryNs CLI-editor surface, not part of mission-claim-1 substrate. Findings report: `eval-memory-per-worker-scope-honored.md`. | MISSION.md status §"runtime closed loops"; `crates/argo-memory/src/layers.rs:258-278`; `crates/argo-memory/src/summary.rs:176-211` | 🟢 closed | — |
| `argo-team-headline` | ? | 🔴 NOT AUDITED | Crate exists, unknown surface. Worker overview page uses it per `routes_team_list.rs` import. | `routes_team_list.rs:31` | 🟢 P3 | `eval-team-headline-crate-surface` — read + assess. |

### Claim 2 — Delegation is the unit

| Crate | LOC | Audited? | Known gap | Cite | Priority | Brief shape |
|---|---|---|---|---|---|---|
| `argo-cron` + `argo-orchestrator::DutyScheduler` | ~800 + ~410 | 🔴 **WIRE BROKEN** (eval-cron-duty-fires-agent-runner-end-to-end, this round) | **Pre-flight finding 2026-05-23**: gateway boot actually wires `argo_orchestrator::DutyScheduler::tick` (not `argo_cron::Scheduler`) per `boot_spawns.rs:158`. `DutyScheduler::tick` creates a `Session` row + `duty_fired_session` attribution but **does NOT invoke `AgentRunner::run_turn`** — struct holds no AgentRunner handle. **NO `WorkerDutyFired` AuditKind exists** and `DutyScheduler` writes no audit row at all. 3 passing tests pin current behaviour (happy + no-match-negative + cross-tenant); 2 `#[ignore]`'d FINDINGS-ONLY regression hooks document the missing wires. Fix-brief required. | `crates/argo-orchestrator/src/duty_scheduler.rs:218`; `crates/argo-cli/src/handlers/gateway/boot_spawns.rs:158`; `crates/argo-agent-runner/tests/cron_duty_e2e_test.rs` (this round) | 🔴 P0 | **NEXT**: `fix-duty-scheduler-invokes-agent-runner` — add `AgentRunner` handle to `DutyScheduler`, invoke `run_turn` on auto-approved fires, write `WorkerDutyFired` audit row; flip the two `#[ignore]`'d tests to live. |
| `argo-judge` | ? | 🔴 NOT AUDITED | Eval seed scoring — runs evals at hire-time + post-execution. Used by skill creation (fail-open per #936) + forge promote (fail-closed per #940). | MISSION.md status §"runtime closed loops" | 🟢 P3 | `eval-argo-judge-crate-surface`. |
| KPI surface | ? | 🔴 GAP | Worker.kpis = None at hire. KPIs derived from duty-execution audit trail. **No code path I've found auto-derives the KPI rollup.** Mariah-card mockup shows live KPIs but those numbers need a populator. | `store_hire.rs:199-206` (kpis: None at hire) | 🟡 P2 | `mission-kpi-rollup-from-audit-trail` — populate `worker.kpis` JSON from a nightly-cron job that walks the worker's audit_record rows and aggregates by action_type / outcome. |

### Claim 3 — The platform builds workers (Genesis)

| Crate | LOC | Audited? | Known gap | Cite | Priority | Brief shape |
|---|---|---|---|---|---|---|
| `argo-genesis` | **8881** | 🟢 end-to-end smoke landed | 20+ pub modules wired; `plan + write_vertical` produces a valid vertical tree (vertical.toml + worker.toml + prompts/ + memory/ + eval/ + eval-seed.toml) that round-trips through `argo_vertical::load_dir`. `safety_judge::judge` rejects any sensitive-verb action without `requires_approval`; auto_fix promotes charter-derived sensitive actions before write. `genesis_judge` Allow/Block path exercised via mock client. **Finding** (`#[ignore]`'d FINDINGS-ONLY): customer-support archetype underflows the offline quality gate (1 eval seed vs ≥10; 157 prompt words vs ≥200); pipeline does not gate on `check_plan` today. | `crates/argo-genesis/tests/pipeline_end_to_end_test.rs` | 🟡 P1 | `eval-genesis-pipeline-end-to-end-smoke` — SHIPPED. |
| `argo-vertical` | ? | 🔴 NOT AUDITED | Loads `.workflow.toml` files. Authoritative source for v1 per `workflow_definition.rs:1-10` docstring. **Mission claim 3 says Genesis-generated verticals must work; need to verify Genesis output round-trips through argo-vertical's loader.** | `argo-vertical::parse_workflow` cited in `workflow_definition_write.rs:45` | 🟡 P2 | `eval-genesis-output-loads-cleanly-through-argo-vertical` — drive Genesis on 5 distinct role descriptions, write the trees to tempdirs, `argo_vertical::load_dir` each, assert no parse errors. |
| `argo-vertical-packs` | 934 | 🟡 AUDITED 2026-05-23 (eval-vertical-pack-registry-coverage) — wire broken | **Three parallel vertical-discovery surfaces with no operational bridge.** (a) `~/.argo/packs/` cache: `PackLoader::list` returns empty on fresh install — no bootstrap, no default catalog, no `argo packs search`. Only consumer is the `argo packs` CLI. (b) Repo `verticals/`: 36 subdirs, **32 loadable** via `argo_vertical::load_dir` (`vertical.toml` schema); 4 demo-only stubs (`ap-specialist`/`deal-desk`/`support-tier1`/`vendor-manager`) silently skipped. (c) Repo `packs/`: 10 packs in `worker.toml` schema, **0 consumed by any runtime path** — only `argo_skill::pack_lint` reads them. Pack ids and vertical ids overlap by name but schemas are not compatible (`worker.toml` ≠ `vertical.toml`). `packs/project-coordinator` (Peta) has no `verticals/project-coordinator/` counterpart. Default fetcher base_url is the **private** `github.com/mark-hallam/skycot-packs` repo. 2 LIVE pin tests + 2 `#[ignore]`'d FINDINGS-ONLY tests in `crates/argo-vertical-packs/tests/registry_coverage_pin_test.rs`. Report: `eval-vertical-pack-registry-coverage.md`. | `crates/argo-vertical-packs/src/loader.rs:147`; `crates/argo-vertical-packs/src/manifest.rs:147`; `crates/argo-cli/src/handlers/packs.rs:128`; `verticals/README.md:55-60` | 🟡 P2 audited — fix-briefs queued | **NEW P1**: `mission-pack-format-adapter-to-vertical-loader` (F-2 — bridge `worker.toml` packs into `argo_vertical::load_dir` so the marketplace cache becomes operationally visible). **NEW P2**: `mission-packs-marketplace-listing-endpoint` (F-1 — surface a pack index under `GET /api/v1/packs`). **NEW P3**: `verticals-demo-stubs-promote-or-archive` (F-3), `docs-verticals-readme-table-from-load-dir` (F-4 — README table stale by 28 entries), `packs/project-coordinator`-without-vertical (F-5, covered by F-2). |
| `argo-vertical-regression` | ? | 🔴 NOT AUDITED | Regression-test framework for vertical changes (drift gate). Per `feedback_brief_authoring_preflight_rules.md` the regression suite is consulted before vertical edits ship. | — | 🟢 P3 | `eval-vertical-regression-cadence` — verify the regression suite runs on every vertical edit + flags drift. |
| `argo-workflow` (runtime) | ? | 🟡 AUDITED 2026-05-23 — FINDINGS-ONLY (wire broken) | DAG / executor / checkpoint primitives. Per `argo-workflow/src/lib.rs` — the workflow runner. **`eval-workflow-execution-runs-real-step` shipped FINDINGS-ONLY: executor seam exists (`Executor<R: TaskRunner>`, `TaskRunner` trait) but no production `TaskRunner` impl in the workspace — every caller binds `MockRunner::echo()`. Zero refs to `tenant_id` / `audit_record` / `argo-mcp` in `crates/argo-workflow/`. Surface-existence smoke tests pin the abstract seam; 3 `#[ignore]`'d FINDINGS tests document the wire gaps to un-ignore once `WorkerSessionRunner` lands (referenced in `argo-cli/src/handlers/workflow.rs` header comment).** | `argo-workflow/src/executor.rs`, `argo-workflow/tests/execution_runs_real_step_test.rs` | 🟡 P1 | follow-up: `wire-worker-session-runner` — land `WorkerSessionRunner: TaskRunner` that calls `argo-mcp::McpClient::call_tool` + writes `audit_record`; add `principal: Principal` to `ExecutionRequest` for cross-tenant pre-flight; then un-ignore the 3 FINDINGS tests. |
| Workflow build canvas LOAD path | — | 🔴 GAP confirmed round 7 | Canvas reads `SEED_WORKFLOWS` static fixture not the gateway. | `page.tsx:115-130` | 🔴 P0 | `web-workflow-build-fetch-from-gateway` (round 7). |

### Claim 4 — Skills shared + learned

| Crate | LOC | Audited? | Known gap | Cite | Priority | Brief shape |
|---|---|---|---|---|---|---|
| `argo-skill` | ? | 🟡 round 2 — claim4 e2e test exists | Cross-tenant isolation verified via `claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak`. Marketplace, canary, A/B, dreams, learn modules present. Trust grade ratchet wires through `argo-trust-engine`. | `crates/argo-skill/tests/end_to_end_claim4_loop.rs:54` | — | — |
| `argo-skill-sdk` | ? | 🔴 NOT AUDITED | The crate operators use to AUTHOR custom skills. Unknown surface size + test coverage. | `Cargo.toml` | 🟢 P2 | `eval-argo-skill-sdk-author-flow` — write a 3-skill smoke against the SDK, verify lint + canary + audit. |
| `argo-trust-engine` | ? | 🔴 NOT AUDITED | Trust grade ratchet. Per #1979 trust-grade chip on skill detail. **Gap: which signals advance the grade? Does it actually fire when a worker completes successful executions without RBAC denies?** | `routes_worker_trust_grade.rs` | 🟡 P2 | `eval-trust-engine-ratchet-from-zero-to-grade-A` — simulate 10 successful tool_call executions → verify trust grade advances → simulate 1 RBAC deny → grade pauses. |
| `argo-workplace-knowledge` | ~200 | 🟡 round 3 — substrate exists, gateway just MERGED via #2062 | Round 3 finding (zero-caller drift) resolved. Editor SPA brief next. | round 3 + #2062 | — | `web-workplace-knowledge-editor` (queued, BLOCKED until gateway lands which it now has). |
| Skill rejection flow | — | 🔴 GAP | Mission says "human reviewer rejects → SkillRejected audit row + skill no longer offered". Need to verify the path exists. | journey-map j-skl-7 | 🟡 P2 | `eval-skill-rejection-flow` — submit a learned skill for review, reject it, assert SkillRejected audit + skill removed from `LayeredLookup::list_for_action` results. |

### Claim 5 — Any task a human can do on a PC (MCP + computer-use)

| Crate | LOC | Audited? | Known gap | Cite | Priority | Brief shape |
|---|---|---|---|---|---|---|
| `argo-mcp` (core) | ? | 🔴 NOT AUDITED | Per MISSION.md "currently 15 in-core tools". The MCP client pool is the substrate. Round 4 mentioned `McpConnectionPool` (parallel to `McpClientPool`) as a known cross-tenant credential leak that was fixed; need to verify no new parallel surface has appeared. | MISSION.md §"core platform invariants" #5 | 🟡 P2 | `eval-mcp-client-pool-tenant-keyed` — write a cargo test: pool keyed by tenant + caller-supplied creds; assert tenant_b cannot reuse tenant_a's MCP client. |
| 27 → **29** MCP connectors | ~21K (29 crates) | 🟢 ALL AXES PINNED 2026-05-23 — structural inventory PASS round-10; credential-scoping PASS via `eval-mcp-connector-credential-scoping-sweep`; real-vs-stub PASS via `eval-mcp-connector-real-vs-stub-inventory` (this round). | Slack, gcal, hubspot, jira, linear, pagerduty, notion, gmail, asana, airtable, salesforce, zendesk, discord, stripe, stripe-events, trello, github-issues, typeform, figma, google-meet, calendly, mailchimp, sendgrid, postmark, monday, fireflies, clickup, imessage, onedrive. Workspace count is **29 connectors + 1 shared helper** (round-10's "27" was a sales-page approximation). The 7 originally-flagged stubs (signal, telegram, mailgun, resend, apollo, zoominfo, zoom) have already been **deleted** from origin/main. **Real-vs-stub verdict (`eval-mcp-connector-real-vs-stub-inventory`, 2026-05-23):** 🟢 **29/29 REAL** — every shipped connector hits a real upstream API (reqwest + real production base URL + per-method dispatch) and ships at least one in-crate smoke test that does not hit real network (one-shot `TcpListener` pattern for 27, wiremock for `onedrive`, dedicated cache-sibling for `gcal`). **0 STUB-NEEDS-WORK, 0 STUB-OK.** The substrate-vs-stub gap is structurally green. **🟡 NEW P1 surfaced**: 23 of 29 REAL connectors have **no operator-facing surface** — `web/app/settings/connectors/page.tsx` does not exist, and only 6 ids (gmail, gcal, slack, stripe, linear, github-issues) appear in the hire-wizard catalog at `web/lib/connectors/registry.ts`. Fix-brief `web-settings-connectors-page-build` queued. **Credential-scoping verdict (`eval-mcp-connector-credential-scoping-sweep`, 2026-05-23):** 🔴 **3 × P0 cwd-cache leaks (gcal, gmail, salesforce) — FIXED via #2103 (`mission-mcp-connector-token-cache-tenant-keyed`).** 🟡 **28 × P1 env-only leaks (universal `cred::get_unscoped` pattern) — tracked by already-authored fix-brief `mission-cred-get-tenant-parameter`.** 🟡 **1 × P1 caller-responsibility surface (onedrive — constructor-arg bearer, no production caller yet).** A structural pin test `crates/argo-mcp/tests/connector_inventory_pin_test.rs` was added to fail if the connector set drifts from this audit. | `Cargo.toml` connectors list; `eval-mcp-connector-credential-scoping-sweep.md` + `eval-mcp-connector-real-vs-stub-inventory.md` (this round) | 🟢 all axes pinned | **🟡 NEW P1 (queued)**: `web-settings-connectors-page-build` — build the missing settings route, render rows for every connector advertised in a workspace-managed allowlist (or broaden `web/lib/connectors/registry.ts` to 29 ids). Resolves the 23-REAL-but-invisible inventory gap. **🟢 P2 follow-ups (queued)**: `eval-notion-add-smoke-tests` (2 → 7-ish tests to match peer baseline), `eval-slack-add-smoke-tests` (3 → 7-ish tests; lowest among operator-visible 6). **🟢 P3 (queued)**: `mission-pricing-tier-doc-author` — `docs/skycot/marketing/PRICING.md` does not exist; cell (e) of the real-vs-stub matrix is unanswerable without it. |
| `argo-executor-computer-use` (E2B) | ? | 🔴 NOT AUDITED | E2B Firecracker microVM. Per CLAUDE.md `v1.0 strategic decisions` — DEFERRED-v1.x. | — | 🟢 P3 (v1.x) | `eval-e2b-executor-substrate-status`. |
| `argo-executor-computer-use-macos` | ? | 🟢 round 4 — 37/37 lib tests pass | Smoke test (`notes_smoke.rs`) gated on `ARGO_NATIVE_MACOS_INTEGRATION=1`. End-to-end real-Notes run not yet performed. | round 4 report | 🟡 P2 | `mission-cu-macos-smoke-run-with-real-anthropic-loop` (operator-authorised one-time). |
| `argo-executor-computer-use-{linux,windows}` | ? | 🔴 NOT AUDITED | DEFERRED-v1.x per operator memory. | — | 🟢 P3 (v1.x) | — |
| `argo-executor-document(-csv,-gsuite)` | ? | 🔴 NOT AUDITED | Document executors (CSV reader, Google Sheets). | — | 🟡 P2 | `eval-document-executor-cross-tenant-scoping` — Sheets credentials per tenant; CSV file reads scoped to worker's `worker_fs_root`. |
| `argo-executor-filesystem` | ? | 🔴 NOT AUDITED | Per `worker_fs_root.sql` (migration 79) the executor scopes file access. | migration 79 | 🟡 P2 | `eval-filesystem-executor-fs-root-enforced` — assert a worker cannot read outside its `worker_fs_root`. |
| `argo-sandbox` | ? | 🔴 NOT AUDITED | E2B microVM substrate. | — | 🟢 P3 (v1.x) | — |
| `argo-tool` | ~2.5K | 🟢 AUDITED 2026-05-23 (eval-in-core-tool-inventory-vs-mission) | All 15 in-core tools justified (Direct AgentDeps / sandbox primitive / filesystem primitive / lean fallback). Every tool declares a per-call capability except `session_search`, which intentionally returns `CapabilitySet::empty()` and documents the registry's `ToolDispatch` glob as the sole gate. Every tool has ≥1 test. **"15 in-core tools" claim lives in CLAUDE.md invariant #5, NOT in MISSION.md** — gap-manifest cite shorthand corrected. New pin test at `crates/argo-tool/tests/in_core_tool_inventory_pin_test.rs` asserts both invariants (capability declared + ≥1 test) for all 15 tools. No SHOULD-MIGRATE-TO-MCP verdicts today; `web_search` + `web_extract` are lean fallbacks until a browser MCP connector ships (#642). | `CLAUDE.md:50-52`; `crates/argo-tool/src/tools/mod.rs:222-251`; `eval-in-core-tool-inventory-vs-mission.md` | 🟢 closed | — |
| `argo-routing` | ? | 🔴 NOT AUDITED | Per MISSION.md status: "ApiExecutor remains BLOCKED on schema-vs-acceptance mismatch (mission-wire-api-action-executor)". | MISSION.md §"runtime closed loops" | 🟡 P1 | `mission-wire-api-action-executor` (already a named brief per MISSION.md — verify status; may have shipped since the snapshot). |

### Claim 6 — Multi-channel approval + multi-tenant by construction

| Crate | LOC | Audited? | Known gap | Cite | Priority | Brief shape |
|---|---|---|---|---|---|---|
| `argo-channels` | 9178 | 🟡 partial (this round) | **13 channel impls present** — Discord, WebInbox, Email, Slack, iMessage, SMS, Teams, WhatsApp, Matrix, Voice, Push. Wired via `worker_actions_notify_*` handlers (chat, email, env, fallback, sse, audit). Inbound paths for email + Slack. **Gap: which channels are OPERATIONALLY CONFIGURED in v1.0?** Per `Pro` pricing tier card I saw on /pricing: "Multi-channel (Slack, iMessage, Email)" — so 3 of 13 are in the v1.0 sales promise. Others may be code-shipped but un-configured. | crates/argo-channels/src/*.rs | 🟡 P1 | `eval-channels-v1-config-vs-code-inventory` — for each of the 13 channels, document: (a) impl present? (b) wired to dispatch? (c) operator can configure via /settings/connectors? (d) Pro/Enterprise tier? Surface the matrix to operator decisional. |
| Channel inbound paths | — | 🔴 GAP | Mission claim 6 says "each channel impl handles BOTH outbound (request approval) AND (where applicable) inbound (receive the operator's reply)". I see `email_inbound.rs` + `slack_inbound.rs` only. The other 11 channels don't have an inbound path. | `argo-channels/src/email_inbound.rs`, `slack_inbound.rs` | 🟡 P1 | `eval-channels-inbound-coverage` — list which channels have inbound (approval-reply) wired vs only outbound. Push notifications + SMS + WhatsApp inbound are common gaps. |
| `argo-audit` | ? | 🟢 round 3/4 — cross-tenant fix verified | Hash-chain verified via `argo audit verify`. Audit-feed surfaces action records; platform events (WorkerHired etc.) not surfaced — round-1 P0-D still open. | round 1/3/4 | 🟡 P0-D | `web-audit-feed-platform-events` (still open). |
| `argo-compliance` | ? | 🔴 NOT AUDITED | HIPAA + GDPR. Per pricing page: "HIPAA workflows" claim. Need to verify the substrate. | /pricing claim | 🟡 P2 | `eval-compliance-claims-substrate` — for each compliance claim on /pricing (HIPAA workflows, GDPR Article 30, SOC 2 Type II audit trail), find the substrate that backs it. |
| `argo-redact` | ? | 🔴 NOT AUDITED | PII redaction for audit + export. Per `routes_workers_hire.rs:25` comment: "argo-redact::redact_text (J2) is the real fence once it lands; until then, basic regex covers the common case". **So redact is at v0.5; relies on inline regex in handlers.** | `routes_workers_hire.rs:25-27` | 🟡 P2 | `eval-redact-crate-promote-from-regex-to-real-fence` — verify `argo-redact::redact_text` is now wired (the comment said "once it lands"). |
| Tenant existence-leak in 6 worker handlers | — | 🔴 round 4 finding | Brief `mission-cross-tenant-worker-handler-no-existence-leak` authored, awaiting claim. | round 4 | 🟡 P2 | (brief authored). |
| Audit-feed platform-event gap | — | 🔴 round 1 finding | WorkerHired et al never surface on /audit. | round 1 | 🟡 P1 | `web-audit-feed-platform-events` (still open). |

---

## Cross-cutting (infrastructure / RBAC / observability)

| Crate | Mission claim served | Audited? | Known gap | Priority |
|---|---|---|---|---|
| `argo-types` | All | 🟢 by use | Canonical types. CI guards parallel-pub-types per operator memory `feedback_brief_authoring_preflight_rules.md`. | — |
| `argo-agent-runner` | All (worker runtime) | 🟢 AUDITED 2026-05-23 (eval-argo-agent-runner-turn-lifecycle) | **TWO VIEWS — both useful.** (1) Surface: `dispatch_one_with_tenant` (line 161) is the canonical entry with explicit tenant_id; `McpConnectionPool` keyed by `(tenant_id, name)`; capability engine checked BEFORE every MCP dispatch (dispatch.rs:206-217); 30+ files, 5918 LOC, decomposed cleanly. Strict `deny(unwrap_used, expect_used)`. (2) Turn-lifecycle deep-audit: 9 findings — 2 × P0 (skill table no tenant_id; skill_ab_test_run no tenant_id), 3 × P1 (audit-not-in-tx; pre-turn freeze missing audit; canary workspace-global), 4 × P2/P3. 4 FINDINGS-ONLY `#[ignore]`'d tests pin invariants. Report: `eval-argo-agent-runner-turn-lifecycle.md`. | 🟡 P1 — fix-briefs authored 2026-05-23: `mission-skill-table-add-tenant-id-column`, `mission-skill-ab-test-run-tenant-id-bind`. Runtime correct on load-bearing paths (memory namespace, MCP pool isolation, min_trust_grade SQL bind, skill propagation per turn). |
| `argo-runtime` | All | 🔴 NOT AUDITED | Relationship to argo-agent-runner unclear; may be deprecated or a wrapper. | 🟡 P2 — clarify. |
| `argo-config` | All | 🔴 NOT AUDITED | Workspace + tenant config. | 🟢 P3 |
| `argo-store` | All | 🟡 partial (round 5 fix-merge verified + round 13 subtask_ledger closed) | Migration framework now comment-aware (#2025). Workflow_definition primitive audited round 7. **`subtask_ledger` audited 2026-05-23 (eval-subtask-ledger-caller-tenant-audit) — 12 SQL statements, 7 production callers; 4 🟡 (CLI handlers, no remote surface) + 3 🟢 (server-resolved or natural-key) + 0 🔴. 5 contract-pinning tests shipped (1 runtime + 4 CLI). Same shape as audit_detail (#1992) but no remote surface to attack — verdict CLOSED; follow-ups deferred to fix-briefs listed in the report §5.** Other tables (worker, persona, role, duty, audit_record, skill, etc.) need similar tenant-scoping audits. | 🟡 P1 — `eval-argo-store-tenant-bind-sweep`. `subtask_ledger` row CLOSED — see `eval-subtask-ledger-caller-tenant-audit.md`. |
| `argo-capability` | All RBAC | 🟢 by use | Per-role + per-tenant capability engine. Used by routes_workflows + routes_workers + etc. | — |
| `argo-credentials` | Auth, channels, MCP | 🟢 round 9 — audited (eval-credentials-cross-tenant-isolation, 2026-05-23) | **TWO VIEWS — both useful.** (1) Crate-itself is small (623 LOC) + clean; store is FILE-BASED at `<workspace>/.argo/credentials.toml` (per-workspace, not per-tenant SQL); at-rest encryption via `age` + atomic 0600 writes. (2) **Three P0 findings at the SEAM** (not inside the crate): process-global `OnceLock<MasterKey>` at `routes_onboarding_helpers.rs:180` (data loss + cross-tenant blast); `argo_config::cred::get(name)` tenant-blind across 40+ callsites; OAuth caches at `<cwd>/.argo/connectors/<name>/access_token.json` shared. Plus 2 P1 + 2 P2 + 1 P3. ⚠ legacy plaintext entries still accepted on read (`EncryptedEntry::Plain` in `lib.rs`). See report: `docs/skycot/evals/scheduled/2026-05-23/eval-credentials-cross-tenant-isolation.md`. | 🟢 audited — fix-briefs authored 2026-05-23: `mission-credentials-per-tenant-master-key`, `mission-cred-get-tenant-parameter`, `mission-skill-table-add-tenant-id-column`, `mission-skill-ab-test-run-tenant-id-bind`. Plus 🟡 P2 `eval-credentials-force-migrate-on-read` for the legacy-accept path. |
| `argo-cli` | All (binary + gateway) | 🟡 spot-audited | Gateway routes module-by-module. Some submodules audited (workflows ×7, hire ×1, audit-export verified). Others not. | 🟡 P1 |
| `argo-tui` | UX | 🟢 N/A — not v1 | Per CLAUDE.md operator-locked: argo-cli monolith leave-alone for v1. | — |
| `argo-voice-tui` | UX | 🟢 N/A — not v1 | Same. | — |
| `argo-security` | All | 🔴 NOT AUDITED | Likely SIEM event emission. | 🟡 P2 |
| `argo-telemetry` | All | 🔴 NOT AUDITED | OTLP exporter (#938). | 🟡 P2 |
| `argo-siem` | All | 🔴 NOT AUDITED | SIEM dispatch (per migration 103). | 🟡 P2 |
| `argo-replay` | Mission claim 6 audit | 🔴 NOT AUDITED | Audit-replay surface. Cross-tenant fix in #1992. | 🟡 P2 — re-verify. |
| `argo-billing` | Operational | 🟢 by use | Stripe HMAC + checkout + portal. Per round-1 audit listed as tenant-agnostic by construction. | — |
| `argo-orchestrator` | Multi-worker | 🟢 deepened — all DutyStore methods self-defending (mission-orchestrator-duty-store-tenant-args, 2026-05-23) | **SQL is tenant-scoped at every WHERE clause** — `find_handoff`, `insert_handoff`, `list_pending_handoffs_for_worker`, `list_handoffs_for_session`, `update_handoff_status`, `cancel_handoff`, `RetryPolicyStore::get`/`list_for_skill`, `DutyStore::list_for_tenant`, `SlaStore::get`/`list`/`update`/`delete` all join on `tenant_id`. **🟢 Round-16 follow-on** (`mission-orchestrator-duty-store-tenant-args`): `DutyStore::get`/`mark_fired`/`set_paused`/`link_fire`/`sessions_for_duty` now also thread `tenant_id` (5 cross-tenant negative tests added — Surface 6 in `cross_tenant_handoff_test.rs`). Surface is self-defending; the round-3 audit-export risk class is closed at the API level, not just the call-site level. | 🟢 closed — all 5 DutyStore methods self-defending; future refactors can't add unsafe callers. |
| `argo-forge` | Worker A/B | 🔴 NOT AUDITED | Per MISSION.md status: "Judge is consulted by forge promote (fail-CLOSED — promotion blast-radius is larger)". | 🟡 P2 |
| `argo-eval` | Worker quality | 🔴 NOT AUDITED | Eval framework. | 🟡 P2 |
| `argo-marketplace` | Claim 4 distribution | 🔴 NOT AUDITED | OCI skill distribution. | 🟡 P2 |
| `argo-plugin` | Polyglot core | 🔴 NOT AUDITED | JSON-RPC plugin ABI per MISSION.md invariant #1. | 🟡 P1 — `eval-plugin-abi-stable`. |
| `argo-test-cassette` | Test infra | 🟢 by use | Test cassettes for record/replay. | — |
| `argo-acp` | Operator tooling | 🔴 NOT AUDITED | Agent Connection Protocol — connect SKYCOT to operator's coding agents (Claude Code, etc.). | 🟢 P3 |
| `argo-core` | All | 🔴 NOT AUDITED | Shared primitives. | 🟡 P2 |

---

## Master gap list (sorted by priority)

🟡 **NEW P2 — operator-locked 2026-05-23 vocabulary + UX (evaluator round 16):**

- **`web-rename-duty-to-schedule-in-ui-copy`** — every operator-facing string that says "Duty/Duties" in the SPA / hire wizard / settings / docs / marketing renames to "Schedule/Schedules." Keep `duty` in code, SQL, audit kinds, brief filenames, route paths (internal contracts). Cite: `project_vocabulary_duty_vs_schedule.md` operator memory. ~150 LOC + Playwright assertion that no SPA page contains "Duty/Duties" anymore.

- **`web-workflow-detail-add-as-schedule-button`** — on `/workflows/<v>/<n>` detail page, show a "+ Add as Schedule for <worker>" CTA. Picks the worker from the active cohort, opens a cron picker, POSTs to `/api/v1/workers/<id>/duties` (API path stays — internal contract). Closes the round-15 finding that the architecture already supports vertical-wide workflow access for a worker, but operators can't easily schedule a newly-added workflow. ~200 LOC + 1 Playwright spec.

---

🔴 **P0** — ship-blocker or active demo-blocker:

1. ✅ `eval-credentials-cross-tenant-isolation` — `argo-credentials` AUDITED 2026-05-23. 8 fix-briefs queued (3 × P0, 2 × P1, 2 × P2, 1 × P3). See report at `docs/skycot/evals/scheduled/2026-05-23/eval-credentials-cross-tenant-isolation.md`.
2. `eval-argo-agent-runner-turn-lifecycle` — THE runtime. Largest unaudited surface for mission claims 1-6.
3. (already authored) `web-workflow-build-layout-ts-rename` — round 5; in IN_REVIEW as #2066.
4. (already known) `web-workflow-build-fetch-from-gateway` — round 7; canvas reads static fixture.
5. (already known) `mission-workflow-audit-write-transactional` — round 7; audit write outside workflow tx.
6. (already known) `web-audit-feed-platform-events` — round 1; WorkerHired et al never surface.

🟡 **P1** — substantive gap blocking mission claim:

7. `eval-cron-duty-fires-agent-runner-end-to-end` — does the duty scheduler ACTUALLY drive the agent runner?
8. `eval-genesis-pipeline-end-to-end-smoke` — does Genesis produce a real vertical from a plain-English role?
9. `eval-workflow-execution-runs-real-step` — argo-workflow's executor + checkpoint.
10. `eval-mcp-connector-real-vs-stub-inventory` — 27 connectors, which are real?
11. ✅ `eval-in-core-tool-inventory-vs-mission` — 🟢 CLOSED 2026-05-23. All 15 tools justified; pin test added; CLAUDE.md (not MISSION.md) is the source of the "15 in-core" claim.
12. `mission-wire-api-action-executor` — per MISSION.md known-blocked status.
13. `eval-channels-v1-config-vs-code-inventory` — 13 channels in code, 3 in Pro pricing card; matrix the gap.
14. `eval-channels-inbound-coverage` — which channels have inbound (approval-reply) paths.
15. `eval-argo-store-tenant-bind-sweep` — round-7 verified workflow_definition; sweep the other 30+ tables.
16. `eval-argo-cli-gateway-module-sweep` — complete handler-by-handler RBAC + tenant audit.
17. `eval-plugin-abi-stable` — MISSION.md invariant #1 says polyglot core with stable JSON-RPC ABI.
18. ✅ `eval-memory-per-worker-scope-honored` — 🟢 CLOSED 2026-05-23. SQL bind layer honors `(tenant_id, worker_id, memory_scope)`; 7 contract-pinning tests in `crates/argo-memory/tests/per_worker_scope_test.rs`; no fix-brief needed.
19. `mission-kpi-rollup-from-audit-trail` — Mariah-card KPIs aren't auto-derived.

🟡 **P2** — quality / completeness:

20. `eval-trust-engine-ratchet-from-zero-to-grade-A`
21. `eval-skill-rejection-flow`
22. `eval-genesis-output-loads-cleanly-through-argo-vertical`
23. ✅ `eval-vertical-pack-registry-coverage` — `argo-vertical-packs` AUDITED 2026-05-23 (wire broken). 5 fix-briefs queued: `mission-pack-format-adapter-to-vertical-loader` (P1), `mission-packs-marketplace-listing-endpoint` (P2), `verticals-demo-stubs-promote-or-archive` (P3), `docs-verticals-readme-table-from-load-dir` (P3), F-5 covered by F-2. Report: `eval-vertical-pack-registry-coverage.md`.
24. ✅ `eval-orchestrator-cross-tenant-handoff-safety` — `argo-orchestrator` audited round 12 (eval-orchestrator-cross-tenant-handoff-negative-test, 2026-05-23). 13 cross-tenant negative tests pin the contract; 1 fix-brief queued (`mission-orchestrator-duty-store-tenant-args`).
25. `eval-document-executor-cross-tenant-scoping`
26. `eval-filesystem-executor-fs-root-enforced`
27. `eval-redact-crate-promote-from-regex-to-real-fence`
28. `eval-compliance-claims-substrate` (HIPAA / GDPR / SOC 2)
29. `eval-forge-promote-fail-closed-judge-wire`

(Lower priorities + the already-authored briefs are in their respective sections above.)

---

## Crates NOT YET TOUCHED by any eval session (next-session pickup)

In rough order of mission-leverage:

1. ~~`argo-agent-runner`~~ — 🟢 CLEARED round 9
2. ~~`argo-credentials`~~ — 🟢 CLEARED round 9 (eval-credentials-cross-tenant-isolation, 2026-05-23)
3. `argo-mcp` (core, not connectors) — 🟡 P1 — **now top of next-session list**
4. `argo-cron` (deep walk — partially started this round) — 🟡 P1
5. `argo-genesis` (deep walk through 8881 LOC) — 🟡 P1
6. `argo-workflow` (executor, checkpoint, DAG) — 🟡 P1
7. `argo-channels` (deep walk per channel) — 🟡 P1 (partial this round)
8. ~~`argo-orchestrator`~~ — 🟢 CLEARED round 12 (eval-orchestrator-cross-tenant-handoff-negative-test, 2026-05-23)
9. `argo-routing` — 🟡 P1 (for ApiExecutor blocker)
10. ~~`argo-tool`~~ — 🟢 CLEARED 2026-05-23 (eval-in-core-tool-inventory-vs-mission)
11. `argo-trust-engine` — 🟡 P2
12. `argo-vertical` — 🟡 P2; ~~`argo-vertical-packs`~~ — 🟡 AUDITED 2026-05-23 (eval-vertical-pack-registry-coverage) — wire broken, 5 fix-briefs queued
13. ~~`argo-memory`~~ — 🟢 CLEARED 2026-05-23 (eval-memory-per-worker-scope-honored)
14. `argo-skill-sdk` — 🟡 P2
15. `argo-replay` — 🟡 P2 (re-verify after #1992)
16. `argo-redact` — 🟡 P2
17. `argo-compliance` — 🟡 P2
18. `argo-store` (full table sweep, not just workflow_definition) — 🟡 P1
19. `argo-cli` (handler-by-handler) — 🟡 P1
20. `argo-plugin` — 🟡 P1
21. `argo-marketplace` — 🟡 P2
22. `argo-security` + `argo-telemetry` + `argo-siem` — 🟡 P2
23. The 27 → **29** MCP connectors — 🟢 ALL AXES CLEARED 2026-05-23. Credential-scoping (`eval-mcp-connector-credential-scoping-sweep`: 3 × P0 cwd-cache leaks FIXED via #2103, 28 × P1 env-only leaks tracked by `mission-cred-get-tenant-parameter`). Real-vs-stub (`eval-mcp-connector-real-vs-stub-inventory`: 29/29 REAL, 0 STUB-NEEDS-WORK, structural pin test added). 🟡 NEW P1 surfaced: `web-settings-connectors-page-build` (23 REAL connectors with no operator-facing surface).
24. `argo-executor-document(-csv,-gsuite)` + `argo-executor-filesystem` — 🟡 P2
25. `argo-team-headline` — 🟢 P3
26. `argo-acp` — 🟢 P3
27. `argo-runtime` (relationship to argo-agent-runner unclear) — 🟡 P2 clarification
28. `argo-eval`, `argo-forge`, `argo-judge` — 🟡 P2 each
29. `argo-vertical-regression` — 🟢 P3

Estimated next-session cadence: 4-6 crates per session × ~30 unaudited crates = **5-7 evaluator sessions to reach full mission-vs-code coverage**.

---

## Done-criteria

This manifest is "done" when:

1. Every crate row is 🟢 (audited within last 30 days) OR explicitly marked "deferred to v1.x" with operator sign-off.
2. Every gap with priority 🔴 P0 is closed (PR merged + evaluator re-verified).
3. Every gap with priority 🟡 P1 is closed OR explicitly deferred with a brief in `streams/`.
4. The "Master gap list" section has zero un-claimed P0/P1 rows.

---

## How the conductor reads this doc

Every conductor tick (cron, every 20 min):

1. Step 4.5b runs `scripts/check-unread-handoffs.sh` against `SHARED.md` and sees `[evaluator] round 8 — gap manifest authored`.
2. The conductor opens this file.
3. Picks the top un-actioned 🔴 / 🟡 gap row.
4. Authors a brief at `docs/build-plan/streams/<brief-id>.md` using the "brief shape" column as the seed.
5. Inserts the brief at the top of Phase A on the ledger.
6. A worker claims it and ships.
7. Next eval session re-reads the row, flips it 🟢, and adds new gaps that the closing PR surfaces.

This is the autonomous loop the operator described: evaluator extends → conductor brief-ifies → workers ship → evaluator verifies → loop.
