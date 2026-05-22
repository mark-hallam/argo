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
| `argo-memory` | ? | 🔴 NOT AUDITED | Per MISSION.md status: "per-worker memory scope (workspace-shared shipped, the schema extension is queued)". The `MemoryNs::WorkerSelf` scope exists on Worker but may not be honored by the memory store. | MISSION.md status §"runtime closed loops" | 🟡 P1 | `eval-memory-per-worker-scope-honored` — verify worker.memory_namespace constrains read/write at SQL bind layer + add cross-worker negative test. |
| `argo-team-headline` | ? | 🔴 NOT AUDITED | Crate exists, unknown surface. Worker overview page uses it per `routes_team_list.rs` import. | `routes_team_list.rs:31` | 🟢 P3 | `eval-team-headline-crate-surface` — read + assess. |

### Claim 2 — Delegation is the unit

| Crate | LOC | Audited? | Known gap | Cite | Priority | Brief shape |
|---|---|---|---|---|---|---|
| `argo-cron` | ~800 | 🟡 partial (this round) | **Duty scheduler ticks every minute under file-lock** per `lib.rs:14-21`. Confirmed wire at gateway boot in #939 per MISSION.md. ⚠ Gap: does the duty `entry_action` actually invoke the worker's agent runner? Need to trace `Scheduler::tick → JobRunner → AgentRunner`. | `crates/argo-cron/src/lib.rs:1-30` | 🟡 P1 | `eval-cron-duty-fires-agent-runner-end-to-end` — write a cargo integration test: hire worker with `default_enabled=true` duty cron `* * * * *` → tick → assert agent_runner invoked → audit row WorkerDutyFired written. |
| `argo-judge` | ? | 🔴 NOT AUDITED | Eval seed scoring — runs evals at hire-time + post-execution. Used by skill creation (fail-open per #936) + forge promote (fail-closed per #940). | MISSION.md status §"runtime closed loops" | 🟢 P3 | `eval-argo-judge-crate-surface`. |
| KPI surface | ? | 🔴 GAP | Worker.kpis = None at hire. KPIs derived from duty-execution audit trail. **No code path I've found auto-derives the KPI rollup.** Mariah-card mockup shows live KPIs but those numbers need a populator. | `store_hire.rs:199-206` (kpis: None at hire) | 🟡 P2 | `mission-kpi-rollup-from-audit-trail` — populate `worker.kpis` JSON from a nightly-cron job that walks the worker's audit_record rows and aggregates by action_type / outcome. |

### Claim 3 — The platform builds workers (Genesis)

| Crate | LOC | Audited? | Known gap | Cite | Priority | Brief shape |
|---|---|---|---|---|---|---|
| `argo-genesis` | **8881** | 🔴 NOT AUDITED at scale | 20+ pub modules: archetype, charter_dialogue, conflict, eval_seed, genesis_judge, job_description, manifest_writer, memory_schema, pipeline, prompt_engineer, prompt_writer, quality, refine, safety_judge, templates, worker_config. **Need verification each module is wired AND that the pipeline produces a valid vertical tree.** | `crates/argo-genesis/src/lib.rs:1-40` | 🟡 P1 | `eval-genesis-pipeline-end-to-end-smoke` — given a plain-English role string, assert Genesis produces a valid `vertical.toml` + `worker.toml` + `system_prompt.md` + `memory_schema.toml` + at least 1 eval seed; assert `safety_judge` rejects any sensitive action without `requires_approval`. |
| `argo-vertical` | ? | 🔴 NOT AUDITED | Loads `.workflow.toml` files. Authoritative source for v1 per `workflow_definition.rs:1-10` docstring. **Mission claim 3 says Genesis-generated verticals must work; need to verify Genesis output round-trips through argo-vertical's loader.** | `argo-vertical::parse_workflow` cited in `workflow_definition_write.rs:45` | 🟡 P2 | `eval-genesis-output-loads-cleanly-through-argo-vertical` — drive Genesis on 5 distinct role descriptions, write the trees to tempdirs, `argo_vertical::load_dir` each, assert no parse errors. |
| `argo-vertical-packs` | ? | 🔴 NOT AUDITED | Vertical marketplace registry per `project_skycot_packs_repo.md`. **Gap: which verticals ship today? Which are reachable from `/api/v1/verticals`?** | `routes_verticals.rs` | 🟢 P2 | `eval-vertical-pack-registry-coverage` — list installed verticals on a fresh `~/.argo/`, compare to `verticals/` repo subdir + the marketplace catalog, surface any drift. |
| `argo-vertical-regression` | ? | 🔴 NOT AUDITED | Regression-test framework for vertical changes (drift gate). Per `feedback_brief_authoring_preflight_rules.md` the regression suite is consulted before vertical edits ship. | — | 🟢 P3 | `eval-vertical-regression-cadence` — verify the regression suite runs on every vertical edit + flags drift. |
| `argo-workflow` (runtime) | ? | 🔴 NOT AUDITED | DAG / executor / checkpoint primitives. Per `argo-workflow/src/lib.rs` — the workflow runner. **Mission claim 3 needs the SPA build canvas → save → publish → execute loop end-to-end. Round 7 confirmed CRUD layer is solid; execution layer is unaudited.** | `argo-workflow/src/executor.rs` | 🟡 P1 | `eval-workflow-execution-runs-real-step` — given a saved workflow with a single `tool_call` step against a mock MCP server, executor invokes the tool, captures audit, writes checkpoint. |
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
| 27 MCP connectors | ? | 🔴 NOT AUDITED individually | Slack, gcal, hubspot, jira, linear, pagerduty, notion, gmail, asana, airtable, salesforce, zendesk, discord, stripe, stripe-events, trello, github-issues, typeform, figma, google-meet, calendly, mailchimp, sendgrid, postmark, monday, fireflies, clickup, imessage, onedrive. **Per operator memory: 4 consumer/redundant connector stubs (signal, telegram, mailgun, resend) are being deleted; 3 enterprise stubs (apollo, zoominfo, zoom) stay as DEFERRED-v1.x placeholders.** Need to enumerate which are MERGED-real vs stub. | `Cargo.toml` connectors list | 🟡 P1 | `eval-mcp-connector-real-vs-stub-inventory` — for each connector crate, check (a) does it have a real `connect()` that hits the upstream API, (b) is its `request()` keyed by per-tenant OAuth tokens, (c) is there at least one passing smoke test that doesn't hit real network? Tag each connector REAL / STUB-OK / STUB-NEEDS-WORK. |
| `argo-executor-computer-use` (E2B) | ? | 🔴 NOT AUDITED | E2B Firecracker microVM. Per CLAUDE.md `v1.0 strategic decisions` — DEFERRED-v1.x. | — | 🟢 P3 (v1.x) | `eval-e2b-executor-substrate-status`. |
| `argo-executor-computer-use-macos` | ? | 🟢 round 4 — 37/37 lib tests pass | Smoke test (`notes_smoke.rs`) gated on `ARGO_NATIVE_MACOS_INTEGRATION=1`. End-to-end real-Notes run not yet performed. | round 4 report | 🟡 P2 | `mission-cu-macos-smoke-run-with-real-anthropic-loop` (operator-authorised one-time). |
| `argo-executor-computer-use-{linux,windows}` | ? | 🔴 NOT AUDITED | DEFERRED-v1.x per operator memory. | — | 🟢 P3 (v1.x) | — |
| `argo-executor-document(-csv,-gsuite)` | ? | 🔴 NOT AUDITED | Document executors (CSV reader, Google Sheets). | — | 🟡 P2 | `eval-document-executor-cross-tenant-scoping` — Sheets credentials per tenant; CSV file reads scoped to worker's `worker_fs_root`. |
| `argo-executor-filesystem` | ? | 🔴 NOT AUDITED | Per `worker_fs_root.sql` (migration 79) the executor scopes file access. | migration 79 | 🟡 P2 | `eval-filesystem-executor-fs-root-enforced` — assert a worker cannot read outside its `worker_fs_root`. |
| `argo-sandbox` | ? | 🔴 NOT AUDITED | E2B microVM substrate. | — | 🟢 P3 (v1.x) | — |
| `argo-tool` | ? | 🔴 NOT AUDITED | Tool capability declarations. Per MISSION.md "new tools default to MCP unless they require direct AgentDeps". **Gap: which tools are in-core vs MCP today? The 15-in-core list per MISSION.md needs verification.** | MISSION.md core-platform invariant #5 | 🟡 P1 | `eval-in-core-tool-inventory-vs-mission` — enumerate every in-core tool, verify capability declaration, compare to the MISSION's 15-tools claim. |
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
| `argo-agent-runner` | All (worker runtime) | 🟢 round 9 — surface + dispatch.rs audited | **VERDICT: SOLID.** `dispatch_one_with_tenant` (line 161) is the canonical entry — explicit `tenant_id` parameter, caller cannot bypass. `McpConnectionPool.client(tenant_id, name, transport)` acquires from pool keyed by `(tenant_id, name)` — per-tenant isolation at the pool layer. **Capability engine is checked BEFORE every MCP tool dispatch** (`engine.check(principal, &Capability::ToolDispatch{...})` at dispatch.rs:206-217). `dispatch_tenant_isolation_tests::mcp_pool_isolates_same_server_by_tenant` exists as the negative test. 30+ files, 5918 LOC, decomposed cleanly (turn_budget, turn_canary, turn_context_policy, turn_helpers ×4, turn_memory, turn_planner, turn_routing, turn_run_loop, turn_skills ×4, turn_system_prompt, turn_vertical ×2). Strict `deny(clippy::unwrap_used, clippy::expect_used)` at lib.rs:38. Future-eval: walk the turn_run_loop + turn_planner deeper to verify subtask-budget tree behavior and tier-routing classifier. | 🟢 — major P0 cleared. |
| `argo-runtime` | All | 🔴 NOT AUDITED | Relationship to argo-agent-runner unclear; may be deprecated or a wrapper. | 🟡 P2 — clarify. |
| `argo-config` | All | 🔴 NOT AUDITED | Workspace + tenant config. | 🟢 P3 |
| `argo-store` | All | 🟡 partial (round 5 fix-merge verified) | Migration framework now comment-aware (#2025). Workflow_definition primitive audited round 7. **Other tables (worker, persona, role, duty, audit_record, skill, etc.) need similar tenant-scoping audits.** | 🟡 P1 — `eval-argo-store-tenant-bind-sweep`. |
| `argo-capability` | All RBAC | 🟢 by use | Per-role + per-tenant capability engine. Used by routes_workflows + routes_workers + etc. | — |
| `argo-credentials` | Auth, channels, MCP | 🟢 round 9 — audited | **VERDICT: BY-DESIGN WORKSPACE-SCOPED.** Store is FILE-BASED at `<workspace>/.argo/credentials.toml` — one credentials file per workspace (cwd), not a per-tenant SQL table. The "cross-tenant" concern I feared is a non-issue: tenant boundary IS the workspace location, enforced at the OS/process level. ✅ At-rest encryption via `age` with workspace master key. ✅ Atomic write with `0600` Unix permissions. ✅ Migration tool `migrate_plaintext_in_place` available. 🟡 **Caveat**: legacy plaintext entries are STILL accepted on read (`EncryptedEntry::Plain(s)` enum variant in `lib.rs`). For enterprise customers expecting forced encryption-at-rest, migration should be auto-run on first read, not opt-in. | 🟢 P0 cleared; 🟡 P2 quality follow-up `eval-credentials-force-migrate-on-read` for the legacy-accept path. |
| `argo-cli` | All (binary + gateway) | 🟡 spot-audited | Gateway routes module-by-module. Some submodules audited (workflows ×7, hire ×1, audit-export verified). Others not. | 🟡 P1 |
| `argo-tui` | UX | 🟢 N/A — not v1 | Per CLAUDE.md operator-locked: argo-cli monolith leave-alone for v1. | — |
| `argo-voice-tui` | UX | 🟢 N/A — not v1 | Same. | — |
| `argo-security` | All | 🔴 NOT AUDITED | Likely SIEM event emission. | 🟡 P2 |
| `argo-telemetry` | All | 🔴 NOT AUDITED | OTLP exporter (#938). | 🟡 P2 |
| `argo-siem` | All | 🔴 NOT AUDITED | SIEM dispatch (per migration 103). | 🟡 P2 |
| `argo-replay` | Mission claim 6 audit | 🔴 NOT AUDITED | Audit-replay surface. Cross-tenant fix in #1992. | 🟡 P2 — re-verify. |
| `argo-billing` | Operational | 🟢 by use | Stripe HMAC + checkout + portal. Per round-1 audit listed as tenant-agnostic by construction. | — |
| `argo-orchestrator` | Multi-worker | 🔴 NOT AUDITED | Inter-worker handoffs. Per coverage report: HIGH-risk for cross-tenant strategy disclosure. | 🟡 P2 |
| `argo-forge` | Worker A/B | 🔴 NOT AUDITED | Per MISSION.md status: "Judge is consulted by forge promote (fail-CLOSED — promotion blast-radius is larger)". | 🟡 P2 |
| `argo-eval` | Worker quality | 🔴 NOT AUDITED | Eval framework. | 🟡 P2 |
| `argo-marketplace` | Claim 4 distribution | 🔴 NOT AUDITED | OCI skill distribution. | 🟡 P2 |
| `argo-plugin` | Polyglot core | 🔴 NOT AUDITED | JSON-RPC plugin ABI per MISSION.md invariant #1. | 🟡 P1 — `eval-plugin-abi-stable`. |
| `argo-test-cassette` | Test infra | 🟢 by use | Test cassettes for record/replay. | — |
| `argo-acp` | Operator tooling | 🔴 NOT AUDITED | Agent Connection Protocol — connect SKYCOT to operator's coding agents (Claude Code, etc.). | 🟢 P3 |
| `argo-core` | All | 🔴 NOT AUDITED | Shared primitives. | 🟡 P2 |

---

## Master gap list (sorted by priority)

🔴 **P0** — ship-blocker or active demo-blocker:

1. `eval-credentials-cross-tenant-isolation` — `argo-credentials` stores OAuth + API keys, never audited.
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
11. `eval-in-core-tool-inventory-vs-mission` — MISSION.md claims 15 in-core tools; verify.
12. `mission-wire-api-action-executor` — per MISSION.md known-blocked status.
13. `eval-channels-v1-config-vs-code-inventory` — 13 channels in code, 3 in Pro pricing card; matrix the gap.
14. `eval-channels-inbound-coverage` — which channels have inbound (approval-reply) paths.
15. `eval-argo-store-tenant-bind-sweep` — round-7 verified workflow_definition; sweep the other 30+ tables.
16. `eval-argo-cli-gateway-module-sweep` — complete handler-by-handler RBAC + tenant audit.
17. `eval-plugin-abi-stable` — MISSION.md invariant #1 says polyglot core with stable JSON-RPC ABI.
18. `eval-memory-per-worker-scope-honored` — per MISSION.md status, schema extension queued.
19. `mission-kpi-rollup-from-audit-trail` — Mariah-card KPIs aren't auto-derived.

🟡 **P2** — quality / completeness:

20. `eval-trust-engine-ratchet-from-zero-to-grade-A`
21. `eval-skill-rejection-flow`
22. `eval-genesis-output-loads-cleanly-through-argo-vertical`
23. `eval-vertical-pack-registry-coverage`
24. `eval-orchestrator-cross-tenant-handoff-safety`
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
2. ~~`argo-credentials`~~ — 🟢 CLEARED round 9
3. `argo-mcp` (core, not connectors) — 🟡 P1 — **now top of next-session list**
4. `argo-cron` (deep walk — partially started this round) — 🟡 P1
5. `argo-genesis` (deep walk through 8881 LOC) — 🟡 P1
6. `argo-workflow` (executor, checkpoint, DAG) — 🟡 P1
7. `argo-channels` (deep walk per channel) — 🟡 P1 (partial this round)
8. `argo-orchestrator` — 🟡 P2
9. `argo-routing` — 🟡 P1 (for ApiExecutor blocker)
10. `argo-tool` — 🟡 P1
11. `argo-trust-engine` — 🟡 P2
12. `argo-vertical` + `argo-vertical-packs` — 🟡 P2
13. `argo-memory` — 🟡 P1
14. `argo-skill-sdk` — 🟡 P2
15. `argo-replay` — 🟡 P2 (re-verify after #1992)
16. `argo-redact` — 🟡 P2
17. `argo-compliance` — 🟡 P2
18. `argo-store` (full table sweep, not just workflow_definition) — 🟡 P1
19. `argo-cli` (handler-by-handler) — 🟡 P1
20. `argo-plugin` — 🟡 P1
21. `argo-marketplace` — 🟡 P2
22. `argo-security` + `argo-telemetry` + `argo-siem` — 🟡 P2
23. The 27 MCP connectors — 🟡 P1 inventory + spot-check
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
