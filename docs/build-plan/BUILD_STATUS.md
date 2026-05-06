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
| codex-connector-jira | codex | IN_REVIEW | #845 | PR opened: native Jira Cloud MCP connector crate with 7 tools, JQL search passthrough, schema approval markers, unit + wire coverage |
| codex-connector-asana | codex | MERGED | #846 | native Asana MCP connector crate (9 files, 735 LOC) — auth/client/lib/main/schemas/service split, 8 methods (workspaces/projects/tasks/get/create/update/complete/comment), ASANA_PAT bearer auth, mocked HTTP + stdio wire tests, approval marks correct (read=false, mutate=true) |
| codex-skill-marketplace-search | codex | MERGED | #849 | `argo skill search <query>` cross-tenant skill search over mocked OCI manifest index; --trust-min/--tenant/--limit/--json filters; 4 unit + 1 CLI smoke test; new cli_skill_search_cmds.rs sibling-file pattern |
| claude-anomaly-detector | claude | IN_REVIEW | #847 | PR opened — Welford baselines + z-score detection + `argo worker anomalies` CLI; 12 tests; CONFLICT after tick63 merge wave: needs rebase (post-#851 worker_actions.rs split) + migration renumber 17→18 (#848 took 17). Conductor commented with rebase instructions. |
| claude-workflow-dag | claude | IN_REVIEW | #857 | PR opened — sweep was premature, build-time delay not a crash. New `argo-workflow` crate (DAG/executor/expr/yaml) + `argo workflow validate/run/status/list-runs` + migration 18 (`workflow_run`). 27 unit + 2 integration + 1 doc-test green; clippy clean; all files under 500 LOC. |
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
| claude-spend-anomaly-alerts | claude | TODO | — | V1.1 reliability: hourly z-score on spend per worker per hour-of-day; |z|>4.0 inserts anomaly row + fires notify; `argo budget anomalies`; complementary to claude-anomaly-detector (action behavior) |
| codex-gateway-smoke-flake-fix | codex | MERGED | #858 | hardened gateway smoke fixtures: TOCTOU port-handoff fix, isolated ARGO_HOME setup, gateway boot serialization, schema/lock retry; clear RCA in PR body; gateway_smoke + gateway_smoke_writes flakes RESOLVED. Tick-60 known-flakes list will be updated to remove these entries.
| codex-connector-airtable | codex | MERGED | #856 | native Airtable MCP connector crate (8 files, 791 LOC); 7 methods (list_bases/tables/records, get/create/update/delete_record); PAT auth via AIRTABLE_PAT; filterByFormula passthrough on list_records; mirror gmail/asana crate pattern |
| codex-worker-portrait-cli | codex | TODO | — | waiting on claude-anomaly-detector merge; then render worker portrait markdown/json summary |
| codex-activity-feed-cli | codex | MERGED | #860 | `argo activity tail [--tenant --worker --since]` real-time audit feed; `argo activity since <duration>`; subscribes to AuditSink::subscribe(); --json NDJSON envelope; reconnect-on-break; 4 unit + 1 CLI smoke test; 695 LOC in 8 files
| codex-gdpr-delete-cli | codex | MERGED | #866 | V2.1 governance: `argo gdpr delete --subject <email>` walks 6 surfaces (memory/audit/skills/channels/workers/sessions); --dry-run default + --confirm with email retype; redaction (not deletion) preserves audit chain; gdpr_subject_erasure audit row; --bundle-out tarball; 8 unit + 1 CLI smoke; 1116 LOC across 9 files. Third attempt at this brief — first two crashed; this one delivered cleanly via the new looping prompt
| codex-connector-linear | codex | MERGED | #865 | native Linear MCP connector crate (8 files, 787 LOC); 7 methods (list_teams/list_issues/get/create/update/comment/list_workflows); GraphQL filter passthrough on list_issues; LINEAR_API_KEY bearer auth; mirror gmail/asana crate pattern
| codex-connector-monday | codex | MERGED | #868 | native Monday.com MCP connector crate (10 files, 839 LOC); 7 methods (list_boards/items/get/create/update_status/add_update/list_groups); API-token bearer; mirror crate pattern
| codex-connector-pagerduty | codex | IN_REVIEW | #867 | PR opened: native PagerDuty MCP connector crate with 7 REST tools, Token auth, approval-gated create/ack/resolve, unit + stdio wire coverage; branch merged current main and is mergeable |
| codex-connector-fireflies | codex | MERGED | #871 | native Fireflies.ai MCP connector crate (8 files, 774 LOC); 5 methods (list_meetings/get_meeting/get_transcript/search_transcripts/list_users); GraphQL passthrough on search; all autonomous-safe (read-only); FIREFLIES_API_KEY bearer; 13th native connector
| codex-audit-report-cli | codex | MERGED | #874 | V2.1 governance: `argo audit report --since/--until --format markdown|html [--output]`; 6 sections (summary stats / channel breakdown / top spenders / pending queue / chain integrity / known flakes); 892 LOC
| codex-doctor-watch-cli | codex | MERGED | #872 | V1.4: `argo doctor watch [--interval --clear]` continuous health monitoring; 206 LOC; reuses argo doctor --json envelope
| claude-policy-as-code | claude | TODO | — | V2.1 governance: declarative org policies in YAML at workspace/policies/*.yaml; applies_to + rules (action_kind / require_signatures / quiet_hours / spend_cap); resolution chain action→worker→roster→workspace→default; `argo policy list/show/validate/apply` |
| claude-data-classification | claude | TODO | — | V2.1 governance: DataClass enum (Pii/Phi/Financial/Secret/Public/Untagged) + tag table; memory writes accept class param; per-class retention + redaction rules in workspace config; `argo classify <kind>:<id> <class>` + list + rules |
| claude-checkpoint-resume | claude | CLAIMED | — | claimed by claude looping tab (iteration 2 after #882)
| claude-trajectory-training-export | claude | MERGED | #870 | V2.0 self-improvement: `argo trajectory export --since/--until --format anthropic|openai|jsonl [--filter passed|all]`; walks .skycot/outputs/<job>/actions/<id>/ bundles; PII redaction integrated; for fine-tuning data pipelines. 1052 LOC across 8 files. Brief was swept tick100, reclaimed + delivered cleanly under endless-loop prompt.
| codex-connector-clickup | codex | MERGED | #878 | native ClickUp MCP connector crate (8 files, 959 LOC); 8 methods (list_workspaces/spaces/lists/tasks, get/create/update_task, add_comment); CLICKUP_TOKEN bearer + CLICKUP_BASE_URL test override; mirror gmail/asana crate pattern; 14th native connector
| codex-tenant-cli-extensions | codex | MERGED | #877 | V1.x: `argo tenant rename/archive/transfer` w/ archived_at soft-delete + global audit row on transfer (old/new owner) + --include-archived list filter; migration 22__tenant_archived.sql; 816 LOC across 9 files. Brief was stale-swept tick110, then re-pushed by tab — eventual-consistency model holds again
| codex-genesis-refine-cli | codex | MERGED | #876 | V1.x: `argo genesis refine <id>` re-refines existing verticals via LLM (reuses #812 prompt + safety judge); 575 LOC across 7 files; --dry-run + --no-judge + --max-iterations; persists versioned old vertical kept as <id>-v<n>; new argo-vertical::version helpers. Conductor opened the PR — worker pushed branch but stalled before gh pr create
| claude-memory-tool-wire | claude | MERGED | #875 | wires memory.list/read/write through MemoryStore w/ vertical scoping; 539 LOC; replaces stale PR #775; vertical-bound writes via append_for_vertical (correctness fix); no uuid dep added; back-compat preserved for builtin_tools(cwd) call sites
| codex-connector-discord | codex | IN_REVIEW | #879 | PR opened: native Discord MCP connector with 7 REST tools, bot-token auth, approval-gated send/react, unit + stdio wire coverage; boundary note for pre-existing argo-cli test failures |
| codex-vertical-archive-cli | codex | BLOCKED | — | verification env failed repeatedly before PR: disk full / corrupted target metadata / focused compile killed |
| codex-cost-by-skill-cli | codex | CLAIMED | — | claimed by codex |
| codex-doctor-fix-cli | codex | TODO | — | V1.4: `argo doctor fix [--auto] [--issue <id>]` 3 categories (safe / confirmation / unsafe); audit-rowed per fix; safe set: missing dirs, stale lock, default policy template |
| claude-skill-quality-judge | claude | TODO | — | UNBLOCKED tick119: predecessor claude-cli-subcommands-split #882 just merged; cli_subcommands.rs is now 269 LOC with headroom for the SkillCmd::Quality variant. V2.0 self-improvement: per-skill 0-100 quality score (success_rate + latency + retry_count + p99); skill_quality_score table; nightly recompute tick; `argo skill quality/quality-list/quality-recompute`; --quality-min filter on skill list |

- status: claude-worker-self-exit after 3 PRs (reason: shipped #864/#870/#875; skill-quality-judge BLOCKED on cli_subcommands.rs 500-LOC ceiling — needs split refactor outside scope; iteration time degrading; remaining claude TODOs all touch the same ceiling or are large refactors. Fresh tab can pick up handoff-routing-rules / data-classification / quality-judge once cli_subcommands.rs is split.)
| claude-handoff-routing-rules | claude | IN_REVIEW | #883 | PR opened — declarative HandoffRoutingRule (RoutingEngine + RoutingRuleStore) + WhenClause (expected_outcome_contains / hour_of_day_utc_in / weekday_utc_in, AND-combined) + TargetSelector (SpecificWorker + RoundRobin fully implemented, SkillMatch + RoleMatch reserved for future skill/role registry lookup). HandoffWalker::with_routing_engine opt-in hook; routing_override consult before chain advance in process_overdue. `argo handoff-rules list/show/add/remove/test` (top-level, since HandoffCmd lives outside may-touch). 19 unit + 1 walker integration + 1 CLI integration. Same 2 pre-existing flakes as #880. |
| claude-worker-duty-scheduler | claude | TODO | — | V1.3 LOAD-BEARING (autonomous loop): Role.duties extended w/ cron+entry_action+enabled+paused_until; new argo-orchestrator::duty_scheduler tokio tick fires per-worker; fired duty creates Session w/ started_by_duty; AutoApprovalPolicy gate honored; `argo duty list/show/pause/resume/fire-now`. Mission claim "works autonomously" hinges on this. |
| claude-onboarding-flow | claude | TODO | — | V1.4: `argo onboard` 7-stage interactive walkthrough — welcome/workspace/credentials/connector-auth/first-worker/first-action/done; --non-interactive reads onboarding.toml; first-experience for non-developer operators |
| codex-credentials-management-cli | codex | TODO | — | V1.4: `argo cred set/list/clear/export/import` stores credentials in <workspace>/.argo/credentials.toml mode 0600; existing env-var paths get fallback to credentials.toml; replaces fragile env-var-only setup |
| codex-connector-authorize-cli | codex | TODO | — | V1.4: `argo connector authorize/list-status/revoke/test <kind>` triggers OAuth (gmail/gcal/hubspot/notion/salesforce) or token-prompt (others); validates via no-op API call; replaces manual env-var hunt |
| claude-installer-macos | claude | TODO | — | V1.4 deployment: Homebrew formula + signed .dmg + launchd plist via `argo daemon install/uninstall`; CI builds .dmg on release tag; replaces `cargo install`-from-source onboarding |
| claude-end-to-end-product-test | claude | TODO | — | V1.4: operator-perspective integration test — fresh ARGO_HOME → onboard --non-interactive → genesis worker → action emit → gate → channel notify → audit chain. Asserts SKYCOT works as a PRODUCT, not just a toolbox. Complement to existing claude-e2e-integration-test (#810/#832) which covers internals |
| claude-runtime-planner-trait | claude | TODO | — | V1.3 (from DREAMS_AND_RUNTIME_SWARM.md): new argo-runtime crate w/ Planner trait + LlmPlanner + TemplatePlanner; decomposes coarse goals into argo-workflow DAGs; `argo plan generate/show/execute`. Mission claim "delegation is the unit" needs this |
| claude-subtask-ledger-store | claude | TODO | — | V1.3: subtask_ledger table tracks Planner-emitted subtasks per parent goal; durability for restart-resume; CLI `argo subtask list/show/tree`; precursor to v2.0 dreams-plan-templates cross-pollination |
| claude-web-dashboard-mvp | claude | TODO | — | V1.4 operator UX: server-rendered HTML + HTMX dashboard at /dashboard/ — workers/sessions/approvals/spend tabs; SSE auto-refresh; inline approve/deny; bundled htmx.min.js (no CDN). Minimal, no React, no build step |
| codex-channel-imessage-bluebubbles | codex | TODO | — | V1.4 comms gap: iMessage approval channel + MCP connector via Blue Bubbles bridge (open-source Mac server exposing iMessage REST/WS); new argo-bluebubbles-client shared crate; BLUE_BUBBLES_URL+PASSWORD auth; closes the most-used messaging channel for Apple operators |
| codex-channel-voice-twilio | codex | TODO | — | V1.4 comms gap: voice approval channel — outbound Twilio call + TTS (Anthropic or Twilio Say) + DTMF capture (1=approve/2=deny/9=details); reuses existing Twilio creds from D-sms; HMAC token in callback URL; high-stakes-action ergonomics |
| claude-tui-realtime-dashboard | claude | MERGED | #881 | V1.4 operator UX: TUI overlay key `D` — workers/pending-approvals/audit-tail in 3 stacked panes (30/30/40 split); auto-refresh on existing 1s event-loop tick (no polling); 923 LOC across 10 files; mirrors claude-tui-h-stack-panels (#822) + claude-trajectory-replay-ui (#853) architecture
| claude-genesis-charter-conversation | claude | CLAIMED | — | claimed by claude looping tab |
| claude-mobile-approval-app-tauri | claude | TODO | — | V1.4 mobile UX: Tauri iOS+Android shell — 2 screens (pending approvals list, settings); thin client over existing /approvals/pending + /decisions/<token> routes; deep-link from push notifications; bundled, no CDN, app < 30 MB |
| codex-channel-whatsapp-twilio | codex | TODO | — | V1.4 comms: WhatsApp approval channel via Twilio (parallel to D-sms); reuses Twilio creds + HMAC validation; webhook /twilio/whatsapp/decision; ~400 LOC small extension |
| codex-vertical-publish-real-oci | codex | TODO | — | V1.x: replace mocked OCI registry index with real OCI Distribution Spec push via oci-distribution crate; auth chain --token > credentials.toml > docker config; works against ghcr.io / Docker Hub / ttl.sh / Zot; mock fallback preserved for back-compat |
| claude-skill-auction | claude | TODO | — | V1.3 multi-worker: handoff fallback when no rule matches — workers bid based on skill fit + TrustGrade + quality score; auction closes 5s; highest score wins, ties broken by lowest eta; auction_log + auction_bid tables for audit; complement to claude-handoff-routing-rules |
| codex-backup-restore-cli | codex | TODO | — | V1.x: `argo backup create/list/verify` + `argo restore <path> [--into --dry-run]`; tarball of <workspace>/.argo/; credentials excluded by default (security); sha256 in manifest.json inside tarball; refuses non-empty workspace restore |
| claude-docs-site-build | claude | TODO | — | V1.4 documentation: mdBook site at /book/ — Quickstart + How-to + Reference (auto-gen from argo --help) + Architecture (PROTOCOL.md + investigations) + Operations (RUNBOOK.md); GitHub Pages deploy on main push; mdbook-linkcheck in CI |
| codex-self-update-cli | codex | TODO | — | V1.4 deployment: `argo self-update [--check --channel --force]`; downloads from GitHub Releases matching platform; cosign/minisign signature verification mandatory; atomic temp-then-mv replacement; restarts daemon if running |
| claude-operator-feedback-channel | claude | TODO | — | V2.0 self-improvement: thumbs up/down per action via `argo feedback up/down/list`; TUI dashboard `+`/`-` keys; web dashboard inline buttons; feeds skill-learning loop (up = bonus weight, down = blacklist from skill-candidate emission) |
| claude-computer-use-native-macos-poc | claude | TODO | — | V1.5: NATIVE macOS computer-use PoC — design doc + working executor driving macOS Notes via accessibility APIs + Anthropic CU model. Bounded scope: 1 design doc + 1 PoC app + 1 smoke test. Closes the "browser-only CU" gap toward "any task a human can do on a Mac" |
| claude-multi-host-orchestrator-design | claude | TODO | — | V2.x: distributed conductor design + 2-host POC — design doc covering topology/sync/discovery/dispatch/split-brain + working /peer/sync endpoint with cross-host session visibility. Bounded scope (NOT full distributed system); follow-up briefs after this lands |
| claude-document-creation-executor | claude | TODO | — | V1.2 capability: new ExecutorKind::DocumentCreation — PDF (markdown→headless Chrome, reuse build-pdf toolchain) + Markdown + slides (reveal.js) + CSV (RFC 4180); writes to fs_root/outputs only; approval-gated by default |
| claude-voice-local-tui-whisper | claude | TODO | — | V1.4: `argo voice` TUI — local Whisper.cpp + cpal mic capture; grammar (approve/deny/details/next/quit); fuzzy worker-name match; wake-word "Argo" gates listening; pure-local privacy + offline-capable |
| claude-dreams-memory-summarize | claude | TODO | — | V2.0 (Dreams Integration A): replace argo-memory::summary LLM-call path with Claude Dreams API backend; nightly per-worker tick + on-demand `argo memory dream-now`; per-worker $5/day cap; anomaly-event guard prevents poisoning; feature-flag gates path |
| claude-dreams-skill-distillation | claude | TODO | — | V2.0 (Dreams Integration B): Dreams over cohorts of successful trajectories (≥3 with cosine sim > 0.7) → polished SkillCandidate body via dreams_distillation; mechanical fallback for small cohorts; shared daily cap with memory-summarize |
| claude-dreams-plan-templates | claude | TODO | — | V2.0 cross-pollination (claim4×claim2): Dreams over PLANNER COHORTS (subtask_ledger by parent_goal similarity) → LearnedPlanTemplate; TrustGrade ladder applies; TemplatePlanner consults templates first. The genuinely-novel proposal from DREAMS investigation |
| claude-vertical-finance-ar | claude | TODO | — | V2.2 demo verticals (4th): Accounts Receivable demo — AR worker reviews invoices, sends 7/14/30-day reminders via gmail, escalates at 45d, logs Stripe payment events, weekly aging report via document-creation-executor. Touches gmail+stripe+documents (broad capability proof) |
| codex-skill-quality-judge-cli | codex | TODO | — | V2.0 self-improvement (CLI surface): `argo skill quality/quality-list/quality-trend/quality-recompute` + ASCII sparkline of 30-day score history; `--quality-min` filter on `argo skill list`; companion to claude-skill-quality-judge architectural brief |
| claude-error-recovery-suggestions | claude | TODO | — | V1.4: `argo recover <session-id>` LLM-assisted post-mortem — walks audit chain, calls Anthropic with brief+manifest+chain+failure, returns structured suggestions (likely cause + 3 ranked fixes + command snippets); cached per (session, chain-hash); advisory only (no auto-apply) |
| claude-cli-subcommands-split | claude | MERGED | #882 | V1.x architectural prep: split crates/argo-cli/src/cli_subcommands.rs (was 500 LOC) into 3 focused sibling files — cli_subcommands_skill.rs (136 LOC) + cli_subcommands_provider.rs (49 LOC) + cli_subcommands_cron.rs (55 LOC); main file trimmed to 269 LOC; pure refactor, public API stable via re-exports; 4th post-sweep recovery this session. Unblocks claude-skill-quality-judge |

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
