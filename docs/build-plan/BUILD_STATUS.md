# Build status — live ledger

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
| H-audit-query-extensions | codex | CLAIMED | — | claimed by codex |
| H-channel-health-checks | codex | MERGED | #828 | NotifyChannel connectivity pings (Postmark/Slack/Twilio/Expo); doctor_channels.rs (315 LOC) + 5s timeout per check |
| H-shared-team-cli | codex | BLOCKED | — | adding MemoryCmd variant requires touching handlers/memory.rs exhaustiveness, but brief forbids any other handler |
| H-vertical-list-cli | codex | CLAIMED | — | claimed by codex |
| H-worker-list-cli | codex | BLOCKED | — | brief names missing handlers/worker.rs; required WorkerCmd changes need worker_actions.rs dispatch outside may-touch |
| H-genesis-list-templates | codex | IN_REVIEW | #830 | PR opened |
| claude-cu-skill-learn-wire | claude | MERGED | #807 | CU success → maybe_propose → SkillCandidate via H-4c → SkillStore::propose_for_review; new bridge module + 10 unit tests; learning-loop errors swallow + log so the action never fails |
| claude-tui-h-stack-panels | claude | MERGED | #822 | 3 ratatui overlays (O sessions / H handoffs / B budget) reading via gateway HTTP; renders "Gateway unreachable" cleanly while H-orchestrator-routes pending; 14 new tests (9 snapshot + 5 input) |
| claude-e2e-integration-test | claude | MERGED | #810 | 6/9 steps execute end-to-end; 3 SKIPPED with TODO comments (action emit + channel notify + transition — see claude-action-record-identity-plumbing + claude-e2e-test-step-completion follow-ups) |
| claude-genesis-llm-refinement | claude | MERGED | #812 | opt-in --refine flag with Anthropic call + JSON schema validation + safety judge re-runs on refined actions; 9 unit tests offline |
| claude-skill-success-metric | claude | MERGED | #813 | SuccessMetric (confidence + trajectory_length + duration + final_state_score) + ProposalThreshold defaults (≥0.8 / ≤25 steps / ≤600s); CU executor computes from real trajectory; legacy f64 API preserved |
| claude-worker-instance-policy | claude | MERGED | #815 | Worker::resolved_policy(workspace_default) + argo worker update --auto-approval-json; gate hookup deferred (waits on claude-action-record-identity-plumbing for worker_id on ActionRecord) |
| claude-orchestrator-handoff-walker | claude | MERGED | #825 | HandoffWalker::sweep_once + tokio sweeper at gateway boot; SchemeRoutingNotifier; 12 tests; makes orchestrator genuinely autonomous |
| claude-skycot-runbook | claude | MERGED | #816 | 944-line operator runbook (`docs/skycot/RUNBOOK.md`) covering all 10 brief sections: install → bootstrap → genesis → approvals → first action → skill learning → handoff → audit → next steps |
| claude-action-record-identity-plumbing | claude | MERGED | #829 | ActionRecord.worker_id threaded through 19 files; emit_actions_from_text gains worker_id param; FOUNDATIONAL — unblocks e2e-test-step-completion + worker-instance-policy gate hookup |
| claude-e2e-test-step-completion | claude | TODO | — | replenish-3: flip the 3 SKIPped steps in #810 to real assertions once identity plumbing lands; flips v1 demo gate from yellow (6/9) → green (9/9) |
| claude-marketplace-vertical-publish | claude | MERGED | #821 | `argo vertical publish/pull` via OCI; build_vertical_bundle + unpack_vertical_bundle; hermetic round-trip + tamper-reject tests; --rename for conflict resolution |
| claude-realtime-audit-stream | claude | CLAIMED | — | claimed by claude |
| H-eval-runner-cli | codex | TODO | — | replenish-codex3: `argo eval run --vertical <id> [--score]` runs eval seed against worker pipeline |
| H-tenant-cli | codex | TODO | — | replenish-codex3: `argo tenant list/info/create` for tenant management |
| H-handoff-cancel | codex | TODO | — | replenish-codex3: `argo handoff cancel <id>` operator override + Cancelled lifecycle variant |
| H-skill-export-import | codex | TODO | — | replenish-codex3: `argo skill export/import` for offline skill distribution |
| claude-action-cancel-flow | claude | TODO | — | replenish-claude3: `Cancelled` state in V3 action lifecycle + `argo worker actions cancel` CLI |
| claude-audit-replay | claude | TODO | — | replenish-claude3: `argo audit replay <session_id>` walks chain in seq order; pretty/raw formats |
| claude-skill-conflict-resolution | claude | TODO | — | replenish-claude3: workspace-vs-library skill conflicts UI: list/show diff/resolve |
| claude-eval-streaming | claude | TODO | — | replenish-claude3: streaming per-fixture progress for `argo eval run` |

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
