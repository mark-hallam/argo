# Argo Coding Agent Instructions

Argo is a commercial AI-agent platform.

The architecture is:

1. Core Platform
2. Vertical Agent Templates

The core platform is already substantial. Do not rebuild it
unnecessarily.

Core platform includes: runtime, tools, provider routing, memory,
skills, audit, gateway, eval, forge, replay, MCP, RBAC, marketplace.

Vertical agents must be implemented through templates, not hardcoded
runtime forks. Vertical-specific behavior belongs in: manifests,
workflow definitions, prompt packs, memory schemas, tool permissions,
escalation policies, evaluation cases, UI/gateway metadata.

Never place receptionist/support/PR/sales-specific logic into core
crates unless the abstraction is reusable across multiple verticals.

## Priorities

1. reliability
2. observability
3. repeatability
4. security
5. commercial usability
6. extensibility

## Maintain

- single-binary architecture
- Rust 1.75 compatibility
- <500 LOC per file
- additive migrations (see `docs/skycot/operator/migrations.md`)
- tests for every feature
- JSON output for CLI list/show/status commands

## Core-platform invariants

Five non-negotiable invariants:

1. Polyglot core (Rust core + JSON-RPC plugin ABI; plugins are subprocesses)
2. Multi-tenant by construction (tenant_id plumbed through every primitive)
3. Capability-based security with first-class RBAC
4. Provider manifests, not adapters (TOML, not Rust)
5. MCP-first integration (lean core, currently 15 in-core tools;
   new tools default to MCP unless they require direct AgentDeps
   or runtime access)

CI rejects PRs that violate any of these:

- No file > 500 LOC
- No fn > 80 LOC
- No struct impl block > 200 LOC
- No `unsafe` Rust outside marked FFI shims
- No `unwrap`/`expect` outside `#[cfg(test)]`
- No global mutable state. AgentDeps is the only injection point.
- Every public Rust API has rustdoc with at least one example

When in doubt:

- Subtract before adding. Idiot Index is solution complexity ÷ problem complexity.
- Question the requirement. If something doesn't make sense, raise it; do not invent.
- Ship the 80% solution. Working software beats the perfect solution that arrives late.

Plan documents, in order of operative precedence:

- `docs/plan/argo-build-plan.md` (v0.2) — original architecture
  reference for the core-platform layer. Still valid for that scope,
  but no longer the operative phased plan.
- `docs/build-plan/BUILD_PLAN_V11.md` — operative phased plan for
  backend tracks (v1.1 production hardening through v2.2 demo
  verticals). Authoritative for backend scope; product-surface /
  frontend work moved to PRODUCT_ROADMAP.md as of 2026-05-11.
- `docs/build-plan/PRODUCT_ROADMAP.md` — operative plan for the
  post-MVP product-surface program (Tracks A–G: UX research →
  design language → wireframes → frontend implementation +
  parallel claim-4 backend fill-ins + claim-5 finish). Read this
  before picking up any UI / UX / persona / journey work.
- `docs/build-plan/BUILD_STATUS.md` — the live ledger (rows + state
  transitions, edited every tick).
- `docs/build-plan/MISSION_BACKLOG.md` — canonical source of
  un-shipped work, status-tagged for queue replenishment.
- `docs/skycot/ux-research/` — persona docs + journey maps. The
  design brief for all frontend implementation. Referenced from
  PRODUCT_ROADMAP.md Track A.

The vertical-template system is documented in its own design doc
once Phase 1 lands.

**🛑 IF YOU'RE A FRESH AGENT JOINING POST-2026-05-25**: the swarm is currently
**PAUSED** (`/tmp/argo-stop` in place; supervisor PID 3791 + 4 worker
loops + claude/cargo subprocesses all killed in the 2026-05-25 interactive
recovery session). Do NOT relight workers without operator approval. The
older [`quota-conservation-handoff-2026-05-23.md`](docs/skycot/operator/quota-conservation-handoff-2026-05-23.md)
doc is now ~2 days stale — its specific PR lists were corrected the same
day, and substantial merge activity (66 PRs + 1 closed-as-dup in the
2026-05-25 session) has moved main far ahead of the state it describes.

**Fresh agents (any time): start at [`docs/build-plan/RESUME.md`](docs/build-plan/RESUME.md)** — a single-page primer pointing you at the
5 files to read (in order) to pick up the swarm. The
`docs/build-plan/BUILD_STATUS.md` header (first ~90 lines) is the
live ledger — operator amendments (queue thresholds, agent-family
availability, paused streams, active directives) live there and
override every other doc. Memory and template defaults are not
authoritative; the live ledger is.

**Doc-staleness gotchas (read before trusting any text in this repo)** —
the following entry-point docs are KNOWN STALE; check the date in the
section before relying on what they say:

- **CLAUDE.md "Status" block below** refreshed 2026-05-25 after the interactive merge-train session shipped 66 PRs + closed 1 as duplicate. Prior corrections (2026-05-24) about the 8 LIVE-test wave PRs being MERGED (#2088, #2092, #2109, #2110, #2112, #2113, #2133, #2137) remain accurate.
- `docs/skycot/operator/quota-conservation-handoff-2026-05-23.md` — ~2 days old; superseded by 2026-05-25 session activity. Treat as historical context only.
- `docs/build-plan/RESUME.md` "Last sweep" claims 2026-05-20; flagged for refresh.
- `docs/build-plan/BUILD_PLAN_V11.md` — refreshed 2026-05-24 via PR #2213, now points at current mission gate; operative for backend track only; product-surface in `docs/build-plan/PRODUCT_ROADMAP.md`.
- `docs/build-plan/PRODUCT_ROADMAP.md` — refreshed 2026-05-24 via PR #2216.
- `docs/build-plan/CONTRIBUTING_AGENT.md` — refreshed 2026-05-24 via PR #2215.
- `docs/skycot/demos/v1-mission-mvp-runbook.md` — consolidated 2026-05-24 via PR #2205 (canonical 6-scene runbook).
- `docs/plan/argo-build-plan.md` (v0.2) — **HISTORICAL.** Not the operative plan. Reference only.
- `docs/build-plan/MISSION_BACKLOG.md` (2026-05-19) — items marked `[NEW]` may have shipped; cross-check against `git log`.
- The ~1,015 brief files in `docs/build-plan/streams/` — many shipped or stale; **always check the ledger row in BUILD_STATUS.md** for the brief's current status before claiming.

**Source of truth, in order**:
1. `git log -20 origin/main` (what actually shipped — the merge-train of 2026-05-25 moved main substantially; trust git over any narrative doc)
2. `docs/build-plan/BUILD_STATUS.md` header + ledger rows
3. `docs/skycot/MISSION.md` (operator-locked LIVE criteria, latest 2026-05-24)
4. `docs/skycot/agents/heartbeats/conductor.json` (latest tick state)
5. `docs/skycot/agents/SHARED.md` (recent evaluator/conductor/supervisor handoffs)
6. The 2026-05-23 quota-conservation handoff doc (historical context only)

PR title format: `<phase> — <section> — <one-line summary>`

## v1.0 strategic decisions (operator-locked 2026-05-14)

These are not architectural preferences; they are operator decisions that scope v1.0. Read before authoring briefs or designing implementations.

- **macOS is the v1 demo path for computer-use.** `argo-executor-computer-use-macos` is the only OS-native executor v1 hardens. E2B, Linux, and Windows executors are DEFERRED-v1.x — they stay shipped, just not v1-polished. See `streams/claude-defer-non-macos-computer-use.md`. Demo recording `docs/skycot/demos/v1-mission-mvp.mp4` is macOS.
- **Enterprise focus, not consumer.** SKYCOT serves enterprise customers. MCP connector priority and authoring decisions reflect this. The 4 consumer/redundant connector stubs (signal, telegram, mailgun, resend) are being deleted; the 3 enterprise stubs (apollo, zoominfo, zoom) stay as DEFERRED-v1.x placeholders. When in doubt about a connector, ask "is this enterprise?"
- **Type canonicalization mostly done, surfaces now contract-locked.** Phase A audit found 11+ duplicate `pub struct/enum` definitions across crates (worst: `AuditRow` in 4 crates, real data-integrity risk). All 4 canonicalization briefs + 1 CI-lint brief have shipped. As of 2026-05-25, the 16-PR eval surface-pin wave (`tests/surface_contract.rs` per crate) has locked public surfaces for argo-runtime / argo-acp / argo-forge / argo-core / argo-config / argo-security / argo-siem / argo-telemetry / argo-eval / argo-marketplace / test-cassette / e2b substrate / credentials / process-registry / team-headline. Continue: do not introduce new parallel `pub` types until the canonical home is established. Re-export from `argo-types` is the preferred pattern when in doubt.
- **Skill-library worker load: SHIPPED** (#1682 + cross-tenant negative test #1702). `LayeredLookup::with_library` is wired at `crates/argo-agent-runner/src/turn_helpers.rs:65`. MISSION.md success scenario #2 is now runtime-live.
- **Migration slot collisions: GUARDED** (post #1735). `scripts/check-migration-slots.sh` runs in CI; `crates/argo-store/src/migrations/registry.rs:21` calls `duplicate_slot_validator::validate_no_duplicate_slots()` at boot. Last admin-wave-6 added migrations 109-114 cleanly.

The full audit reports live in `docs/skycot/audit/`:
- `2026-05-14-phase-A` through `phase-D` — original 5-phase deep audit
- `2026-05-15-codex-full-architecture-audit.md` — 18-finding audit
- `2026-05-16-one-shot-architecture-audit.md` — F-01 through F-14
- `2026-05-17-readonly-codebase-audit.md` — codex audit (13 findings)
- `2026-05-17-claude-conductor-audit.md` — claude audit (13 findings + closure verification)

**Status 2026-05-25 (post interactive merge-train session):** Mission state per MISSION.md operator-locked LIVE criteria + evaluator rounds 1-54, post the 2026-05-25 merge-train (66 PRs shipped, 1 closed-as-duplicate):

- Claim 1 (Teammate, not tool) — 🟢 **LIVE** (regression test passes cold per round-19; SQL tenant gate for team-headline now wired via #2261)
- Claim 2 (Delegation is the unit) — 🟢 **LIVE** (delegate-task fallback-chain inheritance now wired in live runner dispatch via #2256; evaluator round-54 PRE-MERGE PASS confirmed cold cargo; runtime is now carrying the active handoff chain through `ToolContext.current_handoff_id`)
- Claim 3 (Platform builds workers) — 🟢 **LIVE** (evaluator round-54 addendum: cold cargo 2/2 PASS, 169min wall under load 183; verdict 🟡→🟢; LIKELY-LIVE→LIVE flip recommended; #2089 + #2106 + #2109 wire complete)
- Claim 4 (Skills shared + learned) — 🟢 **LIKELY-LIVE** (#2092 + #2110 merged; criteria operator-locked round 19; cold rerun of `claim_4_live_install_invoke` still recommended to confirm)
- Claim 5 (Any-PC execution) — 🟡 **LIKELY-LIVE-pending-perms** (#2112 merged + evaluator round 51 confirmed claim-5 substrate 47/47 cold cargo PASS; #2231 added cu(macos) screenshot compression config; #2264 added routing decision tenant-scope; #2218 added document executor bundle tenant-segment; still needs operator macOS perms on argo binary + cold rerun of `claim_5_live_mcp_and_cu`)
- Claim 6 (Multi-channel + multi-tenant) — 🟢 **LIKELY-LIVE** (#2113 merged; cross-tenant negative pins shipped via #2257; inbound SMS approval route via #2224; Discord interaction signature verify via #2222; voice DTMF audit via #2208)
- v1.0 demo MP4 — ❌ NOT RECORDED (operator-only artifact)

**🚀 2026-05-25 INTERACTIVE MERGE-TRAIN — 66 PRs MERGED + 1 CLOSED-AS-DUP** in one operator-driven recovery session:

- **Conductor LOC unblock** (1): #2265 (conductor-authored mechanical extraction of `routes_approvals_json_helpers.rs` sibling — the brief had been zombie-claimed 3× over 6h without shipping; conductor took over inline and shipped, unblocking the entire merge queue)
- **Evaluator pre-verified cluster** (5): #2244 #2248 #2256 (claim-2 LIVE wire) #2257 (claim-6 regression-pin) #2259 (M22 Status Reporter pack — Patrick joins Peta)
- **Track P (v1.0 demo headline)** (2): #2214 (P-01 truth-boundary capability schema) + #2209 (P-03 voice-io crate) — operator-locked priority-1; unblocks P-02/P-04/P-05/P-06/P-07/P-08
- **Tenant-binding security wave** (6): #2218 #2245 #2246 #2260 #2261 #2264 — post-#2167 sweep complete; SCIM mutations, elevated-view grant revoke, skill query, document executor bundles, team-headline, routing decisions all tenant-bound
- **Docs batch** (4): #2205 #2213 #2215 #2216 — v1 demo runbook + BUILD_PLAN_V11 + CONTRIBUTING_AGENT + PRODUCT_ROADMAP all refreshed against current mission gate
- **Web demo-blockers** (3 first-pass + 1 rebased): #2241 #2242 #2243 #2247 — SPA workers/audit/workflows/env-var fixes ship; **⚠️ post-merge curl-check needed on #2242 (NEXT_PUBLIC_ env var rename) per `feedback_styled_jsx_requires_use_client.md` — landing/marketing pages must not 500**
- **Ops lint wave** (11): #2201 #2202 #2203 #2219 #2223 #2239 #2249 #2250 #2251 #2252 #2254 — conductor singleton preflight guard, heartbeat UTC stamp lint, unread-handoffs action lint, ledger-unrowed-stream lint, build-status stale-claim lint, open-PR overlap triage, vertical-regression gate, brief-file existence check + more
- **Eval surface-pin wave** (16): #2211 #2221 #2226 #2227 #2228 #2229 #2230 #2232 #2233 #2234 #2235 #2236 #2237 #2238 #2240 #2263 — all critical-crate public surfaces locked via tests/surface_contract.rs (argo-runtime, argo-acp, argo-forge, argo-core, argo-config, argo-security, argo-siem, argo-telemetry, argo-eval, argo-marketplace, test-cassette, e2b-substrate, credentials migrate-on-read, process-registry escape-hatch removal, team-headline)
- **Misc CLEAN** (15): #2204 #2206 #2207 #2208 #2210 #2212 #2220 #2222 #2224 #2225 #2231 #2255 #2258 #2262 #2266 — packs marketplace listing, Skills mobile dark-mode, SPA error pages, PDF .staging cleanup, Discord sig verify, SMS inbound route, storybook coverage, cu(macos) compression, model=latest runtime, ESLint plugin, preventive split, tenant vertical discovery + more
- **Rebase wave** (3): #2253 (CONTRIBUTING_AGENT.md conflict resolved) + #2247 (workflows.test.ts conflict resolved — kept #2242 mock-bypass test + replaced legacy multi-endpoint test with #2247 never-hits-verticals test) + #2217 (document-csv executor: combined #2218 ensure_document_bundle_dir API with #2217 tenant_id threading)
- **Closed as duplicate** (1): #2200 closed-of-#2224 (both wired inbound SMS approval; same file `routes_channels_sms.rs`)

**Operator authorizations granted this session** (saved to global memory):
- `--skip-clippy` on `conductor-pre-merge-gate.sh` authorized for: (a) evaluator-pre-verified PRs (cold cargo PASS within ~6h on PR head SHA), (b) conductor-authored mechanical-extraction PRs where conductor has personally `cargo check -p <crate>` exit-0 on the PR head, (c) Track P PRs (operator-locked v1.0 demo scope), (d) tenant-binding security wave + doc-only PRs (operator categorized-cluster directive "go merge cluster those")
- Diagnostic discovery: the prior 5 consecutive scheduled-tick merge-deferrals (T18-06 / T19-08 / T20-08 / T21-11 / T22-10) were rooted in macOS jetsam killing cargo under memory pressure (workers' shared `/tmp/argo-shared-target` grew to 328 GB; tmpfs-like file-cache pressure pegged free RAM at ~30 MB). Fix: kill workers + remove 9.8 GB zombie + 328 GB shared cache; use real-disk target dirs (NOT `/tmp`) for any future cargo runs while workers are running.

**Open PR count after session: 0.** No DIRTY, no stuck, no stale. First time in this swarm cycle.

**Earlier history (preserved for context):**
- **2026-05-23 LIVE-test chain-merge wave** shipped 8 PRs 08:34-12:42 UTC: #2088 + #2092 (substrate); #2109 + #2110 + #2112 + #2113 + #2137 (the 5 claim-N-LIVE regression tests); #2133 (conductor block-history guard).
- **2026-05-23 tick-91 chain-merge wave** shipped 8 unrelated PRs in one tick — #2089 WorkerSessionRunner (claim-3 broken-wire closed), #2090 per-tenant master key, #2098 workflow-runs pagination, #2100 orchestrator duty-store tenant-args, #2101 skill-ab-test tenant-id (schema bumped to v79), #2102 rebase-cluster, #2103 mcp-connector token-cache, #2105 memory per-worker-scope, #2106 workflow-execute route.
- **2026-05-18 state:** Magic-link signup+verify LIVE on main via #1823. CI gates GREEN locally (parallel-pub-types-enforce + loc-budget + prepared-statements + unsafe-safety + audit-sink-tenant-guard). All 11 disabled GHA workflows intentionally kept disabled per `.github/workflows/DISABLED_WORKFLOWS_POLICY.md` (v1 strategy: admin-merge bypass while GHA billing is operator-blocked). Codex pool at full Pro 20x. SQLCipher 4-PR sequence MERGED.
- PR #2131 (mission-orchestrator-cross-tenant-handoff-audit-kind) MERGED 2026-05-24T07:11Z.

**Remaining for v1.0 launch (2026-05-25 updated):**
- (a) **agent**: cold-rerun the remaining claim-N-LIVE regression tests on current main — claims 2/3/5 now have evaluator-round-51/53/54 PASSes; claim-4 still needs `claim_4_live_install_invoke` cold rerun; claim-6 still needs `claim_6_live_cross_tenant` cold rerun
- (b) **agent**: restart gateway (`launchctl list com.skycot.gateway` shows last exit 0; no process running; nothing listening on port 8088)
- (c) **operator-only**: grant macOS permissions on `argo` binary (Accessibility + Screen Recording) for claim-5
- (d) **operator-only**: record demo MP4 per `docs/skycot/demos/v1-mission-mvp-runbook.md`
- (e) **agent**: Track P briefs P-02 / P-04 / P-05 / P-06 / P-07 / P-08 still need worker implementation (operator-locked Track P scope per memory `track_p_v1_mvp_scope`). P-01 (#2214) + P-03 (#2209) shipped today and unblock the cascade. ~6-8 weeks total on full swarm per memory `pitch_agent_v1_demo`.
- (f) **agent (post-relight)**: curl-check landing/marketing pages after #2242 `NEXT_PUBLIC_` env-var rename — must not 500 (per memory `styled_jsx_requires_use_client`)

**Items (a)/(b)/(e)/(f) are agent-doable but the swarm is currently PAUSED.** Items (c)/(d) are operator-only.

**To relight the swarm (post tier-1+2 memory-pressure fixes 2026-05-25):**
1. `rm /tmp/argo-stop ~/.argo/stop` (clears both transient + persistent halt signals — added in commit `a352b4bfb`)
2. `launchctl load ~/Library/LaunchAgents/com.skycot.worker-supervisor.plist` (will auto-load on next login via `RunAtLoad=true`)
3. The plist now launches `worker-supervisor.sh 2` (was 4) with `CARGO_BUILD_JOBS=4` env var to cap rustc parallelism

**Memory-pressure safeguards now in place** (kick in automatically post-relight):
- Supervisor refuses to spawn a new worker iteration when `vm_stat` free+speculative < 1 GB
- Exponential backoff: 5 min → 15 min → 30 min cap on sustained pressure
- At the 30-min cap, supervisor appends a `[supervisor-ram-pressure]` line to `docs/skycot/agents/SHARED.md` so the operator sees sustained pressure in the next conductor tick's handoff scan
- Tunable via `RAM_PRESSURE_THRESHOLD_MB` env var (default 1024)
- Workers still use `CARGO_TARGET_DIR=/tmp/argo-shared-target` (NOT changed — that's a Tier-2 follow-up; the per-cargo job cap + memory gate should keep the shared dir bounded by reducing peak concurrent compile load)

## Project rename — Argo → Skycot

The project will rename from Argo to **Skycot** (domain: skycot.com).
Continue using `argo` everywhere for now (binary, crates, env vars,
`~/.argo/`, `argo.toml`). Do not start renaming proactively.

## Multi-agent swarm protocol

**🛑 The swarm is currently PAUSED as of 2026-05-25** (`/tmp/argo-stop`
in place; supervisor + 4 worker bash loops + claude/cargo subprocesses
all killed during the 2026-05-25 interactive recovery session). Do NOT
relight workers without operator approval. New sessions opened in this
repo while the swarm is paused are operator-driven interactive sessions,
NOT autonomous swarm workers — read what the operator typed, do not
auto-claim briefs.

When the swarm WAS active (and once it's relit), any session opened
in this repo is presumed to be a worker or conductor unless explicitly
invoked otherwise. Follow [`docs/build-plan/RESUME.md`](docs/build-plan/RESUME.md)
to pick up.

[`docs/build-plan/SWARM_PROTOCOL.md`](docs/build-plan/SWARM_PROTOCOL.md)
is the source of truth for: iteration sequence, hard rules,
verification, stop conditions, manager-worker mode, and graceful
exit. Re-read it on every iteration — the ledger on `origin/main` is
authoritative, not memory.

Companion docs:

- [`docs/build-plan/PROTOCOL.md`](docs/build-plan/PROTOCOL.md) — architecture and anti-patterns.
- [`docs/build-plan/PROTOCOL_LOOP_PROMPT.md`](docs/build-plan/PROTOCOL_LOOP_PROMPT.md) — conductor tick prompt.
- [`docs/build-plan/CONTRIBUTING_AGENT.md`](docs/build-plan/CONTRIBUTING_AGENT.md) — short worker companion.
- [`docs/build-plan/KNOWN_FLAKES.md`](docs/build-plan/KNOWN_FLAKES.md) — pre-existing test flakes.
- [`docs/skycot/DRIFT_PREVENTION.md`](docs/skycot/DRIFT_PREVENTION.md) — drift patterns, brief pre-flight checklist, audit cadence, unfucking severity ladder.

Genuinely ad-hoc sessions (invoked outside the swarm context — e.g.
the operator running a one-off cleanup) can ignore the build-plan
layer; the core rules above still apply.

## Drift prevention (conductor + worker)

The audit chain on 2026-05-13/14 surfaced a ~24% intent-drift rate in
conductor-authored briefs (new parallel surfaces alongside ones that
already shipped) plus systemic dead-code drift (subsystems landed
without callers, 17 connector crates with zero workspace consumers,
duplicate `SessionId`/`JudgeVerdict`/`Role` types across crates). The
full playbook is [`docs/skycot/DRIFT_PREVENTION.md`](docs/skycot/DRIFT_PREVENTION.md).
Load-bearing rules, inline so they cannot be missed:

- **Grep before write.** Before declaring any new pub struct, enum, or
  type alias in a brief, run `rg 'pub (struct|enum|type) <Name>' crates/`.
  If a same-named or near-same-named surface exists, the brief either
  extends it, supersedes it, or carries a one-line justification.
- **Wire or defer.** Every new public surface in a brief names its
  consumer. If the consumer doesn't exist yet, the brief either
  includes the wire-up or is paired with a follow-up brief in the same
  batch. "Land scaffold now, wire later" is the failure mode.
- **No parallel abstractions.** Two structs with the same name and
  overlapping responsibilities in different crates is a bug, not a
  feature. Past examples that became P1s: `McpConnectionPool` (in
  `argo-agent-runner`) parallel to canonical `McpClientPool` (in
  `argo-mcp`) without per-tenant keying → cross-tenant credential
  leak.
- **Freshness gate.** Briefs declare `Authored against: origin/main
  <SHA> on <YYYY-MM-DD>` within 24h of authoring, plus
  `Validate before claim: bash scripts/check-brief-freshness.sh
  <brief-id>`. Stale briefs are refused at claim time.
- **Recovery briefs re-validate intent.** Before authoring
  `claude-rebase-pr-<N>`, confirm the underlying intent is still
  load-bearing on current `origin/main`. If the bug or feature has
  already been fixed by another PR, close the original PR with
  STALE-FIXED instead.
- **Ledger drift sweep every tick.** Conductor runs
  `scripts/find-stale-claim-drift.py --quiet` every tick. Zero drift
  rows is the steady state.
- **Lint briefs before commit.** `scripts/lint-brief.sh <brief-id>`
  validates a brief's freshness header, `Validate before claim:`
  line, parallel-surface declarations against `crates/`, and
  parallel-name brief-id collisions against `streams/` + the ledger.
  A pre-commit hook (install via `scripts/install-brief-hooks.sh`)
  runs the lint on every staged brief in WARN mode; flip to enforce
  with `git config skycot.brief-lint enforce`.
