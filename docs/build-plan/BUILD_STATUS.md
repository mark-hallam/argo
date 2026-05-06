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
| F-memory-shared-team | codex | BLOCKED | — | full cargo test fails in argo-cli chain_result_propagation outside this brief's file boundary |
| H-orchestrator-persist | codex | CLAIMED | — | claimed by codex |
| H-spend-persistence | codex | CLAIMED | — | follow-up: wire H-2 gate to read today's spend + record charge after AutoApprove (currently passes SpendSnapshot::zero); deferred from H-2 PR #800 |
| H-orchestrator-cli | codex | TODO | — | tier-2 follow-up: `argo session start/list/show` + `argo handoff list/show` subcommands (waits on H-orchestrator-persist) |
| H-orchestrator-routes | codex | TODO | — | tier-2 follow-up: gateway HTTP endpoints for sessions + handoffs (waits on H-orchestrator-persist) |
| H-skill-library-routes | codex | TODO | — | tier-2 follow-up: gateway HTTP endpoints `GET /skills` + `GET /skills/<name>` exposing the H-4b LayeredLookup (independent) |
| H-budget-cli | codex | TODO | — | tier-2 follow-up: `argo budget show <worker_id>` + `argo budget recent` (waits on H-spend-persistence) |
| H-doctor-extensions | codex | TODO | — | tier-2 follow-up: 4 new `argo doctor` checks for the H-stack (independent; degrades gracefully if persist tables missing) |
| H-metrics | codex | TODO | — | tier-2 follow-up: Prometheus counters for auto-approval decisions + sessions + handoffs + evaluation latency histogram (independent) |
| migration-consolidation-test | codex | TODO | — | tier-2 follow-up: integration test verifying the full migration chain applies cleanly on fresh DB (independent; high-leverage regression guard) |
| demo-marketplace-seller-vertical | codex | TODO | — | tier-2 follow-up: full v1 demo vertical end-to-end (Genesis-shape manifest + 5-item inventory + auto_approval policy + eval seeds); pure content, no Rust |
| claude-cu-skill-learn-wire | claude | TODO | — | replenish: wire H-4c skill-learning into argo-executor-computer-use post-completion hook; closes the learning loop the brief stubbed |
| claude-tui-h-stack-panels | claude | TODO | — | replenish: 3 new ratatui overlays (sessions / handoffs / budget); waits on H-orchestrator-persist for content but degrades cleanly without |
| claude-e2e-integration-test | claude | TODO | — | replenish: end-to-end integration test exercising Genesis → V3 ledger → auto-approval gate → channel notify → executor → audit; the v1 demo gate |
| claude-genesis-llm-refinement | claude | TODO | — | replenish: opt-in `--refine` flag on `argo genesis create` that adds an Anthropic refinement pass after the deterministic pipeline (safety judge re-runs); deferred from G-genesis PR body |

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
