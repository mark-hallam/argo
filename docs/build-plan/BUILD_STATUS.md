# Build status — live ledger

> **CONDUCTOR HARD RULE (load-bearing, do not drift):** check queue
> depth at the START of every tick. If `codex` TODO count ≤ 8 OR
> `claude` TODO count ≤ 8, fire queue replenishment IMMEDIATELY
> (Step 8 of PROTOCOL_LOOP_PROMPT.md, brainstorm 3-5 new briefs from
> BUILD_PLAN_V11.md / MISSION.md gaps / PR-body deferrals / git log)
> BEFORE touching merges. Workers stand down if queue runs dry.
> Target depth after replenishment: 12-16 total briefs, weighted
> ~50/50 codex/claude. **Skipping this rule has happened twice this
> session and is not acceptable.**

**Updated by every agent on every state transition.** Always
read from origin/main; never trust a stale local copy.

Status values: `TODO` (claim me) · `CLAIMED` (someone is
working) · `IN_REVIEW` (PR open) · `MERGED` (on main) ·
`BLOCKED` (needs unblock).

When you claim or update a row, commit BUILD_STATUS.md with
message `status: <action> <brief-id>` (e.g. `status: claim
C-gmail`, `status: C-gmail in review (#812)`).

---

## Active queue

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| **claude-rebase-skill-auction-pr900** | claude | MERGED | #900 | recovery complete: PR #900 rebased onto current main (sha `0e30e4e`); 5 conflict files resolved within may-touch (argo-orchestrator lib.rs/Cargo.toml, argo-cli cli.rs/handlers/mod.rs/main.rs); cargo build + clippy --workspace --all-targets -D warnings + cargo test -p argo-orchestrator (81 unit + 2 integration + 7 doc) + handoff_auctions_smoke (2 integration) all clean; PR now MERGEABLE/CLEAN. Conductor handles the actual merge. |
| 0-ci-cleanup | (in landing PR) | TODO | — | (not yet started) |
| A-executor-trait | claude | MERGED | #784 | Executor trait + manifest enum + 4 impl shape; B/C/G unblocked |
| B-computer-use-executor | claude | MERGED | #787 | CU executor + E2B sandbox + Anthropic CU loop + marketplace_browser profile |
| C-mcp-wire | codex | MERGED | #788 | dispatch_one routes MCP tools via per-server connection pool; manifest [mcp_servers] section + integration test landed; 5 connector briefs unblocked |
| C-gmail | codex | MERGED | #795 | search / read / draft / send / label + OAuth refresh + access-token cache (`<cwd>/.argo/connectors/gmail/`); ALL 5 connectors landed |
| C-slack | codex | MERGED | #793 | post / list / read / dm / search via SLACK_BOT_TOKEN; integration test gated |
| C-gcal | codex | MERGED | #791 | list / create / update / suggest_time / respond + OAuth refresh + access-token cache |
| C-hubspot | codex | MERGED | #792 | contacts / deals / activity / pipeline + private-app token |
| C-notion | codex | MERGED | #794 | search / read_page / create_page / append_block / update_database via NOTION_INTEGRATION_TOKEN |
| D-email | codex | MERGED | #780 | NotifyChannel trait + EmailChannel landed; predecessors unblocked |
| D-slack-approvals | codex | MERGED | #783 | Block Kit buttons + HMAC-signed webhook |
| D-sms | codex | MERGED | #782 | Twilio SMS approval channel + HMAC token decision route |
| D-push | codex | MERGED | #798 | PushApprovalChannel via Expo (APNs/FCM); device registration + deep-link decision route; integration test gated on EXPO_TEST_TOKEN |
| D-webinbox | codex | MERGED | (in 7c9f156) | webinbox files landed in tick 5 via commit 7c9f156 (status update inadvertently swept up codex's WIP); #789 closed as redundant — content identical to main; full inbox HTML + WS push + manifest live |
| E-identity | codex | MERGED | #781 | Worker/Persona/Role primitives + sqlite migration landed |
| F-memory-layering | codex | MERGED | #786 | 3-layer memory + sqlite-vec/Voyage-3 landed; H now closer to ready |
| G-genesis | claude | MERGED | #790 | charter → vertical generator + safety judge + 12-archetype classifier landed; deterministic offline pipeline (LLM refinement is v1.x) |
| H-auto-approval | claude | MERGED | #796 #799 #800 #801 #803 #805 | ALL 6 sub-slices landed: H-1 #796 (policy + budget) + H-2 #800 (gate hookup) + H-3 #799 (orchestrator) + H-4a #801 (trust ladder) + H-4b #803 (two-tier lookup) + H-4c #805 (skill-learning primitives) |
| F-memory-shared-team | codex | MERGED | #819 | `MemoryNs::shared_team(tenant)` + author-stamped writes; cross-tenant isolation; new shared_team.rs (232 LOC) + migration 15 |
| H-orchestrator-persist | codex | MERGED | #806 | sqlite persistence for Session + Handoff types; OrchestratorStore CRUD + migration 16; lifecycle invariants enforced; 6 unit tests + cross-tenant isolation |
| H-spend-persistence | codex | MERGED | #809 | spend_today_for_worker + record_charge helpers; gate reads spend before evaluation + writes charge after AutoApprove; reuses H-1's worker_budget schema; 4 tests cover over-cap / at-cap / cross-worker / cross-day |
| H-orchestrator-cli | codex | MERGED | #824 | `argo session start/list/show` + `argo handoff list/show`; --tenant required + --json |
| H-orchestrator-routes | codex | MERGED | #827 | gateway HTTP endpoints for sessions + handoffs; routes_orchestrator.rs (411 LOC) |
| H-skill-library-routes | codex | MERGED | #817 | gateway HTTP endpoints `GET /skills?risk=...` + `GET /skills/<name>` exposing the H-4b LayeredLookup; workspace overrides library on duplicates; 400 on invalid risk |
| H-budget-cli | codex | MERGED | #820 | `argo budget show <worker_id>` + `argo budget recent` CLI; spend_today_for_worker + spend_this_month rollups + remaining-headroom calc; --json on every list/show |
| H-doctor-extensions | codex | MERGED | #808 | 4 new `argo doctor` checks for the H-stack: workspace policy presence + bare-enabled WARN, worker_budget table reachable, session/handoff tables (MISSING-friendly), notify-channel manifests; 9 unit tests + JSON envelope extension |
| H-metrics | codex | DEFERRED-v1.x | — | DEFERRED to v1.x: the existing `/metrics` handler manually constructs output rather than reading from Prometheus's `default_registry()`, so the sibling-file pattern can't expose new counters without refactoring `metrics.rs` (already 545 LOC; refactor exceeds the brief's scope). Operator-value of Prometheus counters for v1 demo is low; deferring. v1.x will refactor metrics.rs to gather from default_registry() and re-enable this brief. Codex: do NOT claim. |
| migration-consolidation-test | codex | MERGED | #811 | integration test verifying full migration chain applies cleanly on fresh DB; H-stack tables checked with skip-when-missing fallback |
| demo-marketplace-seller-vertical | codex | MERGED | #818 | full demo vertical: vertical.toml + persona/system prompts + memory schema + eval seeds + 5-item skate inventory.json + workspace auto_approval.toml; ready for v1 demo |
| H-skill-review-cli | codex | MERGED | #823 | `argo skill review list/show/approve/reject`; --vertical filter + --json |
| H-action-replay | codex | MERGED | #826 | `argo action replay <id>` debug replay landed; human dry-run/bundle flow + computer_use hook; tests cover replay bundle + dry-run |
| H-audit-query-extensions | codex | MERGED | #833 | `argo audit query` filters --decision/--worker/--session/--escalation for H-stack debugging |
| H-channel-health-checks | codex | MERGED | #828 | NotifyChannel connectivity pings (Postmark/Slack/Twilio/Expo); doctor_channels.rs (315 LOC) + 5s timeout per check |
| H-shared-team-cli | codex | MERGED | #841 | `argo memory shared-team list/write/grep` for inspecting tenant-shared memory namespace |
| H-vertical-list-cli | codex | MERGED | #834 | `argo vertical list/show/describe` for installed verticals |
| H-worker-list-cli | codex | MERGED | #837 | `argo worker list/show + policy show` for worker discovery + per-worker policy resolution |
| H-genesis-list-templates | codex | MERGED | #830 | `argo genesis list-archetypes/show-archetype/suggest` exposing the 12 archetypes via CLI |
| claude-cu-skill-learn-wire | claude | MERGED | #807 | CU success → maybe_propose → SkillCandidate via H-4c → SkillStore::propose_for_review; new bridge module + 10 unit tests; learning-loop errors swallow + log so the action never fails |
| claude-tui-h-stack-panels | claude | MERGED | #822 | 3 ratatui overlays (O sessions / H handoffs / B budget) reading via gateway HTTP; renders "Gateway unreachable" cleanly while H-orchestrator-routes pending; 14 new tests (9 snapshot + 5 input) |
| claude-e2e-integration-test | claude | MERGED | #810 + #832 | 🎯 V1 DEMO GATE GREEN (9/9): full Genesis → V3 ledger → auto-approval gate → channel notify → executor → audit chain; #810 shipped 6/9 steps + #832 completed the remaining 3 |
| claude-genesis-llm-refinement | claude | MERGED | #812 | opt-in --refine flag with Anthropic call + JSON schema validation + safety judge re-runs on refined actions; 9 unit tests offline |
| claude-skill-success-metric | claude | MERGED | #813 | SuccessMetric (confidence + trajectory_length + duration + final_state_score) + ProposalThreshold defaults (≥0.8 / ≤25 steps / ≤600s); CU executor computes from real trajectory; legacy f64 API preserved |
| claude-worker-instance-policy | claude | MERGED | #815 | Worker::resolved_policy(workspace_default) + argo worker update --auto-approval-json; gate hookup deferred (waits on claude-action-record-identity-plumbing for worker_id on ActionRecord) |
| claude-orchestrator-handoff-walker | claude | MERGED | #825 | HandoffWalker::sweep_once + tokio sweeper at gateway boot; SchemeRoutingNotifier; 12 tests; makes orchestrator genuinely autonomous |
| claude-skycot-runbook | claude | MERGED | #816 | 944-line operator runbook (`docs/skycot/RUNBOOK.md`) covering all 10 brief sections: install → bootstrap → genesis → approvals → first action → skill learning → handoff → audit → next steps |
| claude-action-record-identity-plumbing | claude | MERGED | #829 | ActionRecord.worker_id threaded through 19 files; emit_actions_from_text gains worker_id param; FOUNDATIONAL — unblocks e2e-test-step-completion + worker-instance-policy gate hookup |
| claude-e2e-test-step-completion | claude | MERGED | #832 | flipped 3 SKIPped steps to real assertions: worker_action JSON round-trip / StubNotifier capture / pending→executing→completed memory-row chain; **v1 demo gate now GREEN (9/9)** |
| claude-marketplace-vertical-publish | claude | MERGED | #821 | `argo vertical publish/pull` via OCI; build_vertical_bundle + unpack_vertical_bundle; hermetic round-trip + tamper-reject tests; --rename for conflict resolution |
| claude-realtime-audit-stream | claude | MERGED | #831 | AuditSink::subscribe() broadcast channel + GET /audit/stream snapshot route with kind/worker filters + bearer auth; 13 tests; heartbeat deferred to v1.x WS transport |
| H-eval-runner-cli | codex | MERGED | #840 | `argo eval run --vertical <id>` runs eval seed against worker pipeline; unblocks claude-eval-streaming |
| H-tenant-cli | codex | MERGED | #839 | `argo tenant list/info/create` for tenant management |
| H-handoff-cancel | codex | MERGED | #843 | `argo handoff cancel <id> [--note]` + HandoffStatus::Cancelled variant + cancel() guard (rejects Settled/Expired); audit row 'handoff_cancelled' on cancel; exhaustive-match arms updated per Tick 45 amendment |
| H-skill-export-import | codex | MERGED | #844 | `argo skill review export/import` for offline distribution: md/toml/json formats + frontmatter + body + provenance hash + lock round-trip; rejects duplicate hashes; integration test covers default+explicit+lock+collision; surfaced at `skill review` not top-level due to may-touch boundary (rename follow-up deferred) |
| claude-action-cancel-flow | claude | MERGED | #835 | ActionState::Cancelled (terminal) + cancel() transition handler + `argo worker actions cancel` CLI; 6 tests; pending/approved → cancelled allowed, executing rejected |
| claude-audit-replay | claude | MERGED | #836 | `argo audit replay <session>` walks chain in seq order; pretty (default) + raw NDJSON formats; --from-seq + --until filters; 10 tests |
| claude-skill-conflict-resolution | claude | MERGED | #838 | argo-skill::conflicts (detect/render_diff) + `argo skill conflicts list/show/resolve` CLI; 7 tests; drive-by fix: AuditQueryCmd::Replay variant restored (unblocked 4 BLOCKs) |
| claude-eval-streaming | claude | MERGED | #842 | `--stream off|simple|verbose` flag for `argo eval run`; per-fixture progress lines (Simple) + NDJSON (Verbose) + legacy batch (Off); 10 streaming tests; new argo-eval/src/streaming.rs (225 LOC) |
| codex-circuit-breaker | codex | MERGED | #848 | worker circuit breaker for consecutive-fail auto-pause; CircuitState (Closed/Open/HalfOpen) + worker_circuit_history table (migration 17) + `argo worker resume <id>`; threshold env-overridable ARGO_CIRCUIT_THRESHOLD; 5 unit + 1 integration test |
| codex-multi-step-approval | codex | DEFERRED-v1.x | — | DEFERRED to v1.x: tick58 re-block confirmed the work is genuinely architectural — needs dispatch + approval + channel-reply path changes across worker_actions.rs (already at 512 LOC; over the 500 LOC project invariant), worker_actions_transitions.rs, routes_channels.rs. Prerequisite: claude-worker-actions-split (queued below). After that split lands, this brief can resume as 2 sub-briefs: (1) action_signatures table + insert path on channel reply; (2) gate hookup that holds pending until N sigs collected. Codex: do NOT claim. |
| claude-worker-actions-split | claude | MERGED | #851 | pure refactor — extracted list/show/collect_latest_actions/short/lookup_max_attempts into new sibling worker_actions_query.rs; worker_actions.rs 512→305 LOC under 500 invariant; 126 tests pass; no behavior change |
| codex-connector-stripe | codex | MERGED | #861 | native Stripe MCP connector crate (10 files, 695 LOC) — 7 methods (list_customers/get_customer/create_charge/list_charges/refund_charge/create_invoice/list_invoices); STRIPE_API_KEY bearer + STRIPE_BASE_URL test override; charge/refund/invoice approval-gated; list/get autonomous-safe; 4 mocked HTTP unit + 1 stdio wire test; mirror gmail/asana crate pattern. Originally blocked tick 60 on pre-existing flake; unblocked tick 78 after #858 fixed flakes; landed cleanly tick 88. |
| codex-connector-jira | codex | MERGED | #845 | native Jira Cloud MCP connector crate with 7 tools, JQL search passthrough, schema approval markers; merged earlier — ledger drift fix |
| codex-connector-asana | codex | MERGED | #846 | native Asana MCP connector crate (9 files, 735 LOC) — auth/client/lib/main/schemas/service split, 8 methods (workspaces/projects/tasks/get/create/update/complete/comment), ASANA_PAT bearer auth, mocked HTTP + stdio wire tests, approval marks correct (read=false, mutate=true) |
| codex-skill-marketplace-search | codex | MERGED | #849 | `argo skill search <query>` cross-tenant skill search over mocked OCI manifest index; --trust-min/--tenant/--limit/--json filters; 4 unit + 1 CLI smoke test; new cli_skill_search_cmds.rs sibling-file pattern |
| claude-anomaly-detector | claude | MERGED | #847 | Welford baselines + z-score detection + `argo worker anomalies` CLI; 12 tests; migration 19__anomaly.sql (renumbered after #848/#857 took 17/18); merged tick130 — auto-mergeable post-#851 split |
| claude-workflow-dag | claude | MERGED | #857 | argo-workflow crate (DAG/executor/expr/yaml) + `argo workflow validate/run/status/list-runs` + migration 18; 27 unit + 2 integration + 1 doc-test; merged earlier — ledger drift fix |
| claude-filesystem-executor | claude | MERGED | #859 | new argo-executor-filesystem crate (1345 LOC across 17 files); Sandbox guard with traversal/symlink/size caps; 6 typed FS ops (read/write/list/mkdir/remove/stat); FsOpKind classifier feeds AutoApprovalPolicy; FsExecutor/FsConfig/FsAction façade; per-worker fs_root jail. Tab survived TWO stale-sweeps (tick72 + tick81) and finally pushed.
| claude-skill-versioning-ab-test | claude | MERGED | #863 | V2.0 self-improvement: argo-skill::version + ab_test modules; reused existing skill(name,version) UNIQUE constraint (no new table); SHA-256 deterministic 50/50 split + chi-square significance + auto-promote; migration 20__skill_ab_tests.sql; 1182 LOC across 11 files |
| claude-org-awareness | claude | MERGED | #869 | V1.3 multi-worker: argo-identity::org module — MemberKind (Human/Worker), OrgUnit, OrgMember, OrgStore (ensure_schema/list_units/list_members/aliases_for/resolve/reports_to_chain w/ 64-hop cycle guard); 4-pass YAML ingest; migration 21__org_chart.sql; `argo org tree/member/reports-to/ingest`; 1092 LOC across 13 files
| claude-demo-multi-vertical | claude | MERGED | #873 | V2.2 demo verticals (3-in-one): customer-support + sales-sdr + marketing-intern; 677 LOC; charter + skills + Genesis archetype; mission claim 'any role' now has 4 demo verticals shipped
| codex-connector-salesforce | codex | MERGED | #850 | native Salesforce MCP connector crate (8 files, 867 LOC); 7 tools (query/get/create/update/list_accounts/list_opportunities/add_note); OAuth client-credentials + token cache; SOQL passthrough on `query`; approval marks correct |
| codex-connector-zendesk | codex | IN_REVIEW | #852 | PR opened: native Zendesk Support MCP connector crate; CONFLICT after tick63 merge wave: Cargo.toml + Cargo.lock collision after #850 (salesforce) landed first. Needs rebase against current main; connector code itself should be clean. Conductor commented with instructions. |
| codex-setup-wizard-cli | codex | MERGED | #854 | `argo setup` 7-step interactive wizard (doctor/workspace/vertical/worker/channel/archetype/audit); --non-interactive + --json envelope; ASCII checklist rendering; 6 unit + 1 CLI smoke test; new sibling files setup_steps.rs + setup_render.rs |
| codex-compliance-export-cli | codex | MERGED | #855 | `argo audit export --since/--until --format ndjson|csv|tarball [--bundle-out] [--gdpr-redact-emails]`; preflight chain integrity check; tarball bundles audit.ndjson + manifest.json + README.txt; sibling-file pattern in audit_export.rs; 6 unit + 1 smoke test |
| claude-trajectory-replay-ui | claude | MERGED | #853 | TUI overlay key `V` (R was taken by force-refresh) for trajectory replay; 3-pane layout (top bar + action list + step detail); CU/MCP/Human steps render; arrow nav + clamp; 13 new tests; autoplay + search-input + sixel rendering deferred |
| claude-code-execution-executor | claude | MERGED | #864 | V1.2 capability: ExecutorKind::CodeExecution for Python 3.12 + Node 20 via argo-sandbox; new worker_actions_executor_code.rs (~385 LOC); 11 unit tests + 2 ARGO_CODE_EXEC_INTEGRATION-gated integration tests; ARGO_E2B_TOKEN backend stub returns clear 'not yet wired' error pointing at v1.x exec-HTTP-client follow-up brief; conductor opened the PR after worker tab pushed branch but stalled before gh pr create |
| claude-worker-roster | claude | MERGED | #880 | V1.3 multi-worker: tenant-scoped Roster (RosterStore + RosterId in argo-identity); migration 22__roster.sql; Worker::resolved_policy_with_rosters threads roster layer between worker + workspace; multi-roster conflicts collapse via AutoApprovalPolicy::most_restrictive (min budget / fewest windows / intersection of wake_action_types); `argo roster create/list/show/add-member/remove-member` + `argo worker show` surfaces lineage; 1598 LOC across 17 files, 18 unit + 2 integration tests. Brief swept tick114, recovered post-sweep — 3rd post-sweep recovery this session
| claude-spend-anomaly-alerts | claude | MERGED | #892 | V1.1 reliability complementary to #847: argo-store::spend_anomaly Welford rolling stats per (worker, hour-of-day) + z-score detection; migrations 26+27; `argo budget anomalies` + `anomaly-tick`; pre-update z-score; tracing::warn! alert (full NotifyChannel hookup deferred to v1.x); 19 tests; 1197 LOC; merged tick137 |
| codex-gateway-smoke-flake-fix | codex | MERGED | #858 | hardened gateway smoke fixtures: TOCTOU port-handoff fix, isolated ARGO_HOME setup, gateway boot serialization, schema/lock retry; clear RCA in PR body; gateway_smoke + gateway_smoke_writes flakes RESOLVED. Tick-60 known-flakes list will be updated to remove these entries.
| codex-connector-airtable | codex | MERGED | #856 | native Airtable MCP connector crate (8 files, 791 LOC); 7 methods (list_bases/tables/records, get/create/update/delete_record); PAT auth via AIRTABLE_PAT; filterByFormula passthrough on list_records; mirror gmail/asana crate pattern |
| codex-worker-portrait-cli | codex | MERGED | #895 | worker portrait markdown/json summary with identity, policy lineage, budget, action stats, handoffs, skill candidates, anomalies, circuit breaker; 4 unit + 1 smoke; 717 LOC; tab recovered after my tick145 sweep — endless-loop validated again; merged tick147 |
| codex-activity-feed-cli | codex | MERGED | #860 | `argo activity tail [--tenant --worker --since]` real-time audit feed; `argo activity since <duration>`; subscribes to AuditSink::subscribe(); --json NDJSON envelope; reconnect-on-break; 4 unit + 1 CLI smoke test; 695 LOC in 8 files
| codex-gdpr-delete-cli | codex | MERGED | #866 | V2.1 governance: `argo gdpr delete --subject <email>` walks 6 surfaces (memory/audit/skills/channels/workers/sessions); --dry-run default + --confirm with email retype; redaction (not deletion) preserves audit chain; gdpr_subject_erasure audit row; --bundle-out tarball; 8 unit + 1 CLI smoke; 1116 LOC across 9 files. Third attempt at this brief — first two crashed; this one delivered cleanly via the new looping prompt
| codex-connector-linear | codex | MERGED | #865 | native Linear MCP connector crate (8 files, 787 LOC); 7 methods (list_teams/list_issues/get/create/update/comment/list_workflows); GraphQL filter passthrough on list_issues; LINEAR_API_KEY bearer auth; mirror gmail/asana crate pattern
| codex-connector-monday | codex | MERGED | #868 | native Monday.com MCP connector crate (10 files, 839 LOC); 7 methods (list_boards/items/get/create/update_status/add_update/list_groups); API-token bearer; mirror crate pattern
| codex-connector-pagerduty | codex | MERGED | #867 | native PagerDuty MCP connector crate with 7 REST tools, Token auth, approval-gated create/ack/resolve, unit + stdio wire coverage; merged tick130 |
| codex-connector-fireflies | codex | MERGED | #871 | native Fireflies.ai MCP connector crate (8 files, 774 LOC); 5 methods (list_meetings/get_meeting/get_transcript/search_transcripts/list_users); GraphQL passthrough on search; all autonomous-safe (read-only); FIREFLIES_API_KEY bearer; 13th native connector
| codex-audit-report-cli | codex | MERGED | #874 | V2.1 governance: `argo audit report --since/--until --format markdown|html [--output]`; 6 sections (summary stats / channel breakdown / top spenders / pending queue / chain integrity / known flakes); 892 LOC
| codex-doctor-watch-cli | codex | MERGED | #872 | V1.4: `argo doctor watch [--interval --clear]` continuous health monitoring; 206 LOC; reuses argo doctor --json envelope
| claude-policy-as-code | claude | MERGED | #888 | V2.1 governance: argo-identity::policy module + chain resolver (caps tighten only, signatures escalate to max, quiet-hours always apply); migration 25__policy_files; `argo policy list/show/validate/apply`; gate hookup in maybe_auto_approve; 30 tests; 1264 LOC across 13 files; merged tick134 |
| claude-data-classification | claude | MERGED | #893 | V2.1 governance: DataClass enum (Pii/Phi/Financial/Secret/Public/Untagged) + tag table (migration 26 lazy-bootstrap); ClassificationRules TOML override; `argo classify set/list/rules` + --expired filter; PII tags compose with existing --gdpr-redact-emails; per-class retention defaults (PII 90d/PHI 365d/Financial 730d/Secret 30d); 17 unit + 3 integration; 1338 LOC; merged tick143 |
| claude-checkpoint-resume | claude | MERGED | #887 | V1.3 multi-worker: argo-workflow checkpoint + resume + cache_key idempotency — per-task output persisted to workflow_checkpoint table on completion; on-startup scan resumes pending workflows from last checkpoint; cache_key skips re-execution; `argo workflow resume/checkpoints/rollback`; 1271 LOC across 16 files
| claude-trajectory-training-export | claude | MERGED | #870 | V2.0 self-improvement: `argo trajectory export --since/--until --format anthropic|openai|jsonl [--filter passed|all]`; walks .skycot/outputs/<job>/actions/<id>/ bundles; PII redaction integrated; for fine-tuning data pipelines. 1052 LOC across 8 files. Brief was swept tick100, reclaimed + delivered cleanly under endless-loop prompt.
| codex-connector-clickup | codex | MERGED | #878 | native ClickUp MCP connector crate (8 files, 959 LOC); 8 methods (list_workspaces/spaces/lists/tasks, get/create/update_task, add_comment); CLICKUP_TOKEN bearer + CLICKUP_BASE_URL test override; mirror gmail/asana crate pattern; 14th native connector
| codex-tenant-cli-extensions | codex | MERGED | #877 | V1.x: `argo tenant rename/archive/transfer` w/ archived_at soft-delete + global audit row on transfer (old/new owner) + --include-archived list filter; migration 22__tenant_archived.sql; 816 LOC across 9 files. Brief was stale-swept tick110, then re-pushed by tab — eventual-consistency model holds again
| codex-genesis-refine-cli | codex | MERGED | #876 | V1.x: `argo genesis refine <id>` re-refines existing verticals via LLM (reuses #812 prompt + safety judge); 575 LOC across 7 files; --dry-run + --no-judge + --max-iterations; persists versioned old vertical kept as <id>-v<n>; new argo-vertical::version helpers. Conductor opened the PR — worker pushed branch but stalled before gh pr create
| claude-memory-tool-wire | claude | MERGED | #875 | wires memory.list/read/write through MemoryStore w/ vertical scoping; 539 LOC; replaces stale PR #775; vertical-bound writes via append_for_vertical (correctness fix); no uuid dep added; back-compat preserved for builtin_tools(cwd) call sites
| codex-connector-discord | codex | IN_REVIEW | #879 | PR opened: native Discord MCP connector with 7 REST tools, bot-token auth, approval-gated send/react, unit + stdio wire coverage; boundary note for pre-existing argo-cli test failures |
| codex-vertical-archive-cli | codex | MERGED | #884 | V1.x: `argo vertical archive/unarchive/delete <id>` symmetric to tenant-archive (#877); soft-delete via archived_at; --include-archived list filter; delete --force only after archive; bound-worker grace via existing session/worker triggers; 745 LOC across 9 files; tab handled migration race (renumbered 22→24 after #883's migration 23 landed) without force-push — exemplary endless-loop behavior |
| codex-cost-by-skill-cli | codex | IN_REVIEW | #894 | PR opened: `argo budget by-skill` top-N + per-skill history; JSON envelope; 4 store helper tests + CLI smoke. CONFLICTING after #893 data-classification landed; conductor commented w/ rebase instructions. Tab finally pushed after 2 sweeps + 2 reclaims — endless-loop validated. |
| codex-doctor-fix-cli | codex | MERGED | #891 | `argo doctor fix` safe auto-remediation + --auto confirmation + unsafe remediation-only; per-fix audit row; --issue/--json; 6 unit + 1 smoke; original tab recovered after my premature tick135 sweep — exemplary endless-loop behavior; merged tick137 |
| claude-skill-quality-judge | claude | MERGED | #886 | V2.0 self-improvement: per-skill 0-100 quality score (success_rate + latency + retry_count + p99); skill_quality_score table; nightly recompute tick; `argo skill quality/quality-list/quality-recompute`; --quality-min filter on skill list; 1004 LOC across 11 files. The 3rd PR shipped from one tab session w/ #880+#883 — endless-loop validated.

- status: claude-worker-self-exit after 3 PRs (reason: shipped #864/#870/#875; skill-quality-judge BLOCKED on cli_subcommands.rs 500-LOC ceiling — needs split refactor outside scope; iteration time degrading; remaining claude TODOs all touch the same ceiling or are large refactors. Fresh tab can pick up handoff-routing-rules / data-classification / quality-judge once cli_subcommands.rs is split.)
| claude-handoff-routing-rules | claude | MERGED | #883 | V1.3 multi-worker: declarative HandoffRoutingRule DSL (YAML/TOML/JSON via single serde structure); HandoffWalker consults rules by ascending priority before fallback to static escalation chain; selectors: skill_match/role_match/round_robin/specific_worker; `argo handoff rules list/show/add/remove/test`; 1576 LOC across 17 files; composes architecturally with claude-policy-as-code (queued)
| claude-worker-duty-scheduler | claude | MERGED | #899 | **V1.3 LOAD-BEARING (autonomous loop foundation)**: Duty struct extended (id+enabled+paused_until+last_fired_at); new `role_duty` + `duty_fired_session` link tables (migration 28); `argo-orchestrator::duty_scheduler` (DutyStore + DutyScheduler + DutyGate trait + spawn_duty_scheduler tokio task); 5 CLI handlers `argo duty list/show/pause/resume/fire-now`; 13 unit + 2 integration; 1225 LOC; **mission claim "works autonomously" architectural primitive landed**; merged tick150. Gateway boot wiring is small follow-up. |
| claude-onboarding-flow | claude | BLOCKED | — | tick-current investigation: same HARD RULE 6 wall as installer-macos. Adding `Command::Onboard` to cli.rs (currently 509 LOC, already over invariant) pushes it to 528 LOC. Implementation is COMPLETE on branch `claude-onboarding-flow` (pushed to origin): 3 NEW handler files (onboard.rs 32 / onboard_render.rs 76 / onboard_stages.rs 377) + onboard_smoke.rs (99); 14 unit + 3 smoke tests pass; cargo build + clippy --workspace --all-targets -D warnings clean; only the cli.rs/main.rs dispatch wiring violates the LOC invariant. Unblocks cleanly after claude-cli-rs-split lands — pivoting now to ship that. V1.4: `argo onboard` 7-stage interactive walkthrough — welcome/workspace/credentials/connector-auth/first-worker/first-action/done; --non-interactive reads onboarding.toml; first-experience for non-developer operators |
| codex-credentials-management-cli | codex | MERGED | #898 | V1.4: `argo cred set/list/clear/clear-all/export/import`; credentials.toml mode 0600; argo_config::cred::get(key) helper (env-first, falls back to credentials.toml); 6 unit + 1 smoke; merged tick150 — descope strategy worked (fallback wiring deferred to codex-credentials-fallback-wiring) |
| codex-credentials-fallback-wiring | codex | MERGED | #903 | V1.4 mechanical refactor: replaced `std::env::var("X")` → `argo_config::cred::get("X")` across argo-channels + argo-mcp-connector-* + anthropic LLM clients; argo-tool web-search remained env-only (crate cycle); 188+/-111 LOC; descope strategy fully complete; merged tick171 |
| codex-connector-authorize-cli | codex | IN_REVIEW | #905 | PR opened tick178 (post-compaction recovery tab). **NEEDS REWORK**: conductor ran the missing clippy gate locally — 3 `possible_missing_else` errors in connector_list_status.rs:19 (cols 223, 343) + connector_revoke.rs:21 (col 288), all from compacted single-line if-chains during in-place conflict resolution. Conductor commented on PR w/ specific errors. Acceptance criterion "clippy clean" not met. Awaits fresh tab to fix or operator. |
| claude-installer-macos | claude | BLOCKED | — | tick181 investigation: brief requires `argo daemon install/uninstall` subcommand which adds a Command::Daemon arm + dispatch in cli.rs. cli.rs is currently at 509 LOC (already over the 500 invariant); per HARD RULE 6, do NOT add to a 500+ file. Needs new architectural-prep brief `claude-cli-rs-split` (similar to #882/#909 split pattern). Once that lands, this brief can resume — the non-Rust artifacts (Formula/argo.rb + installer/macos/build-dmg.sh + .plist + release.yml extension) are unchanged scope. V1.4 deployment: Homebrew formula + signed .dmg + launchd plist; CI builds .dmg on release tag; replaces `cargo install`-from-source onboarding |
| claude-end-to-end-product-test | claude | TODO | — | V1.4: operator-perspective integration test — fresh ARGO_HOME → onboard --non-interactive → genesis worker → action emit → gate → channel notify → audit chain. Asserts SKYCOT works as a PRODUCT, not just a toolbox. Complement to existing claude-e2e-integration-test (#810/#832) which covers internals |
| claude-runtime-planner-trait | claude | MERGED | #897 | V1.3: new argo-runtime crate (Planner trait + TemplatePlanner offline + LlmPlanner via pluggable AnthropicCaller); `argo plan generate/show/execute` CLI; YAML output + `.reasoning` sidecar; --apply uses argo-workflow MockRunner; 12 unit + 2 integration tests; ~1179 LOC across 16 files; merged tick150 |
| claude-subtask-ledger-store | claude | TODO | — | UNBLOCKED tick150: predecessor #897 runtime-planner-trait merged. V1.3: subtask_ledger table tracks Planner-emitted subtasks per parent goal; durability for restart-resume; CLI `argo subtask list/show/tree`; precursor to v2.0 dreams-plan-templates cross-pollination |
| claude-web-dashboard-mvp | claude | TODO | — | V1.4 operator UX: server-rendered HTML + HTMX dashboard at /dashboard/ — workers/sessions/approvals/spend tabs; SSE auto-refresh; inline approve/deny; bundled htmx.min.js (no CDN). Minimal, no React, no build step |
| codex-channel-imessage-bluebubbles | codex | MERGED | #890 | iMessage approval channel via Blue Bubbles + native MCP connector (5 tools: send/list/get/react); 6 unit + 1 wire test; merged tick134 |
| codex-channel-voice-twilio | codex | MERGED | #896 | voice approval via Twilio: outbound call + TwiML <Say> w/ action description + DTMF capture (1=approve, 2=deny, 9=details); HMAC token in callback URL; signature validation; 5 unit + 1 integration; 1037 LOC; merged tick149 |
| claude-tui-realtime-dashboard | claude | MERGED | #881 | V1.4 operator UX: TUI overlay key `D` — workers/pending-approvals/audit-tail in 3 stacked panes (30/30/40 split); auto-refresh on existing 1s event-loop tick (no polling); 923 LOC across 10 files; mirrors claude-tui-h-stack-panels (#822) + claude-trajectory-replay-ui (#853) architecture
| claude-genesis-charter-conversation | claude | MERGED | #885 | V1.4 Genesis maturity: `argo genesis interactive` 6-prompt conversational walkthrough — role/name/industry/duties/channels/spend-cap → CharterDialogue → renders to plain-English charter consumed by existing argo-genesis::plan; downstream archetype/safety-judge/manifest/refinement unchanged. 1077 LOC across 12 files. The 'one sentence to working Maya' UX gap from tick-103 honest assessment now substantially closed. |
| claude-mobile-approval-app-tauri | claude | TODO | — | V1.4 mobile UX: Tauri iOS+Android shell — 2 screens (pending approvals list, settings); thin client over existing /approvals/pending + /decisions/<token> routes; deep-link from push notifications; bundled, no CDN, app < 30 MB |
| codex-channel-whatsapp-twilio | codex | MERGED | #913 | V1.4 channels: WhatsApp approval via Twilio (parallel to D-sms); routes_channels_whatsapp.rs sibling (post-#910 split layout); approve/deny keyword parsing; HMAC token replay prevention; gateway/mod.rs held at 499 LOC; 5 unit + 1 integration; 685 LOC; conductor verified clippy clean; merged tick194 |
| codex-vertical-publish-real-oci | codex | MERGED | #908 | V1.x: real OCI Distribution Spec push/pull via oci-distribution crate when --registry set; mock path preserved; auth fallback `--token > credentials.toml > docker config > anonymous`; typed vertical layers; ttl.sh live round-trip PASS; mock fixed (chunked PATCH, Location header, URL-decode); 6+ unit + 1 gated smoke. **Conductor ran missing clippy gate — clean.** Compaction-degraded tab disclosed gap honestly; extra-scrutiny merged tick180 |
| claude-skill-auction | claude | MERGED | #900 | V1.3 multi-worker: argo-orchestrator::auction module + AuctionStore + fit_score + deterministic 3-tier tie-break; migration 27; HandoffWalker `with_auction_store` opt-in; `argo handoff-auctions list/show/cancel`; 15 unit + 2 integration; 1208 LOC. Rebased + merged tick177 after recovery brief landed. |
| codex-backup-restore-cli | codex | MERGED | #904 | V1.x: `argo backup create/list/verify` + `argo restore`; tarball of <workspace>/.argo/; credentials excluded by default (--include-credentials opt-in); sha256 in manifest.json; refuses non-empty workspace restore; 9 unit + 1 CLI smoke round-trip; 1007 LOC; merged tick173 |
| claude-docs-site-build | claude | TODO | — | V1.4 documentation: mdBook site at /book/ — Quickstart + How-to + Reference (auto-gen from argo --help) + Architecture (PROTOCOL.md + investigations) + Operations (RUNBOOK.md); GitHub Pages deploy on main push; mdbook-linkcheck in CI |
| codex-self-update-cli | codex | BLOCKED | — | blocked: brief depends on `claude-installer-macos`, which is currently BLOCKED; daemon install/restart and release-signing workflow are not landed yet |
| claude-operator-feedback-channel | claude | MERGED | #889 | operator_feedback table (migration 24) + Vote enum/CRUD; FeedbackVote + should_propose_with_feedback in argo-skill (Up forces propose, Down blacklists, None falls back); `argo feedback up/down/list` + --json; 9 unit + 2 integration tests; merged tick134 |
| claude-computer-use-native-macos-poc | claude | TODO | — | V1.5: NATIVE macOS computer-use PoC — design doc + working executor driving macOS Notes via accessibility APIs + Anthropic CU model. Bounded scope: 1 design doc + 1 PoC app + 1 smoke test. Closes the "browser-only CU" gap toward "any task a human can do on a Mac" |
| claude-multi-host-orchestrator-design | claude | TODO | — | V2.x: distributed conductor design + 2-host POC — design doc covering topology/sync/discovery/dispatch/split-brain + working /peer/sync endpoint with cross-host session visibility. Bounded scope (NOT full distributed system); follow-up briefs after this lands |
| claude-document-creation-executor | claude | TODO | — | V1.2 capability: new ExecutorKind::DocumentCreation — PDF (markdown→headless Chrome, reuse build-pdf toolchain) + Markdown + slides (reveal.js) + CSV (RFC 4180); writes to fs_root/outputs only; approval-gated by default |
| claude-voice-local-tui-whisper | claude | TODO | — | V1.4: `argo voice` TUI — local Whisper.cpp + cpal mic capture; grammar (approve/deny/details/next/quit); fuzzy worker-name match; wake-word "Argo" gates listening; pure-local privacy + offline-capable |
| claude-dreams-memory-summarize | claude | TODO | — | V2.0 (Dreams Integration A): replace argo-memory::summary LLM-call path with Claude Dreams API backend; nightly per-worker tick + on-demand `argo memory dream-now`; per-worker $5/day cap; anomaly-event guard prevents poisoning; feature-flag gates path |
| claude-dreams-skill-distillation | claude | TODO | — | V2.0 (Dreams Integration B): Dreams over cohorts of successful trajectories (≥3 with cosine sim > 0.7) → polished SkillCandidate body via dreams_distillation; mechanical fallback for small cohorts; shared daily cap with memory-summarize |
| claude-dreams-plan-templates | claude | TODO | — | V2.0 cross-pollination (claim4×claim2): Dreams over PLANNER COHORTS (subtask_ledger by parent_goal similarity) → LearnedPlanTemplate; TrustGrade ladder applies; TemplatePlanner consults templates first. The genuinely-novel proposal from DREAMS investigation |
| claude-vertical-finance-ar | claude | TODO | — | V2.2 demo verticals (4th): Accounts Receivable demo — AR worker reviews invoices, sends 7/14/30-day reminders via gmail, escalates at 45d, logs Stripe payment events, weekly aging report via document-creation-executor. Touches gmail+stripe+documents (broad capability proof) |
| codex-skill-quality-judge-cli | codex | MERGED | #912 | V2.0: `argo skill quality-trend <name>` 30-day ASCII sparkline (▁..█ + `·` gap); reuses SkillQualityStore::history from #886; 6 unit + 1 smoke; conductor verified clippy clean. **First PR after #909 split — confirms unlock pattern works (skill.rs 184 LOC has room).** Merged tick188 |
| claude-error-recovery-suggestions | claude | MERGED | #901 | V1.4: `argo recover <session-id>` LLM-assisted post-mortem — walks audit chain, calls Anthropic, returns 3 ranked fix suggestions; cached per (session_id, chain_hash); advisory only (no auto-apply); migration 28__recovery_cache (lazy-bootstrap); AnthropicCaller trait + EnvAnthropicCaller; 12 unit + 3 integration tests; 759 LOC; merged tick162 |
| codex-skill-marketplace-publish-cli | codex | CLAIMED | — | claimed by codex |
| codex-budget-alert-channel | codex | MERGED | #902 | V1.4 reliability: budget watcher polls spend_today/spend_this_month, fires PushApprovalChannel at 80%/100% threshold cross; per-day per-(worker,threshold) dedup; midnight reset; --synthetic test bypass; new worker_budget_alert table (migration 26 lazy-bootstrap); `argo budget watch/alerts list/alerts test`; 5+ unit + 1 smoke; 892 LOC; merged tick165 |
| codex-action-history-cli | codex | MERGED | #907 | V1.4 operator UX: `argo action history/timeline/recent` chronological newest-first + bucket histogram + cross-worker view; --since/--include-skill/--tenant/--limit filters; --json envelopes; 6 unit + 1 smoke. **Conductor ran missing clippy gate — clean.** Compaction-degraded tab disclosed gap; extra-scrutiny merged tick180. Deviation: argo-store/lib.rs at 504 LOC (already over invariant) so query path stayed in CLI handlers |
| codex-channel-discord-approvals | codex | BLOCKED | — | depends on (a) codex-connector-discord (#879) merged AND (b) claude-routes-channels-split merged (gateway dispatcher at LOC ceiling). Discord NotifyChannel for approval flow — embed message + 2 buttons via REST; HMAC-signed `/discord/decision` callback; mirrors slack-approvals/sms/push pattern |
| codex-skill-import-export-cli | codex | TODO | — | UNBLOCKED tick181: #909 skill-handlers-split merged. Codex tab previously implemented partial work in /tmp/codex-skill-import-export-cli (cli_skill_io_cmds.rs NEW + skill_export_all.rs NEW + flate2+tar pack/unpack) — now this is a 3-arm wire-up. V2.0 offline: `argo skill export/export-all/import` tarball + signature check + --name rename |
| codex-tenant-quota-cli | codex | TODO | — | V1.4 multi-tenant: `argo tenant quota set/show/list/clear` for per-tenant max_usd/max_actions/max_workers (daily or monthly); migration 28; auto-approval gate consults tenant quota AFTER per-worker; tenant breach blocks regardless of worker headroom |
| codex-tenant-list-show-cli | codex | BLOCKED | — | blocked on `claude-argo-store-lib-split` (queued tick182): argo-store/src/lib.rs at 504 LOC blocks adding tenant_query.rs re-export. Once split lands, this is a 3-handler wire-up |
| codex-eval-compare-cli | codex | CLAIMED | — | claimed by codex |
| codex-doctor-channels-extend | codex | MERGED | #906 | V1.4 mechanical: extended `argo doctor channels` (#828) with imessage (#890 Blue Bubbles ping) + voice (#896 Twilio ping) checks; reuses OK/WARN/ERROR/MISSING vocabulary + 5s reqwest timeout; doctor_channels.rs 315→459 LOC; 4 unit tests; 145+/-1 LOC; merged tick179 |
| codex-skill-runs-cli | codex | TODO | — | V1.4 operator UX: `argo skill runs <name>` chronological invocation history; --worker --since --include-failed --tenant filters; reuses worker_action+spend_ledger joins (no new tables); pure read-side; companion to skill quality/review |
| codex-handoff-stats-cli | codex | BLOCKED | — | scoped-out: schema gap. handoff table has only created_at + deadline — no resolved_at / settled_at column, so brief-required "median latency from request to resolution" is unattainable without a schema migration. `handoff_settled` audit kind is referenced in argo-cli/src/handlers/audit_query.rs but never written by argo-orchestrator/src/persist.rs (only handoff_cancelled is). Other metrics (volume, success/escalation rates, top from/to) are doable but the brief's first-class latency metric requires a new resolved_at column + persist hookup, which is out of brief scope. Re-claim once schema migration is queued separately. |
| codex-recover-cache-cli | codex | IN_REVIEW | #911 | **HOLD — HARD RULE 6 violation**: grew cli.rs 509→518 + argo-store/lib.rs 504→507 (both already over invariant). Conductor commented; merge after PRIORITY splits land (claude-cli-rs-split + claude-argo-store-lib-split). Work clean otherwise: argo recover-cache list/show/clear; 7 store + 5 handler unit + 1 smoke; clippy clean |
| codex-budget-export-csv-cli | codex | TODO | — | swept tick191 (operator confirmed only 2 codex tabs running — eval-compare + whatsapp-twilio — this CLAIM was the zombie). V1.4 mechanical: add `--format csv` to `argo budget show/recent/anomalies/alerts list` (RFC 4180); operators pipe to spreadsheets/billing tools; small ~150 LOC across 4 handlers + 1 smoke test |
| codex-skill-sign-cli | codex | TODO | — | V1.4 supply-chain: cosign-compatible signing on the OCI publish path landed in #908. `argo skill sign/verify/keygen` using ed25519-dalek (existing transitive dep); detached signature stored as sibling OCI artifact per cosign spec; `--keyless` (Sigstore Fulcio) deferred to follow-up. 6 unit + 1 smoke (sign → tamper → verify FAIL) |
| codex-connector-list-cli | codex | TODO | — | V1.4 inspection: `argo connector list/show` walks manifests/connectors/*.toml + crate metadata; --installed-only filter; companion to authorize/list-status (#905). NOTE: depends on cli_connector_cmds.rs created by #905 — wait for #905 to land or extend brief to create that file |
| codex-duty-scheduler-gateway-wiring | codex | TODO | — | V1.3 LOAD-BEARING follow-up to #899: `spawn_duty_scheduler` is a public API on argo-orchestrator but nothing in gateway boot calls it — the autonomous loop is dormant. This brief wires DutyStore + DutyGate (ClosedGate as v1 fail-safe) + spawn into gateway lib.rs run path; integration test verifies a duty fires. Mission claim "works autonomously" requires this to be live. |
| claude-cli-subcommands-split | claude | MERGED | #882 | V1.x architectural prep: split crates/argo-cli/src/cli_subcommands.rs (was 500 LOC) into 3 focused sibling files — cli_subcommands_skill.rs (136 LOC) + cli_subcommands_provider.rs (49 LOC) + cli_subcommands_cron.rs (55 LOC); main file trimmed to 269 LOC; pure refactor, public API stable via re-exports; 4th post-sweep recovery this session. Unblocks claude-skill-quality-judge |
| claude-routes-channels-split | claude | MERGED | #910 | V1.x architectural prep: pure refactor splitting routes_channels.rs (499→184 LOC) into 4 per-channel siblings (email/slack/webinbox/push); pub(super) sub-mods; gateway/mod.rs held at 499 (no regression); 3 tests preserved + 503 workspace pass. Unblocked codex-channel-whatsapp-twilio (now TODO). codex-channel-discord-approvals still BLOCKED on #879. Merged tick181 |
| claude-skill-handlers-split | claude | MERGED | #909 | V1.x architectural prep: pure refactor splitting handlers/skill.rs (500→181 LOC) into 5 per-domain siblings (list/view/verify/create/refine); body_hash pub(super); 502 argo-cli tests pass. Unblocked 3 codex briefs: skill-quality-judge-cli + skill-marketplace-publish-cli + skill-import-export-cli (all now TODO). Merged tick181 |
| **claude-argo-store-lib-split** | claude | CLAIMED | — | **PRIORITY** — architectural prep, unblocks codex-tenant-list-show-cli. argo-store/src/lib.rs at 504 LOC. Pure refactor mirroring #851/#882/#909/#910: lib.rs ≤300 LOC + prelude.rs; pub use re-exports stable |
| **claude-cli-rs-split** | claude | IN_REVIEW | #914 | **PRIORITY** — PR open: pure refactor extracting `#[path] mod cli_<x>_cmds;` declarations + `pub use cli_<x>_cmds::*;` re-exports into new `cli_reexports.rs`. cli.rs 509→443 LOC (57 LOC headroom). API preserved via `pub use cli_reexports::*` glob + 8 explicit module re-exports for `crate::cli::cli_<x>::Type` external paths. cargo build + clippy --workspace --all-targets -D warnings + 512 tests pass. 3 known pre-existing flakes documented as boundary note. Unblocks 3 BLOCKED briefs once merged: claude-installer-macos + claude-onboarding-flow + codex-recover-cache-cli (#911). |

---

## Picking your next brief

Codex picks the first row where:
- `owner == codex`
- `status == TODO`
- All "waits on" predecessors have `status == MERGED`

Claude picks the first row where:
- `owner == claude`
- `status == TODO`
- All "waits on" predecessors have `status == MERGED`

If multiple agents try to claim the same row, the one whose
commit lands first on main wins. The losers will see the row is
already CLAIMED on their next pull and pick a different one.

---

## Blanket CLI allowlist (tick 32 amendment)

Repeated BLOCKs from may-touch lists that allow a NEW file
but forbid the EXISTING dispatch wiring. Going forward, every
CLI-extension brief MAY also touch:

- `crates/argo-cli/src/cli.rs` — narrowly: ONE arm in the
  Command enum + ONE dispatch arm
- `crates/argo-cli/src/main.rs` — narrowly: ONE handler call
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: ONE
  `pub mod <name>;` line
- `crates/argo-cli/tests/<name>.rs` — NEW integration test file
- `crates/argo-cli/Cargo.toml` — ONE line for any new path
  dep on a sibling crate

Strict scope still applies: ONE arm/line per file. Don't
refactor unrelated logic. The blanket unblocks tier-2 CLI
briefs without per-brief amendments.

This applies retroactively to any BLOCKED brief flagging
"dispatch file outside allowlist" — codex may resume those
briefs in fresh worktrees on origin/main.

## Tick 57 amendment: BUILD_PLAN_V11 (beyond v1 demo)

V1 demo gate is GREEN (9/9). The 12 rows added to the queue
this tick (`codex-circuit-breaker` through
`claude-demo-multi-vertical`) are the highest-impact-now
slice of the strategic plan in
[`BUILD_PLAN_V11.md`](BUILD_PLAN_V11.md), which sequences
work from v1.1 (production hardening) through v2.2 (more
demo verticals).

The conductor's standing replenishment loop (Step 8 of the
loop prompt) continues to fire whenever either queue dips
≤8 (raised from the original ≤3 at tick 59 — the lower
threshold risked workers running out of work between
ticks since codex tabs poll on 60s intervals and can
drain ~3 briefs in a single tick window). Replenishment
draws from BUILD_PLAN_V11.md + recent PR-body deferrals +
git-log direction + MISSION.md gaps. Target queue depth
after replenishment: 12-16 briefs total, weighted ~50/50
codex/claude to keep both worker pools busy.

The stop condition is unchanged: every brief MERGED AND
e2e-test green. When fully drained, append a `Plan
complete` note below.

## Tick 45 amendment: exhaustive-match propagation

When a brief adds a new enum variant (e.g.
`HandoffStatus::Cancelled`, `MemoryCmd::SharedTeam`),
Rust's exhaustiveness check forces every existing `match`
on that enum across the workspace to handle the new arm.
Those adjacent files are AUTOMATICALLY may-touch under
strict scope: ONLY the new arm (usually one line; use
`_ => ...` if no bespoke handling is needed).

Specifically: `gateway/routes_orchestrator.rs`,
`handlers/memory.rs`, `worker_actions.rs`, and any other
handler that pattern-matches on the enum being extended
is may-touch — only the new arm; do not refactor.

Default action: fix the exhaustive match in place.

---

## Tick 115 amendment: claim-collision defenses

Three collision patterns observed and codified:

1. **Auto-compaction-amnesia** — a tab finishes a brief, gets
   auto-compacted, comes back not knowing its own work shipped,
   re-implements the same brief. **Defense**: worker prompts
   now include a PRE-PUSH FLIGHT CHECK (step 8) — re-pull main
   + grep BUILD_STATUS for the brief-id before pushing the
   feature branch. If MERGED → graceful exit. Stops 90% of
   duplicate-PR situations.

2. **False-positive stale-sweep + late recovery** — conductor
   sweeps a 30-min CLAIMED row to TODO, fresh tab claims, but
   the original tab was alive in slow build/test, eventually
   pushes a PR → conflict. **Defense**: stale-sweep threshold
   bumped from 30 min to **45 min**. Big briefs (architectural,
   multi-crate) legitimately need 30+ min for cargo build +
   test + clippy; 45-min threshold reduces false positives.

3. **Simultaneous next-claim race** — two tabs finish
   near-concurrently, claim the same next-TODO row, race
   resolved by first-push-wins. **Defenses**:
   (a) Worker prompts now include ITERATION JITTER (step 12) —
   sleep random 5-30s before next claim, spreads tabs out.
   (b) RACE-LOST HANDLING (step 3) is more explicit: pull,
   re-read ledger, claim a DIFFERENT row (never retry the
   same one).

**Conductor habit (tick 115)**: when manually opening a PR for
a stalled worker (worker pushed branch but didn't `gh pr create`),
add a note to the BUILD_STATUS row: "conductor-opened; original
tab should self-exit on next ledger re-read". This way the
original tab's RULE 3 check (don't reimplement merged briefs)
fires correctly post-compaction.

---

## Known dependencies

```
A-executor-trait
    ├── B-computer-use-executor
    ├── C-mcp-wire
    │       ├── C-gmail
    │       ├── C-slack
    │       ├── C-gcal
    │       ├── C-hubspot
    │       └── C-notion
    └── G-genesis (after A stable)

D-email
    ├── D-slack-approvals
    ├── D-sms
    ├── D-push
    └── D-webinbox

E-identity
    └── F-memory-layering

H-auto-approval
    ← A, D-email, E-identity, F-memory-layering all MERGED
```

The "waits on" column above encodes this graph. A row with
`MERGED` predecessors is ready to claim.

---

## Blocked rows

(none currently)

When marking BLOCKED, append a short reason:
`status=BLOCKED reason="cargo clippy fails on field X; need Stream Z first"`.

---

## Done rows

(none currently)

When the queue is fully MERGED, append a `Plan complete` note
here with the date and a link to the v1 demo recording.

## Worker self-exits

- status: codex-worker-self-exit after 0 PRs (reason: no claimable codex TODO rows; codex-worker-portrait-cli waits on claude-anomaly-detector, codex-gdpr-delete-cli already CLAIMED)
- status: codex-worker-self-exit after 0 PRs (reason: no claimable codex TODO rows; codex-worker-portrait-cli waits on claude-anomaly-detector, codex-gdpr-delete-cli already CLAIMED)
- status: codex-worker-self-exit after 0 PRs (reason: no claimable codex TODO rows on fresh main; codex-worker-portrait-cli waits on claude-anomaly-detector, codex-gdpr-delete-cli already CLAIMED)
- status: codex-worker-self-exit after 0 PRs (reason: verification environment degraded: disk full / target corruption / compile killed)
- status: claude-worker-self-exit after 3 PRs (reason: shipped #880 worker-roster + #883 handoff-routing-rules + #886 skill-quality-judge cleanly; iter time stable at ~30/80/80 min, no degradation indicators firing, but context-window usage is significant after 3 substantial briefs across 3 different crates and ~50 tool calls per iteration. Clean exit before auto-compaction risk. Worker prompt was the post-recovery "loop endlessly until context degrades" variant; chose to exit while context-awareness is still intact rather than push into degraded territory. Fresh tab can pick from claude-spend-anomaly-alerts (still blocked on claude-anomaly-detector IN_REVIEW), claude-policy-as-code, claude-data-classification, claude-onboarding-flow, etc. — claude TODO depth still healthy.)
- status: claude-worker-self-exit after 3 PRs (reason: shipped #882 cli-subcommands-split MERGED + #887 checkpoint-resume MERGED + #889 operator-feedback-channel IN_REVIEW; iter times ~70/90/70 min; no self-degradation indicators firing (recall of last 2 PRs intact, no in-flight recompile failures unrelated to disk pressure); /tmp disk hit 100% twice during the session and required cleanup of an old baseline-test worktree to recover, suggesting the environment is at its limit; matches the prior tab's healthy 3-PR exit pattern. Fresh tab can pick from claude-spend-anomaly-alerts (anomaly-detector NOW MERGED #847 — newly unblocked), claude-data-classification (was scope-passed by this tab — retention enforcement + audit-export hooks are larger than first scope-pass; needs a fresh dedicated tab), claude-handoff-routing-rules / -worker-duty-scheduler / -onboarding-flow, plus the executor-wiring follow-up to integrate `should_propose_with_feedback` into `argo-executor-computer-use::learn::maybe_propose`. cli_subcommands.rs ceiling is now permanently lifted (269 LOC after split) so the previously-blocked claude TODOs that touched it are all viable.)
- status: codex-worker-self-exit after 1 PRs (reason: auto-compaction fired unexpectedly after #890; stopping per worker prompt stop condition 3)
- status: codex-worker-self-exit after 1 PRs (reason: auto-compaction fired unexpectedly after #895; stopping per worker prompt stop condition 3)
- status: claude-worker-self-exit after 4 PRs (reason: shipped #881 tui-realtime-dashboard + #885 genesis-charter-conversation + #888 policy-as-code + #892 spend-anomaly-alerts cleanly across 4 iterations. Recall of last 2 PRs (#888, #892) intact. Iter times stable at ~3hr / 2hr / 3hr / 2.5hr — no 2× degradation. Same 3 pre-existing flakes (gateway_smoke / setup_steps env-leak / gtm_engineer schema mismatch in verticals) consistently surface across iterations; documented in every PR boundary note. Pre-existing bugs in others' tabs that I did not author. Multiple successful parallel-tab pull/rebase cycles handled cleanly. Choosing to exit while context-awareness is still intact rather than push into degraded territory — context-window usage is heavy after 4 substantial briefs across argo-tui, argo-genesis, argo-identity (policy + spend_anomaly via argo-store), argo-cli (4 new handler families). Total ~4500 LOC + tests across 4 PRs. Fresh tab can pick from: claude-data-classification (was scope-passed by another tab — needs dedicated effort), claude-onboarding-flow (deps merged), claude-installer-macos (no deps; needs Homebrew/.dmg toolchain), claude-runtime-planner-trait (workflow-dag #857 NOW MERGED — newly unblocked), claude-skill-auction (handoff-routing-rules #883 NOW MERGED — newly unblocked), claude-multi-host-orchestrator-design, claude-document-creation-executor, claude-error-recovery-suggestions, claude-vertical-finance-ar (depends on document-creation-executor still TODO), claude-docs-site-build, claude-web-dashboard-mvp, claude-tui-realtime-dashboard's `+`/`-` feedback hookup deferred from #889 (operator-feedback-channel). Most of these are sizeable; pick one focused brief per fresh-tab session.)
- status: claude-worker-self-exit after 0 PRs (reason: race-lost twice in succession (worker-duty-scheduler then onboarding-flow) per worker-prompt rule "If push fails AGAIN: you may be in a degraded state; exit cleanly per RULE 4". Combined with /tmp disk at 98% (22 GiB free across 9 in-flight worktrees in /private/tmp/) — same environmental pressure flagged by the previous claude tab's exit at 351. Multi-tab contention is high right now (3+ claude tabs were active when I started; 1 self-exited mid-session, ≥1 still working on data-classification). Queue health is fine — claude TODO depth still healthy with worker-duty-scheduler / onboarding-flow / installer-macos / end-to-end-product-test / runtime-planner-trait / subtask-ledger-store / skill-auction / web-dashboard-mvp / docs-site-build all TODO. Recommend: free disk space (cleanup MERGED-brief worktrees in /private/tmp/) and reduce parallel claude-tab count before relaunching, otherwise next tab will hit the same contention.)
- status: claude-worker-self-exit after 4 PRs (reason: shipped #893 data-classification MERGED + #897 runtime-planner-trait IN_REVIEW + #900 skill-auction IN_REVIEW + #901 error-recovery-suggestions IN_REVIEW cleanly across 4 iterations. Recall of last 2 PRs (#900 + #901) intact. Iter times stable at ~30/60/50/50 min — no 2× degradation. Same 3 pre-existing flakes (gateway_smoke webinbox / setup_steps env-leak / gtm_engineer schema mismatch in verticals) consistently surface across iterations; documented in every PR boundary note. One race-lost handled cleanly per RULE 3 (claude-onboarding-flow → switched to claude-runtime-planner-trait). Total ~4500 LOC + tests across 4 PRs spanning argo-identity (classification), argo-store (classification + recovery_cache migrations), argo-memory (lib.rs append_with_class), new argo-runtime crate (Planner + LlmPlanner + TemplatePlanner), argo-orchestrator (auction module + walker hookup), argo-cli (classify/plan/handoff-auctions/recover handler families). Choosing to exit while context-awareness is still intact rather than push into degraded territory — matches prior tab's healthy 4-PR exit pattern. Fresh tab can pick from: claude-onboarding-flow (still TODO; deps merged), claude-installer-macos (no deps but needs Homebrew/.dmg toolchain), claude-end-to-end-product-test (depends on onboarding-flow), claude-subtask-ledger-store (now unblocked once #897 merges; persists Planner-emitted subtasks), claude-web-dashboard-mvp (HTML+HTMX dashboard, sizeable), claude-multi-host-orchestrator-design (design doc + 2-host POC), claude-document-creation-executor (new ExecutorKind for PDF/markdown/slides), claude-docs-site-build (mdBook), claude-skill-auction live tokio-broadcast bidder loop (follow-up deferred in #900), and the runtime-planner real worker dispatch follow-up deferred in #897. Several v1.5+ briefs (computer-use-native-macos-poc, voice-local-tui-whisper, dreams-*) need specialized toolchain access.)
- status: claude-worker-self-exit after 2 PRs (reason: shipped #900 claude-skill-auction MERGED via the recovery rebase brief + #910 claude-routes-channels-split MERGED cleanly across iter 1+2. Recall of last 2 PRs (#900, #910) intact. Iter times reasonable: iter1 ~30 min (rebase only), iter2 ~75 min (4 new sibling files for routes_channels split + 503 workspace-test pass). **Iter 3 hit two consecutive friction points before any implementation work**: (1) claimed claude-installer-macos (first claude TODO row) but discovered cli.rs is at 509 LOC — already over the 500 invariant — so adding `Command::Daemon` violates HARD RULE 6. Marked installer-macos BLOCKED with reason "needs claude-cli-rs-split first" and pivoted. (2) Considered claude-multi-host-orchestrator-design but the brief's may-touch list (`crates/argo-gateway/src/peer.rs`, `routes_peer.rs`) is in the wrong crate — argo-gateway is the chat-platform-adapters crate (Slack/Discord/Telegram), not the HTTP gateway server (which lives at `argo-cli/src/handlers/gateway/`). The peer-mesh + /peer/sync HTTP endpoint architecturally belongs alongside routes_orchestrator/routes_audit/etc. in argo-cli; the brief author appears to have conflated the two `gateway`-named things. Rather than pick a misinterpretation that may diverge from intent, exiting cleanly. Per STOP CONDITION 4 (degradation): iteration N planning time growing without output is the soft indicator. Two clean PRs is better than three with the third half-baked. Fresh tab can pick from: (a) claude-cli-rs-split — new architectural-prep brief I'd recommend the conductor queue, mirrors #882/#909 pattern, would unblock installer-macos + subtask-ledger-store + future CLI extensions. (b) claude-multi-host-orchestrator-design after operator/conductor scope-clarifies the crate placement. (c) claude-web-dashboard-mvp (HTML+HTMX dashboard, sizeable but no cli.rs touch — gateway routes only). (d) claude-document-creation-executor (new crate; would still hit cli.rs ceiling for the executor-kind dispatch arm — same blocker). (e) claude-mobile-approval-app-tauri (standalone Tauri shell, no cli.rs touch). (f) claude-docs-site-build (mdBook + GH Pages workflow, no Rust). The cli.rs ceiling is the single biggest blocker for claude TODOs right now; queueing claude-cli-rs-split as PRIORITY would unblock multiple briefs.)
- status: codex-worker-self-exit after 3 PRs (reason: shipped #906 codex-doctor-channels-extend MERGED + #911 codex-recover-cache-cli IN_REVIEW + #912 codex-skill-quality-judge-cli IN_REVIEW cleanly across iter 1+2+5 (iters 3+4 were BLOCK pivots; see below). Recall of last 2 PRs (#911, #912) intact. **STOP CONDITION 2 (Same op fails twice; environment degraded) triggered on iter 6**: tried codex-budget-export-csv-cli, but cargo build inside /tmp/codex-budget-export-csv-cli started failing with `failed to open object file: No such file or directory` after partial implementation. Disk pressure: /private/tmp at 99% capacity (13 GiB free, with 17 GiB held by an abandoned /private/tmp/codex-channel-whatsapp-twilio worktree from the BLOCKED whatsapp brief — biggest single offender). The /tmp/codex-budget-export-csv-cli worktree directory disappeared mid-implementation (uncommitted edits to cli_budget_cmds.rs + budget.rs + budget_anomalies.rs + budget_alerts_list.rs lost). Conductor swept the CLAIM at tick191 thinking my tab was a zombie (operator confirmed only 2 codex tabs running — they didn't account for this third tab). Two BLOCKED rows during the session were both true blockers, not avoidance: codex-skill-import-export-cli BLOCKED on handlers/skill.rs at 500 LOC ceiling (resolved later by #909 split — fresh tab can re-claim, partial work in /tmp/codex-skill-import-export-cli still has cli_skill_io_cmds.rs + skill_export_all.rs + flate2+tar pack/unpack stubs); codex-handoff-stats-cli BLOCKED on schema gap (handoff table has only created_at + deadline, no resolved_at column — brief-required "median latency" needs a separate schema-migration brief queued first; handoff_settled audit kind is referenced in argo-cli but never written by argo-orchestrator/persist.rs, only handoff_cancelled is). Recommendations for operator: (a) Free /private/tmp by removing /private/tmp/codex-channel-whatsapp-twilio (17 GiB; brief is BLOCKED on routes-channels-split which has since merged via #910 — worktree is stale). (b) Queue a schema-migration brief that adds resolved_at + populates it on Settled/Cancelled/Expired transitions in argo-orchestrator::persist::update_handoff_status, then re-open codex-handoff-stats-cli. (c) codex-skill-import-export-cli is now genuinely unblocked by #909; partial work in worktree could be revived. Fresh tab can pick from the still-TODO codex queue: codex-self-update-cli (needs cosign infra), codex-skill-marketplace-publish-cli (V2.0), codex-skill-runs-cli (needs SkillCmd extension), codex-budget-export-csv-cli (just-swept; ~150 LOC mechanical), codex-skill-sign-cli (cosign signing), codex-connector-list-cli (depends on #905 merging), codex-duty-scheduler-gateway-wiring (V1.3 LOAD-BEARING — `spawn_duty_scheduler` is dormant; gateway boot wire-up unblocks the autonomous-loop mission claim).)
