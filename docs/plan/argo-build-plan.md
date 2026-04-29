# Argo — Master Build Plan (v0.2)

**Codename:** Argo (the ship of agents)
**Working title:** A better Hermes Agent
**Version:** v0.2 (incorporates 14 user-validated amendments + Argo Forge + time-travel debug + workspace marketplace)
**Date:** April 29, 2026
**Owner:** Mark Hallam (Acmeco)
**Audience:** Coding agents, contributors
**License target:** MIT
**Companion docs:** `argo-day-0-kickoff.md`, `argo-vs-hermes-gap-analysis.md`

> "Working software is the only measure of progress." Every section ends in **Acceptance** — falsifiable, machine-checkable definition of done. No exceptions.

> **Changes from v0.1:** v0.2 incorporates the 14 amendments and 3 high-priority "beyond Hermes" opportunities surfaced by the user-research gap analysis. Net effect on schedule: +4-5 weeks to v1.0 (24 weeks total). Net effect on scope: Argo ships as a Hermes-superset, not a Hermes-subset, in v1.0. Trade-offs documented in §27.5.

---

## 0. Reading guide and scope

Sections 1-3 set the contract. Sections 4-12 specify the system in fine detail. Sections 13-17 specify the polyglot plugin layer. Sections 18-25 specify the surfaces (CLI, gateway, MCP server, web). Sections 26-27 specify build/release and the phased roadmap. Section 28 specifies coding-agent dispatch. Sections 29-30 specify risks and stop conditions.

Every code block prefixed `// SPEC` is normative. Every code block prefixed `// EXAMPLE` is illustrative.

If a coding agent reads this and has a clarifying question, the spec has failed; raise an issue against this document, do not invent.

**v0.2 NEW SUBSYSTEMS (added since v0.1):**

- **Cost-router** (`argo-routing`) — tiered model selection by classified prompt complexity
- **Judge** (`argo-judge`) — external-LLM adjudication of skill creation/refinement
- **Forge** (`argo-forge`) — DSPy/GEPA-style automated skill optimization, ICLR-2026 Oral lineage
- **Replay** (`argo-replay`) — time-travel debug; replay any session against different models/tools/workspaces
- **Marketplace** (`argo-marketplace`) — workspace bundles as OCI artifacts; push/pull whole agent identities
- **ACP server** (`argo-acp`) — VS Code/Zed/JetBrains integration with multi-harness support

---

## 1. Vision and non-negotiables

### 1.1 Mission

Build the agent framework that **a single engineer can fully audit on a weekend** and that **a 200-person company can deploy to production** without writing the multi-tenant layer themselves.

### 1.2 The five non-negotiable invariants

1. **Polyglot core.** Rust core + JSON-RPC plugin ABI. Plugins are subprocesses. A bad plugin cannot crash the agent. Plugin authors choose their own language.
2. **Multi-tenant by construction.** Every primitive (session, memory, skill, credential, tool call, audit record, **routing decision, judge outcome**) carries `tenant_id` from line one. Single-user is `tenant_id = "default"`. **One tenant supports N named agents** with isolated personas and toolsets (Amendment 5).
3. **Capability-based security with first-class RBAC.** Every tool, skill, plugin, and provider declares an explicit capability set. The core enforces capabilities at call time. Roles (Owner/Admin/User/Guest) are a top-level configuration concept consumed by gateway, CLI, and ACP.
4. **Provider manifests, not adapters.** Quirks live in TOML, not Rust. Adding a provider is a 200-line file. Tool-call format is declared. Model lists are discoverable (`model = "latest"` resolves at runtime).
5. **MCP-first integration.** The core ships ~8 essential tools. Everything else (browser, voice, image-gen, github, slack-tools, …) is an MCP server — versioned, capability-scoped, isolated in its own subprocess.

These five are the spec. Anything that violates them is a bug.

### 1.3 Hard non-functional targets (v0.2 — expanded)

| Target | Number | Measurement |
|---|---|---|
| Cold-boot latency (CLI to first prompt) | < 150 ms | `hyperfine 'argo --version'` p50 |
| Memory at idle (gateway daemon) | < 30 MB RSS | `ps -o rss` after 60s idle |
| Single-binary install size | < 18 MB stripped (was 15; +3 for routing/judge/forge inline) | `wc -c argo` |
| First-message-to-first-token | < 800 ms (excl. network) | tracer span `agent.first_token` |
| Concurrent sessions per gateway | ≥ 1,000 | load test, p95 turn latency stable |
| Core LOC ceiling | ≤ 95,000 (Rust; was 80k; +15k for new crates) | `tokei` |
| Plugin SDK LOC | ≤ 8,000 per language | `tokei sdk-{py,ts,go}` |
| Cold start to docker-sandbox-ready | < 2 s | tracer span `sandbox.ready` |
| Test suite wall-clock | ≤ 5 min on 16-core (was 4; +1 for new crates) | `cargo nextest run` |
| **NEW — Idle token overhead** (system + 8 in-core tools schema, per turn) | **≤ 4,000 tokens** | telemetry counter `agent.context.tokens.system_overhead` |
| **NEW — Cost-router classifier latency** (when `[routing] policy = "tiered"`) | **≤ 80 ms p95** | tracer span `routing.classify` |
| **NEW — Spend-cap enforcement precision** (predicted vs. actual at hard cap) | **within 5%** | NFR check after 1k synthetic turns |
| **NEW — Judge verdict latency** (per skill creation/refinement event) | **≤ 1.5 s p95** | tracer span `judge.adjudicate` |
| **NEW — Forge optimization run wall-clock** (per skill, default settings) | **≤ 8 min p95** | tracer span `forge.optimize_run` |
| **NEW — Forge cost per optimization run** (default) | **≤ $5 USD** | counter `forge.spend_usd_micros` |

If any number is missed by > 50% on a release candidate, the release is held.

### 1.4 What we deliberately don't build in v1.0

The list shrinks substantially from v0.1:

- ~~Voice synthesis in core~~ → **Spec'd as MCP server** (§12, Phase 4.5). Pure Rust whisper.cpp + piper-rs; works on Android (no ctranslate2).
- ~~Browser automation in core~~ → **Spec'd as MCP server** (§12, Phase 4.5). Local Playwright is the default; cloud (Browserbase) is opt-in.
- ~~Image generation in core~~ → **Spec'd as plugin + MCP server** (§11). OpenAI/xAI/Codex backends.
- Native iOS/Android apps. **v1.5.**
- Web dashboard with full feature surface. **v1.0 ships a minimal status dashboard; v1.5 expands.**
- Native Chinese-platform messaging adapters in-tree. **v1.0 ships them as plugins** maintained by community partners.
- Atropos RL training pipeline equivalent. Out of scope.
- Federated cross-install learning. **v2.0** (gap-analysis §5.5).

What we deliberately KEEP shipping in v1.0 (added by v0.2):

- ✅ ACP server (Phase 4.5)
- ✅ Voice MCP server (Phase 4.5)
- ✅ Browser MCP server with local Playwright default (Phase 4.5)
- ✅ Built-in cost-tier router (Phase 2)
- ✅ External-judge adjudicator (Phase 3)
- ✅ Argo Forge skill optimizer (Phase 7.5)
- ✅ Time-travel session replay (Phase 7)
- ✅ Workspace OCI marketplace (Phase 6.5)
- ✅ Hard $$$ spend ceilings per session/day/month (Phase 1, baked in)

Saying yes to these is the v0.2 scope shift. The trade-offs (what we cut to make room) are in §27.5.

### 1.5 Acceptance for §1

- [ ] All five invariants are referenced by name in CONTRIBUTING.md.
- [ ] Every NFR target above has a CI job that fails the build if regressed.
- [ ] The "deliberately don't build" list is referenced in README's "Why not Hermes Agent?" section.
- [ ] CHANGELOG records the v0.1 → v0.2 scope shift.

---

## 2. Definition of Done — the v1.0 gate (v0.2 — expanded)

Argo v1.0 ships when **every** item below is true. No exceptions.

### 2.1 Functional gates (v0.2 — adds 16 items)

#### Carried from v0.1:

- [ ] Conversation loop parity with Hermes' `AIAgent.run_conversation`: streaming, tool calls, parallel-where-safe, retry/repair/fallback.
- [ ] Three first-class providers: OpenAI-compatible, Anthropic Messages, AWS Bedrock Converse.
- [ ] Two execution backends: local + Docker. (SSH/Modal/Daytona/Singularity ship as plugins in v1.1.)
- [ ] Three messaging gateway adapters: Slack, Discord, **Telegram (promoted from plugin to in-tree at v0.2; see §27.5)**.
- [ ] Eight in-core tools: `terminal`, `file_read`, `file_write`, `file_patch`, `web_search`, `web_extract`, `memory`, `delegate_task`.
- [ ] Skills system: agentskills.io-compatible, content-addressable, provenance, `skill_view`/`skill_manage`/`skills_list`.
- [ ] Memory: `MEMORY.md`/`USER.md`/`SOUL.md`/`AGENTS.md`, versioned, with provenance pointers.
- [ ] SQLite + FTS5 session store with proper migrations (up + down).
- [ ] Capability model enforced at every tool dispatch.
- [ ] Plugin protocol stable: pre-tool veto, post-tool transform, terminal-output transform, slash-command register, dispatch-tool, image-gen backend, memory backend.
- [ ] MCP server with 12 tools (sessions, messages, attachments, events, send, channels, permissions, plus 4 management).
- [ ] MCP client over stdio, SSE, HTTP.
- [ ] Cron scheduler with platform delivery.
- [ ] Subagent delegation with token budget tree + COW filesystem isolation.
- [ ] Multi-tenant primitives plumbed through every layer.
- [ ] Audit trail: append-only, hash-chained, exportable.
- [ ] OpenTelemetry traces native.
- [ ] Hot config reload (SIGHUP).
- [ ] Static binaries: macOS arm64/x86_64, Linux arm64/x86_64, Windows x86_64.

#### NEW in v0.2 (Amendments 1-14 + Forge + time-travel + marketplace + cost ceilings):

- [ ] **A1: Built-in cost-tier router.** Config-driven `[routing.tiered]` with classifier + per-tier model. Disabled by default; `--routing tiered` enables.
- [ ] **A2: External-judge adjudicator.** Every reflective skill create/refine routed through a separate judge model. Three-state outcome (Success/Partial/Failure). `Partial` writes a draft skill marked unverified; `Failure` discards; only `Success` ships into the active skill set.
- [ ] **A3: User-locked skills + memory.** `lock: user-protected` and `lock: content-locked` frontmatter honored by all skill/memory writers. Agent edits to user-protected entries are rejected at dispatch time with a clear error.
- [ ] **A4: Gateway RBAC tiers.** Owner/Admin/User/Guest preset roles + arbitrary custom roles. Bindings tied to `(adapter, identity)`. Capability presets editable.
- [ ] **A5: Native multi-agent per tenant.** N named agents alive in a single process. Each has its own persona, default model, default toolset, memory namespace. Gateway routes by `(adapter, channel) → agent_name`.
- [ ] **A6: Adaptive provider timeouts.** Heartbeat-aware reconnect logic; any data byte resets the timer. No fixed 180s reconnect.
- [ ] **A7: Per-delegation fallback inheritance.** `delegate_task` inherits parent's fallback chain by default; per-call override supported.
- [ ] **A8: `model = "latest"` resolution.** Manifest `[discovery]` block with endpoint + filters; resolves at startup, refreshes daily.
- [ ] **A9: `auto_continue_at_max_iterations` policy.** `halt` (default) | `prompt_user` | `auto_continue` with a hard ceiling on extensions.
- [ ] **A10: Local-Playwright browser MCP server.** Default. Auto-detect local Chrome on `--remote-debugging-port=9222`. Cloud (Browserbase, BrowserUse) opt-in only.
- [ ] **A11: ACP server (`argo-acp` crate).** Multi-harness from day 1: Copilot, Cursor agent CLI, generic ACP. No hardcoded single-harness limitation.
- [ ] **A12: Voice MCP server.** Pure-Rust whisper.cpp + piper-rs. OGG/Opus encoding by default (fixes Matrix mobile playback). `instructions` field exposed for OpenAI TTS backend.
- [ ] **A13: Resume interrupted task on restart.** Daemon detects orphaned tool-call records on boot and surfaces a list with policy `prompt_user` | `auto_resume` | `discard`.
- [ ] **A14: Mid-run `/steer`.** CLI, TUI, and gateway all support injecting steering text the running agent sees after its next tool call. No prompt-cache break.
- [ ] **F: Argo Forge.** `argo forge optimize`, `argo forge bench`, `argo forge promote`. DSPy + GEPA-style genetic/Pareto skill optimization. Default off; explicit opt-in. Per-run budget caps. ICLR-2026-Oral algorithmic lineage cited.
- [ ] **R: Time-travel debug.** `argo session replay <id> --model X --tools Y --workspace Z`. Per-session deterministic replay. `argo session diff` and `argo session bisect`.
- [ ] **M: Workspace OCI marketplace.** `argo workspace push <oci_ref>` and `pull` push/pull workspace bundles via OCI registries (GHCR, Docker Hub, Artifact Registry, etc.). Reproducible bundle hashes.
- [ ] **C: Hard $$$ cost ceilings.** Per-session, per-day, per-month enforced caps in `[budget]`. Block before spend, not after. 80% / 95% notifications.

### 2.2 Quality gates

- [ ] All hard NFRs from §1.3 met.
- [ ] Test coverage ≥ 85% line, ≥ 70% branch on `argo-core`, `argo-store`, `argo-capability`, `argo-routing`, `argo-judge`.
- [ ] Eval harness passes ≥ 90% on `core` + `multi-turn` + `memory` + **`judge-disagreement`** (NEW v0.2 suite) + **`routing-correctness`** (NEW v0.2 suite).
- [ ] Zero `unsafe` Rust except in clearly-marked FFI shims.
- [ ] `cargo audit` clean. `cargo deny` rules pass.
- [ ] Every public API has rustdoc with at least one example.
- [ ] Docs site builds. Every doc page's examples auto-extracted as compiled tests.

### 2.3 Operational gates

- [ ] Release pipeline reproducibly produces signed artifacts.
- [ ] Homebrew formula, scoop bucket, apt deb, single-curl install — all working.
- [ ] First-time UX: `curl install` → `argo` → working chat in ≤ 90 seconds, no manual config.
- [ ] Migration tool: `argo import hermes` ports a Hermes installation (config, sessions, skills, memory).
- [ ] **Workspace OCI marketplace pull**: `argo workspace pull ghcr.io/argo-agents/example-bdr:latest` works against a real public bundle on day 1.

If a single checkbox above is unchecked, v1.0 does not ship.

---

## 3. Repository layout (v0.2 — adds 6 crates)

```
argo/
├── README.md
├── CONTRIBUTING.md
├── LICENSE
├── ARCHITECTURE.md
├── ROADMAP.md
├── Cargo.toml
├── rust-toolchain.toml
├── deny.toml
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── release.yml
│   │   ├── eval.yml
│   │   ├── nfr-bench.yml
│   │   └── docs.yml
│   ├── ISSUE_TEMPLATE/
│   └── PULL_REQUEST_TEMPLATE.md
├── crates/
│   ├── argo-core/                     # the agent loop + state machine
│   ├── argo-types/                    # shared types (no deps on other crates)
│   ├── argo-config/                   # config loading + schema
│   ├── argo-store/                    # SQLite + FTS5 + migrations
│   ├── argo-capability/               # capability model + RBAC engine
│   ├── argo-provider/                 # provider manifest loader + transport
│   ├── argo-tool/                     # tool registry + 8 in-core tools
│   ├── argo-memory/                   # MEMORY/USER/SOUL/AGENTS files; locks
│   ├── argo-skill/                    # skills loader + provenance + locks
│   ├── argo-cron/                     # scheduler
│   ├── argo-delegate/                 # subagent budget tree + COW FS
│   ├── argo-sandbox/                  # exec backends (local + docker)
│   ├── argo-plugin/                   # plugin host (subprocess + JSON-RPC)
│   ├── argo-mcp/                      # MCP client + server
│   ├── argo-gateway/                  # messaging gateway runtime
│   │   └── adapters/                  # slack, discord, telegram (NEW v0.2)
│   ├── argo-acp/                      # NEW v0.2 — Agent Client Protocol server
│   ├── argo-cli/                      # `argo` binary
│   ├── argo-tui/                      # ratatui TUI
│   ├── argo-audit/                    # audit log (hash chain)
│   ├── argo-telemetry/                # OTel tracing setup
│   ├── argo-eval/                     # eval harness runtime
│   ├── argo-routing/                  # NEW v0.2 — cost-tier router (Amendment 1)
│   ├── argo-judge/                    # NEW v0.2 — external-LLM adjudicator (Amendment 2)
│   ├── argo-forge/                    # NEW v0.2 — DSPy/GEPA skill optimizer (Phase 7.5)
│   ├── argo-replay/                   # NEW v0.2 — time-travel session replay
│   └── argo-marketplace/              # NEW v0.2 — workspace OCI push/pull
├── manifests/
│   └── providers/                     # one TOML per provider
│       ├── openai-compatible.toml
│       ├── openai-responses.toml
│       ├── anthropic.toml
│       └── bedrock.toml
├── plugins/                           # bundled reference plugins
│   ├── memory-honcho/                 # Python (with handshake timeout — fixes #5726)
│   ├── obs-langfuse/                  # Python
│   └── tool-disk-cleanup/             # Python
├── mcp-servers/                       # bundled reference MCP servers (NEW v0.2 — proper specs)
│   ├── browser/                       # Rust + Playwright local default; Browserbase opt-in
│   ├── voice/                         # Rust whisper.cpp + piper-rs
│   ├── image-gen/                     # Python (OpenAI / xAI / Codex backends)
│   └── github/                        # Go
├── sdk/
│   ├── argo-plugin-py/
│   └── argo-plugin-ts/                # Go SDK deferred to v1.1 — see §27.5
├── docs/
│   ├── intro/, user/, ops/, dev/, ref/
│   └── examples/
├── eval/
│   ├── tasks/
│   │   ├── core/                      # 50 tool-calling correctness
│   │   ├── multi-turn/                # 25 multi-turn
│   │   ├── delegation/                # 15 delegate
│   │   ├── memory/                    # 10 memory
│   │   ├── provider-parity/           # 5 cross-provider
│   │   ├── judge-disagreement/        # NEW v0.2 — 15 deliberate failures judged
│   │   └── routing-correctness/       # NEW v0.2 — 20 routing-decision tasks
│   └── runners/
├── examples/
├── scripts/
└── tests/
    ├── e2e/
    └── fixtures/
```

26 crates total (was 20 in v0.1; v0.2 adds `argo-acp`, `argo-routing`, `argo-judge`, `argo-forge`, `argo-replay`, `argo-marketplace`).

### 3.1 Acceptance for §3

- [ ] `cargo metadata` reports the exact 26-crate count.
- [ ] `tokei` reports each crate ≤ its budget.
- [ ] No source file > 500 LOC; CI fails on violation.
- [ ] No fn > 80 LOC; no struct impl block > 200 LOC.

---

## 4. Architectural pillars

### 4.1 Process model (v0.2 — adds router/judge/forge as in-process subsystems)

```
                       ┌─────────────────────────────────────────────┐
   user input  →      │           argo (single binary)                │   ← config, secrets
                       │                                              │
                       │  ┌────────────┐    ┌──────────────┐         │
                       │  │  argo-cli  │ ↔ │   argo-tui   │         │
                       │  └────┬───────┘    └──────┬───────┘         │
                       │       ▼                   ▼                  │
                       │  ┌────────────────────────────────────────┐  │
                       │  │              argo-core                  │  │
                       │  │   loop · registry · capability · store  │  │
                       │  │   audit · memory · skill · delegate     │  │
                       │  │   ┌──────────────────────────────────┐  │  │
                       │  │   │ NEW v0.2:                        │  │  │
                       │  │   │  argo-routing  argo-judge        │  │  │
                       │  │   │  argo-replay   argo-marketplace  │  │  │
                       │  │   │  argo-forge (separate binary)    │  │  │
                       │  │   └──────────────────────────────────┘  │  │
                       │  └─┬──────────┬──────────┬──────────┬─────┘  │
                       │    ▼          ▼          ▼          ▼        │
                       │  plugin    MCP        ACP        gateway     │
                       │  host      client     server     daemon      │
                       └────┼──────────┼──────────┼──────────┼────────┘
                            ▼          ▼          ▼          ▼
                       plugin     MCP server   IDE host  msg platforms
                       subproc    subproc      (VSCode/  (slack/
                       (py/ts)    (rust/py)    Zed/      discord/
                                                JetBrains) telegram)
```

Five classes of subprocess (was three):
- **Plugins** — Argo plugin protocol over stdio (lifecycle owned by `argo-plugin`)
- **MCP servers** — MCP protocol (lifecycle owned by `argo-mcp`)
- **Gateway daemon** — long-running, embeds `argo-core`
- **ACP harness** — IDE clients connect inbound; `argo-acp` is the server (NEW v0.2)
- **Forge worker** — `argo forge optimize` spawns optimizer worker subprocesses (NEW v0.2)

### 4.2 Data flow per turn (v0.2 — adds routing + judge spans)

```
user message
  → tenant + agent_name resolved
  → session looked up / created
  → capability set computed (per agent + RBAC role)
  → context assembled (system + memory + skills + history)
  → [NEW] router: classify prompt complexity → select model tier
  → provider manifest selected for chosen model
  → request built; sent via transport (heartbeat-aware timeout)
  → response streamed back
  → if tool_calls:
      for each call:
        capability check  (deny if not granted)
        argument coercion (per provider manifest)
        dispatch (in-core | plugin | MCP)
        capture result, persist, audit
      back to provider with results
  → else: final answer; persist; audit
  → if reflective skill candidate detected:
      [NEW] judge: external-LLM verdict (Success/Partial/Failure)
      Success     → write skill (active set)
      Partial     → write draft (unverified set, hidden from skills_list by default)
      Failure     → discard, audit
  → return
```

Every arrow is a tracer span. Every dispatch produces an audit record.

### 4.3 Acceptance for §4

- [ ] `argo trace --turn` prints a flame graph showing every span in the diagram, including new routing/judge spans.
- [ ] Killing any plugin / MCP server / ACP client subprocess does not kill the agent (chaos test).
- [ ] Forge optimizer worker crash is recoverable; partial progress checkpointed.

---

## 5. Type system — `argo-types` (v0.2 — adds AgentName, Role, RoutingPolicy, Lock, JudgeOutcome)

### 5.1 Core IDs (UUIDv7)

```rust
// SPEC
pub struct TenantId(pub Uuid);
pub struct AgentId(pub Uuid);
pub struct AgentName(pub String);          // NEW v0.2 — human-named agent within a tenant
pub struct SessionId(pub Uuid);
pub struct MessageId(pub Uuid);
pub struct ToolCallId(pub Uuid);
pub struct WorkspaceId(pub Uuid);
pub struct SkillId(pub Uuid);
pub struct JudgeRecordId(pub Uuid);        // NEW v0.2
pub struct RoutingDecisionId(pub Uuid);    // NEW v0.2
pub struct ForgeRunId(pub Uuid);           // NEW v0.2
```

### 5.2 Conversation messages

(unchanged from v0.1: `Message`, `ContentBlock`, `ToolCallRequest`, `ToolCallResult`)

### 5.3 Capabilities and roles (v0.2 — adds `Role`)

```rust
// SPEC (v0.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Capability {
    FsRead { path_glob: String },
    FsWrite { path_glob: String },
    NetEgress { host_glob: String, port: Option<u16> },
    EnvRead { var_glob: String },
    SecretRead { name: String },
    ToolDispatch { tool_name_glob: String },
    Spawn { kind: SpawnKind },
    AuditWrite,
    MemoryWrite,
    SkillWrite,
    SkillJudge,                // NEW v0.2 — only the judge subsystem may emit verdicts
    ForgeRun,                  // NEW v0.2 — only the Forge subsystem may run optimizations
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpawnKind { Subagent, Subprocess, Container }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet(pub Vec<Capability>);

// NEW v0.2 — Role
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Role {
    pub name: String,                  // e.g. "owner" / "admin" / "user" / "guest" / "support_l1"
    pub capabilities: CapabilitySet,
    pub inherits_from: Option<String>, // single-inheritance role hierarchy
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBinding {
    pub principal: Principal,          // (adapter, identity) or local CLI user
    pub role_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Principal {
    Local { os_user: String },
    Gateway { adapter: String, identity: String },
    Acp { harness: String, identity: String },
    Forge,                             // the Forge subsystem itself
}
```

### 5.4 Budgets (v0.2 — adds hard $$$ caps)

```rust
// SPEC (v0.2)
pub struct Budget {
    pub iterations: Option<u32>,
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    pub wallclock_ms: Option<u64>,
    pub net_egress_bytes: Option<u64>,
    pub spend_usd_micros: Option<u64>,    // 1 USD = 1_000_000
    pub tool_calls: Option<u32>,
    // NEW v0.2 — hard ceilings
    pub spend_per_session_usd: Option<u64>,   // micros
    pub spend_per_day_usd: Option<u64>,       // micros
    pub spend_per_month_usd: Option<u64>,     // micros
}
```

`BudgetTree` semantics unchanged: CAS-based, child carved out of parent, total bounded by root.

### 5.5 Routing types (NEW v0.2)

```rust
// SPEC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingPolicy {
    Fixed,
    Tiered(TieredRouting),
    PolicyPlugin { plugin: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredRouting {
    pub classifier_model: String,           // small/cheap model name
    pub simple: ProviderModel,
    pub medium: ProviderModel,
    pub complex: ProviderModel,
    pub classify_threshold_ms: u64,         // fall through to default if classifier slow
    pub fallback_to_default_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub id: RoutingDecisionId,
    pub session_id: SessionId,
    pub turn: u32,
    pub policy: String,                     // policy name
    pub tier: String,                       // simple | medium | complex | unrouted
    pub classified_at: DateTime<Utc>,
    pub classifier_latency_ms: u64,
    pub chosen_provider_model: ProviderModel,
    pub reason: String,                     // free-form rationale
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub provider: String,
    pub model: String,
}
```

### 5.6 Judge types (NEW v0.2)

```rust
// SPEC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JudgeVerdict {
    Success,
    Partial { reason: String },
    Failure { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeRecord {
    pub id: JudgeRecordId,
    pub session_id: SessionId,
    pub subject: JudgeSubject,
    pub verdict: JudgeVerdict,
    pub judge_model: ProviderModel,
    pub rubric_id: String,
    pub rubric_version: String,
    pub evidence_hashes: Vec<[u8; 32]>,     // sha256 of inputs the judge saw
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JudgeSubject {
    SkillCreate { name: String, hash: [u8; 32] },
    SkillRefine { name: String, version_from: String, version_to_hash: [u8; 32] },
    MemoryWrite { entry_id: Uuid },         // optional adjudication for memory writes
    DelegateOutcome { delegate_session: SessionId },
}
```

### 5.7 Lock types for memory + skills (NEW v0.2)

```rust
// SPEC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lock {
    None,                                   // default; agent may freely edit
    UserProtected,                          // agent cannot edit; only `argo skill edit --force` allowed
    ContentLocked,                          // agent may edit but lineage hash must remain in chain
}
```

### 5.8 Result envelope (v0.2 — adds variants)

```rust
// SPEC
pub type ArgoResult<T> = Result<T, ArgoError>;

#[derive(Debug, thiserror::Error)]
pub enum ArgoError {
    #[error("capability denied: {0:?}")]      CapabilityDenied(Capability),
    #[error("budget exhausted: {kind:?}")]    BudgetExhausted { kind: BudgetKind },
    #[error("provider error: {0}")]           Provider(String),
    #[error("tool error: {tool}: {msg}")]     Tool { tool: String, msg: String },
    #[error("storage error")]                 Storage(#[from] sqlx::Error),
    #[error("io error")]                      Io(#[from] std::io::Error),
    #[error("schema validation failed: {0}")] Schema(String),
    #[error("invariant violated: {0}")]       Invariant(&'static str),
    #[error("interrupted")]                   Interrupted,
    // NEW v0.2
    #[error("user-protected: {entity}")]      UserProtected { entity: String },
    #[error("judge rejected: {reason}")]      JudgeRejected { reason: String },
    #[error("routing failed: {reason}")]      RoutingFailed { reason: String },
    #[error("rbac denied: {role} lacks {cap:?}")] RbacDenied { role: String, cap: Capability },
    #[error("oci error: {0}")]                Oci(String),
}
```

### 5.9 Acceptance for §5

- [ ] `argo-types` has zero deps on other Argo crates.
- [ ] All ID types are UUIDv7-based.
- [ ] All new types have `proptest::Arbitrary` impls for downstream property tests.
- [ ] `cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used` passes outside tests.

---

## 6. The agent loop — `argo-core` (v0.2 — adds router + judge + steer hooks)

### 6.1 The state machine (v0.2 — adds RoutingDecision + AwaitingJudge)

```rust
pub enum LoopState {
    Idle,
    BuildingContext,
    Routing,                       // NEW v0.2 — classifying for tiered routing
    AwaitingProvider,
    StreamingResponse,
    DispatchingTools { batch: Vec<ToolCallRequest> },
    Compressing,
    AwaitingJudge,                 // NEW v0.2 — adjudicating reflective skill candidate
    FinalAnswer,
    Errored(ArgoError),
    Halted(HaltReason),
}
```

### 6.2 Public surface (unchanged shape; new deps)

```rust
pub struct AgentDeps {
    pub store: Arc<dyn SessionStore>,
    pub provider: Arc<dyn ProviderTransport>,
    pub registry: Arc<ToolRegistry>,
    pub capability_engine: Arc<CapabilityEngine>,
    pub audit: Arc<AuditSink>,
    pub telemetry: Arc<TelemetrySink>,
    pub memory: Arc<MemoryStore>,
    pub skills: Arc<SkillStore>,
    pub budget: BudgetTree,
    // NEW v0.2
    pub router: Arc<dyn Router>,
    pub judge: Arc<dyn Judge>,
    pub steer: Arc<SteerChannel>,
}
```

### 6.3 Steering channel (NEW v0.2)

```rust
// SPEC
pub struct SteerChannel { /* */ }

impl SteerChannel {
    /// Enqueue a steer; the loop consumes it after the next tool result.
    pub fn enqueue(&self, session_id: SessionId, text: String) -> ArgoResult<()>;
    /// Drain any pending steers for a session (called by the loop).
    pub fn drain(&self, session_id: SessionId) -> Vec<String>;
}
```

Steer text is appended to a synthesized user message produced by the loop *between* tool result and next assistant turn. Prompt-cache breakpoints are unaffected (steers come AFTER the cached prefix).

### 6.4 Adaptive timeout policy (NEW v0.2 — Amendment 6)

The loop owns a `HeartbeatMonitor` per active provider call. Timer resets on:
- Any byte received from the streaming response
- Any provider-emitted heartbeat event (`event: ping`, `:keepalive`, etc., per manifest)
- Successful HTTP keep-alive frame on non-streaming providers

Default grace window: 30s from last byte. No fixed 180s reconnect.

### 6.5 Resume-on-restart (NEW v0.2 — Amendment 13)

On `Agent::startup()`:

1. Query `tool_call WHERE ended_at IS NULL` per active session.
2. Build a list of `InterruptedSession { session_id, last_tool_call, started_at, age_seconds }`.
3. Apply policy from config `[loop.on_restart]`:
   - `prompt_user` (default): surface a list to user (CLI/TUI/gateway DM); user picks resume / discard / cancel-session.
   - `auto_resume`: re-emit the last tool call to the model with a "this was interrupted; verify and continue" prefix.
   - `discard`: mark session ended with `end_reason = "restart_discard"`; archive.

### 6.6 Auto-continue at max iterations (NEW v0.2 — Amendment 9)

When `IterationBudget::charge` returns Err:

```toml
[budget]
on_max_iterations = "prompt_user"           # halt | prompt_user | auto_continue
auto_continue_max_extensions = 3            # hard cap
auto_continue_extension_size = 30
```

The loop never silently extends beyond `max_extensions × extension_size + initial`.

### 6.7 Acceptance for §6

- [ ] `argo-core` builds in < 90s (was 60s; +30s for new subsystem deps).
- [ ] `argo-core` ≤ 9,000 LOC (was 8k; +1k for routing/judge/steer hooks).
- [ ] Property tests verify the state machine; new states `Routing` and `AwaitingJudge` covered.
- [ ] Chaos test: kill the router subsystem mid-classification; loop falls through to default model with a logged warning, does NOT halt.
- [ ] Adaptive-timeout property test: a stream emitting one byte per 25s never reconnects; a stream silent for 35s does.

---

## 7. Database schema — `argo-store` (v0.2 — adds tables for new subsystems)

### 7.1 Schema v1 (additions in v0.2)

In addition to the v0.1 tables, the following are required:

```sql
-- SPEC v0.2 additions

CREATE TABLE agent (
    tenant_id       BLOB NOT NULL,
    name            TEXT NOT NULL,
    persona_file    TEXT,                        -- path within workspace
    default_model   TEXT NOT NULL,
    default_toolset TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, name)
);

CREATE TABLE role (
    tenant_id       BLOB NOT NULL,
    name            TEXT NOT NULL,
    inherits_from   TEXT,
    capabilities    TEXT NOT NULL,                -- JSON array of CapabilitySet
    PRIMARY KEY (tenant_id, name)
);

CREATE TABLE role_binding (
    tenant_id       BLOB NOT NULL,
    principal_kind  TEXT NOT NULL,                -- 'local' | 'gateway' | 'acp' | 'forge'
    principal_key   TEXT NOT NULL,
    role_name       TEXT NOT NULL,
    bound_at        INTEGER NOT NULL,
    bound_by        TEXT NOT NULL,
    PRIMARY KEY (tenant_id, principal_kind, principal_key)
);

CREATE TABLE routing_decision (
    id              BLOB PRIMARY KEY,
    session_id      BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    turn            INTEGER NOT NULL,
    policy          TEXT NOT NULL,
    tier            TEXT NOT NULL,
    classifier_latency_ms INTEGER,
    chosen_provider TEXT NOT NULL,
    chosen_model    TEXT NOT NULL,
    reason          TEXT,
    decided_at      INTEGER NOT NULL
);

CREATE TABLE judge_record (
    id              BLOB PRIMARY KEY,
    session_id      BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    subject_kind    TEXT NOT NULL,                -- skill_create | skill_refine | memory_write | delegate_outcome
    subject_ref     TEXT NOT NULL,                -- canonical ref to the subject
    verdict         TEXT NOT NULL,                -- success | partial | failure
    reason          TEXT,
    judge_provider  TEXT NOT NULL,
    judge_model     TEXT NOT NULL,
    rubric_id       TEXT NOT NULL,
    rubric_version  TEXT NOT NULL,
    evidence_hashes TEXT NOT NULL,                -- JSON array of sha256
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_judge_subject ON judge_record(subject_kind, subject_ref);

CREATE TABLE skill_lock (
    skill_id        BLOB PRIMARY KEY REFERENCES skill(id) ON DELETE CASCADE,
    lock            TEXT NOT NULL,                -- 'user-protected' | 'content-locked'
    set_at          INTEGER NOT NULL,
    set_by          TEXT NOT NULL                 -- principal that set the lock
);

CREATE TABLE memory_lock (
    entry_id        BLOB PRIMARY KEY REFERENCES memory_entry(id) ON DELETE CASCADE,
    lock            TEXT NOT NULL,
    set_at          INTEGER NOT NULL,
    set_by          TEXT NOT NULL
);

CREATE TABLE forge_run (
    id              BLOB PRIMARY KEY,
    skill_name      TEXT NOT NULL,
    started_at      INTEGER NOT NULL,
    ended_at        INTEGER,
    base_version    TEXT NOT NULL,
    candidate_version TEXT,                       -- if accepted + promoted
    examples_count  INTEGER NOT NULL,
    iterations      INTEGER NOT NULL,
    spend_usd_micros INTEGER NOT NULL,
    final_score     REAL,
    status          TEXT NOT NULL                 -- running | succeeded | failed | aborted
);

CREATE TABLE marketplace_artifact (
    id              BLOB PRIMARY KEY,
    workspace_id    BLOB NOT NULL REFERENCES workspace(id),
    oci_ref         TEXT NOT NULL,                -- e.g. ghcr.io/argo-agents/sales-bdr:1.2.0
    digest          TEXT NOT NULL,                -- sha256:...
    pushed_at       INTEGER NOT NULL,
    pushed_by       TEXT NOT NULL
);

-- Existing audit_record table extended via payload JSON;
-- new audit kinds: 'routing.decision', 'judge.verdict', 'rbac.deny',
-- 'forge.run.start', 'forge.run.end', 'marketplace.push', 'marketplace.pull',
-- 'session.replay', 'lock.set', 'lock.violate', 'steer.enqueued'.
```

### 7.2 Migrations

Same `Migration` trait + sqlx pattern as v0.1. v0.1 → v0.2 migration adds the tables above. v0.2 → v0.1 drops them. CI round-trips both.

### 7.3 Acceptance for §7

- [ ] All schemas above compile and apply on a clean SQLite.
- [ ] Migration v1 → v2 → v1 round-trip is no-op.
- [ ] Concurrent writers stress test passes (10 writer threads, 10k inserts each).
- [ ] FTS5 still passes phrase + prefix + AND/OR + CJK.

---

## 8. Provider manifest schema — `argo-provider` (v0.2 — adds discovery + heartbeat)

### 8.1 Manifest schema (v0.2 additions)

```toml
# SPEC v0.2 — additions to manifests/providers/openai-compatible.toml

[discovery]
endpoint = "{base_url}/v1/models"
include_pattern = "^gpt-[0-9]"
exclude_pattern = "preview|legacy|deprecated"
prefer = "highest_minor"                      # lexicographic ordering rule
refresh_interval_hours = 24

[transport]
# existing fields unchanged
streaming = "sse"
heartbeat_event = "ping"                       # NEW v0.2 — SSE event name that resets timer
heartbeat_grace_ms = 30000                     # NEW v0.2 — adaptive timeout grace window
keepalive_chunks_reset = true                  # NEW v0.2 — chunked HTTP keep-alive resets timer

[reasoning]
# existing fields unchanged
content_field = "thinking"                     # for Anthropic; absent for non-reasoning models
preserve_across_turns = true
```

### 8.2 `model = "latest"` resolution

When the user config sets `model = "latest"`, the manifest's `[discovery]` block is consulted at startup and every `refresh_interval_hours`. The resolved model is cached and surfaced in `argo model show`. Manual override always wins.

### 8.3 Acceptance for §8

- [ ] All five v1 manifests deserialize.
- [ ] Adding a synthetic OpenAI-compat provider via TOML alone works (no Rust changes).
- [ ] `xml_tool_call` extractor passes against Hermes-format fixture corpus.
- [ ] `model = "latest"` resolves correctly against a recorded `/v1/models` fixture.
- [ ] Heartbeat-driven timeout property test passes (§6.7).

---

## 9. Tool registry — `argo-tool`

(Unchanged from v0.1 in shape; eight in-core tools; concurrency declarations; argument coercion.)

### 9.1 Acceptance — unchanged

---

## 10. Capability + RBAC — `argo-capability` (v0.2 — adds RBAC role engine)

### 10.1 Engine surface (v0.2)

```rust
// SPEC v0.2
pub struct CapabilityEngine {
    granted: Arc<RwLock<CapabilitySet>>,
    role_table: Arc<RwLock<RoleTable>>,
}

pub struct RoleTable {
    roles: HashMap<String, Role>,
    bindings: HashMap<Principal, String>,
}

impl CapabilityEngine {
    pub fn check(&self, principal: &Principal, requested: &Capability) -> ArgoResult<()>;
    pub fn check_batch(&self, principal: &Principal, calls: &[ToolCallRequest]) -> ArgoResult<Vec<ApprovedCall>>;
    pub fn resolve_role(&self, principal: &Principal) -> ArgoResult<Role>;
    pub fn add_role(&self, role: Role) -> ArgoResult<()>;
    pub fn bind(&self, principal: Principal, role_name: String) -> ArgoResult<()>;
    pub fn revoke_binding(&self, principal: &Principal) -> ArgoResult<()>;
    pub fn grant_dynamic(&self, principal: &Principal, cap: Capability, mode: GrantMode) -> ArgoResult<()>;
    pub fn require_user_approval(&self, kind: ApprovalKind) -> ArgoResult<UserApproval>;
}
```

### 10.2 Default role presets (Amendment 4)

Shipped in `argo init`'s default config:

```toml
[gateway.rbac]
default_role = "guest"

[gateway.rbac.roles.owner]
inherits_from = ""
capabilities = [
    { kind = "fs_read", path_glob = "**" },
    { kind = "fs_write", path_glob = "**" },
    { kind = "tool_dispatch", tool_name_glob = "*" },
    { kind = "spawn", spawn_kind = "subagent" },
    { kind = "spawn", spawn_kind = "subprocess" },
    { kind = "spawn", spawn_kind = "container" },
    { kind = "memory_write" },
    { kind = "skill_write" },
    { kind = "audit_write" },
    { kind = "secret_read", name = "*" },
    { kind = "net_egress", host_glob = "**" },
]

[gateway.rbac.roles.admin]
inherits_from = "owner"
capabilities = []                              # owner minus secrets and audit
# (subtractive overrides not yet supported; admin role redefines explicitly)

[gateway.rbac.roles.user]
capabilities = [
    { kind = "fs_read", path_glob = "{workspace}/**" },
    { kind = "fs_write", path_glob = "{workspace}/**" },
    { kind = "tool_dispatch", tool_name_glob = "web_*,memory,skill_view,skills_list,clarify" },
    { kind = "memory_write" },
]

[gateway.rbac.roles.guest]
capabilities = [
    { kind = "tool_dispatch", tool_name_glob = "web_search,web_extract,clarify" },
]
```

`{workspace}` is interpolated to the current workspace root at check time.

### 10.3 Approval flow

(Unchanged from v0.1: Once / ThisSession / ThisWorkspace / Always.)

### 10.4 Acceptance for §10

- [ ] Property test: every tool's declared `capabilities` is a superset of what it actually uses (instrumented harness).
- [ ] Capability denial kills tool dispatch with no leakage.
- [ ] User-approval flow round-trips through CLI, TUI, gateway, and ACP.
- [ ] Role inheritance resolution is deterministic and cycle-free.
- [ ] RBAC: a `guest`-bound principal trying `terminal` is denied with a clear error pointing at the role and missing cap.

---

## 11. Plugin protocol — `argo-plugin` (v0.2 — adds steer + transform_skill + judge_outcome)

### 11.1 Lifecycle

(Unchanged from v0.1: spawn → handshake → handshake_ack → bidirectional RPC → shutdown.)

### 11.2 Argo → plugin methods (v0.2 additions)

| Method | When called | Plugin returns |
|---|---|---|
| `pre_tool_call` | Before each tool dispatch | allow/deny/transform_args |
| `post_tool_call` | After each tool result | result (may transform) |
| `transform_terminal_output` | Per chunk of terminal output | chunk |
| `on_session_start` / `on_session_end` | session lifecycle | {} |
| `on_memory_write` | Before any memory mutation | allow/deny/transform |
| `provide_memory` | Context build | additional blocks |
| `provide_image` | On `image_generate` | image |
| `slash_command` | User runs `/<plugin-cmd>` | output |
| `dispatch_tool` | Plugin-registered tool invoked | result |
| **`transform_skill`** | NEW v0.2 — Before any skill write | allow/deny/transform |
| **`steer`** | NEW v0.2 — Steer channel notification (advisory) | {} |
| **`judge_outcome`** | NEW v0.2 — Optional pre-judge custom verdict | optional override |
| `shutdown` | On exit | {} |

### 11.3 Plugin → Argo methods (v0.2 additions)

| Method | Purpose |
|---|---|
| `log` / `audit` / `read_secret` / `read_memory` / `dispatch_tool` / `request_capability` / `telemetry_event` | (unchanged from v0.1) |
| **`enqueue_steer`** | NEW v0.2 — plugin sends a steer to a running session |
| **`forge_request`** | NEW v0.2 — plugin requests Forge optimization of a skill |
| **`route_hint`** | NEW v0.2 — plugin suggests a routing tier |

### 11.4 SDK (v0.2 — Go deferred to v1.1; see §27.5)

v1.0 ships **Python** and **TypeScript** SDKs only. Go SDK in v1.1.

### 11.5 Acceptance for §11

- [ ] Reference plugins in Python and TS work end-to-end.
- [ ] Killing a plugin mid-call surfaces `PluginCrash`; loop continues.
- [ ] Protocol-version mismatch rejected cleanly.
- [ ] `transform_skill` veto blocks a skill write before the judge is invoked (saves judge cost on plugin-rejected drafts).

---

## 12. MCP integration — `argo-mcp` + `mcp-servers/` (v0.2 — proper specs for browser/voice/image-gen)

### 12.1 MCP client + server

(Unchanged shape from v0.1; 12 tools; stdio/SSE/HTTP transports.)

### 12.2 Bundled MCP servers (v0.2 — proper specs)

#### 12.2.1 Browser MCP server (`mcp-servers/browser/`) — Amendment 10

- Language: Rust + `headless_chrome` Rust crate (or `chromiumoxide`)
- Default backend: Local Playwright via headless Chromium
- Auto-detects `--remote-debugging-port=9222` Chrome and attaches instead of launching
- Optional cloud backends configured per-server: Browserbase, BrowserUse
- Tools: `navigate`, `click`, `type`, `scroll`, `back`, `press`, `snapshot`, `vision`, `console`, `cdp`, `dialog`, `get_images`
- Capabilities required: `NetEgress { host_glob: "**" }`, `Spawn(Subprocess)`
- LOC budget: ≤ 6,000

Acceptance:
- [ ] Without cloud credentials, all 12 tools work against local Playwright.
- [ ] If Chrome runs with `--remote-debugging-port=9222`, the server attaches; otherwise spawns headless.

#### 12.2.2 Voice MCP server (`mcp-servers/voice/`) — Amendment 12

- Language: Rust
- STT backend: `whisper.cpp` via `whisper-rs` bindings (no ctranslate2 → works on Android)
- TTS default: `piper-rs` (offline, fast, decent quality)
- TTS optional cloud backends: OpenAI TTS (with `instructions` field exposed — fixes #14196), ElevenLabs, MiniMax
- Audio encoding: `opus-rs` for Matrix (fixes #14841), MP3 fallback for everything else
- Tools: `transcribe`, `synthesize`, `voice_chat` (full duplex)
- Capabilities required: `FsRead { path_glob: "*" }` (audio files), `NetEgress` (cloud backends only)

Acceptance:
- [ ] Voice round-trip (record → transcribe → respond → synthesize → play) works on macOS, Linux, Windows, Android (Termux).
- [ ] Matrix voice replies render correctly on iOS/Android Element clients.
- [ ] OpenAI TTS `instructions` field exposed in tool schema.

#### 12.2.3 Image-gen MCP server (`mcp-servers/image-gen/`)

- Language: Python (uses provider SDKs)
- Backends: OpenAI Images, xAI Grok image, OpenAI Codex image
- Tools: `image_generate`, `image_edit`, `image_describe`
- Capabilities required: `NetEgress { host_glob: "<provider-host>" }`, `SecretRead`

#### 12.2.4 GitHub MCP server (`mcp-servers/github/`)

- Language: Go
- Tools: per the existing community standard (`gh`-based)

### 12.3 Acceptance for §12

- [ ] Argo successfully calls an off-the-shelf MCP server with no code changes.
- [ ] An external MCP client (Claude Code) can list and call all 12 server tools.
- [ ] All four bundled MCP servers pass their per-server acceptance gates.
- [ ] Killing any MCP server mid-call surfaces `McpServerCrash`; loop retries once with backoff.

---

## 13. Configuration — `argo-config` (v0.2 — adds routing, judge, RBAC, multi-agent, hard cost caps)

### 13.1 Config tree (v0.2)

```toml
# SPEC v0.2 — argo.toml
[core]
home = "~/.argo"
workspace = "."
profile = "default"

[tenant]
mode = "single"                       # single | multi
default_id = "default"
auth_provider = "none"                # none | oidc | header

[telemetry]
enabled = true
exporter = "stdout"                   # stdout | otlp | none
endpoint = ""

[audit]
enabled = true
chain_algo = "sha256"
export_format = "ndjson"

# === BUDGET (v0.2 — adds hard $$$ caps) ============
[budget]
default_iterations = 60
default_tokens_in = 0
default_tokens_out = 0
default_wallclock_ms = 0
default_spend_usd_micros = 0
default_spend_per_session_usd_micros = 1_000_000     # $1.00
default_spend_per_day_usd_micros    = 10_000_000     # $10.00
default_spend_per_month_usd_micros  = 100_000_000    # $100.00
notify_at_pct = [80, 95]                              # in-app warnings

on_max_iterations = "prompt_user"                     # halt | prompt_user | auto_continue
auto_continue_max_extensions = 3
auto_continue_extension_size = 30

[delegate]
max_spawn_depth = 1
max_concurrent_children = 4
child_default_iterations = 30
budget_inheritance = "explicit"                       # explicit | share
fallback_chain_inheritance = true                     # NEW v0.2 — Amendment 7

[parallel]
max_parallel_tools = 8
destructive_command_serial = true

# === ROUTING (NEW v0.2 — Amendment 1) ==============
[routing]
policy = "fixed"                      # fixed | tiered | policy_plugin

[routing.tiered]
classifier = "openai-compatible:gpt-4o-mini"
simple = "openai-compatible:gpt-4o-mini"
medium = "anthropic:claude-haiku-4-5"
complex = "anthropic:claude-opus-4-6"
classify_threshold_ms = 80
fallback_to_default_on_error = true

# === JUDGE (NEW v0.2 — Amendment 2) ================
[judge]
enabled = true
model = "anthropic:claude-haiku-4-5"
rubric = "default"                    # references rubrics/default.yaml in the workspace
verdict_on_error = "partial"          # partial | failure | success
log_evidence_hashes = true

# === FORGE (NEW v0.2) ==============================
[forge]
enabled = false                       # explicit opt-in
optimizer_model = "anthropic:claude-opus-4-6"
default_examples_min = 3
default_iterations = 12
budget_per_run_usd_micros = 5_000_000  # $5
auto_promote_threshold = 0.92         # final_score for auto-promotion to active set

# === PROVIDERS =====================================
[provider.default]
id = "openai-compatible"
base_url = "https://api.openai.com"
model = "latest"                      # NEW v0.2 — Amendment 8
fallback = "anthropic-claude"

[provider.profiles.anthropic-claude]
id = "anthropic"
model = "latest"

# === GATEWAY (v0.2 — adds RBAC + telegram) =========
[gateway]
enabled = false
adapters = ["slack", "discord", "telegram"]

[gateway.rbac]
default_role = "guest"
# role definitions and bindings — see §10.2 for the four shipped presets

[gateway.rbac.bindings."slack:U01OWNER"]   = "owner"
[gateway.rbac.bindings."slack:U02DEV"]     = "user"
[gateway.rbac.bindings."discord:#dev-help"] = "user"

[gateway.slack]
mode = "socket"
bot_token_secret = "SLACK_BOT_TOKEN"
app_token_secret = "SLACK_APP_TOKEN"

[gateway.telegram]
bot_token_secret = "TELEGRAM_BOT_TOKEN"

# === AGENTS (NEW v0.2 — Amendment 5) ===============
[agents.default]
persona_file = "personas/default.md"
default_model = "latest"
default_toolset = "core"
memory_namespace = "default"

[agents.scribe]
persona_file = "personas/scribe.md"
default_model = "anthropic:claude-haiku-4-5"
default_toolset = "writing"
memory_namespace = "scribe"

# Gateway routing of inbound messages to a named agent:
[gateway.agent_routing.slack]
"U01ALICE" = "scribe"
default = "default"

# === LOOP RESTART POLICY (NEW v0.2 — Amendment 13) =
[loop]
on_restart = "prompt_user"            # prompt_user | auto_resume | discard

# === PLUGINS / MCP (unchanged shape) ===============
[plugin]
auto_load = ["memory-honcho", "obs-langfuse"]
search_paths = ["~/.argo/plugins"]

[mcp.servers.browser]
transport = "stdio"
command = ["argo-mcp-browser"]

[mcp.servers.voice]
transport = "stdio"
command = ["argo-mcp-voice"]

[mcp.servers.image-gen]
transport = "stdio"
command = ["argo-mcp-image-gen"]

# === SKILLS ========================================
[skill]
auto_install = false
hub_default = "github:agentskills-io/skills"
auto_judge = true                     # NEW v0.2 — judge required for reflective writes
```

### 13.2 Acceptance for §13

- [ ] All keys above documented in `docs/ref/config.md`.
- [ ] SIGHUP reloads config without restarting the daemon.
- [ ] Invalid config produces a single clear error pointing to the offending key.
- [ ] `argo config validate` reports unknown keys as warnings, malformed values as errors.

---

## 14. Memory and skills — `argo-memory`, `argo-skill` (v0.2 — adds locks + judge integration)

### 14.1 Memory model

Same four files as v0.1: `MEMORY.md`, `USER.md`, `SOUL.md`, `AGENTS.md`. Content-addressed entries, provenance, supersession chain.

NEW: per-entry `lock` field, persisted in `memory_lock` table.

### 14.2 Skill model (v0.2 — adds locks)

```yaml
# SPEC v0.2 — SKILL.md frontmatter
---
name: jupyter-live-kernel
version: "0.3.1"
description: "Use when the user wants an interactive Jupyter kernel."
capabilities:
  - kind: spawn
    spawn_kind: subprocess
  - kind: net_egress
    host_glob: "localhost"
provenance:
  source: builtin
  hash: sha256-…
lock: none                              # NEW v0.2 — none | user-protected | content-locked
---

[skill body]
```

The agent's `skill_manage` tool checks `lock` before any write. `user-protected` rejects with `ArgoError::UserProtected`. `content-locked` allows write but verifies the new content's lineage hash chain remains intact.

### 14.3 Outcome adjudication (NEW v0.2 — Amendment 2)

Every "reflective phase" skill creation or refinement event MUST go through `argo-judge::Judge::adjudicate` before the `skill_manage` write succeeds. The judge:

1. Receives the original task statement, the agent's output, the tool calls executed, the proposed skill content (or diff), and any optional ground-truth checks.
2. Renders against a rubric (`rubrics/default.yaml`, configurable per-tenant).
3. Returns `Success` / `Partial { reason }` / `Failure { reason }`.
4. Result + evidence hashes persisted to `judge_record`.

```rust
// SPEC
#[async_trait]
pub trait Judge: Send + Sync {
    async fn adjudicate(&self, subject: JudgeSubject, evidence: JudgeEvidence)
        -> ArgoResult<JudgeRecord>;
}

pub struct JudgeEvidence {
    pub task_statement: String,
    pub agent_output: String,
    pub tool_calls: Vec<ToolCallRequest>,
    pub tool_results: Vec<ToolCallResult>,
    pub proposed_artifact: ProposedArtifact,
    pub ground_truth: Vec<GroundTruthCheck>,
}
```

`Partial` writes a draft skill with `version_suffix = "-draft"` to a separate `skill_draft` table; `skills_list` does NOT surface drafts unless `--include-drafts`. The user can promote a draft manually.

### 14.4 Acceptance for §14

- [ ] Round-trip a skill through hub install → use → judge-approved refine → publish, with provenance preserved.
- [ ] Memory entries can be reverted (`argo memory revert <id>`).
- [ ] Bad agent edit to `MEMORY.md` rejected by skills-guard analog; entry not persisted.
- [ ] User-protected lock tested: agent attempts write, denied with clear error.
- [ ] Judge integration tested: deliberate-failure scenario produces `Failure` verdict; deliberate-success produces `Success`.

---

## 15. Sandboxes — `argo-sandbox`

(Unchanged from v0.1: `Sandbox` trait, local + Docker backends. Conformance test suite shared.)

---

## 16. Subagent delegation — `argo-delegate` (v0.2 — adds fallback inheritance)

### 16.1 Token budget tree

(Unchanged from v0.1.)

### 16.2 COW filesystem layer

(Unchanged from v0.1.)

### 16.3 Fallback chain inheritance (NEW v0.2 — Amendment 7)

```rust
// SPEC v0.2
pub struct DelegateOptions {
    pub prompt: String,
    pub toolset: String,
    pub budget: Budget,
    pub model: Option<ProviderModel>,
    pub inherit_fallback: bool,            // NEW; default true
    pub fallback_override: Option<Vec<ProviderModel>>,
}
```

When `inherit_fallback = true` and `fallback_override = None`, the child uses the parent's chain. When `fallback_override = Some(...)`, the override wins (possibly an empty Vec to disable fallback).

### 16.4 Acceptance for §16

- [ ] Property test: total spend across an arbitrary delegation tree never exceeds root budget.
- [ ] Two sibling subagents editing different files merge cleanly.
- [ ] Two sibling subagents editing the same file produce a merge conflict.
- [ ] Child inherits parent's fallback chain by default; explicit override works.

---

## 17. Audit trail — `argo-audit` (v0.2 — adds 11 new event kinds)

### 17.1 New event kinds in v0.2

```
routing.decision        — { id, session_id, turn, policy, tier, classifier_latency_ms, chosen_provider, chosen_model, reason }
judge.verdict           — { judge_record_id, subject_kind, subject_ref, verdict, reason, judge_provider, judge_model }
rbac.deny               — { principal, role, requested_capability }
forge.run.start         — { run_id, skill_name, base_version, examples_count, budget_usd_micros }
forge.run.end           — { run_id, status, candidate_version, final_score, spend_usd_micros }
marketplace.push        — { workspace_id, oci_ref, digest }
marketplace.pull        — { workspace_id, oci_ref, digest }
session.replay          — { source_session, replay_session, model_override, tools_override }
lock.set                — { entity_kind, entity_id, lock, set_by }
lock.violate            — { entity_kind, entity_id, principal, attempted_op }
steer.enqueued          — { session_id, by_principal, text_hash }
```

All chained as before. `argo audit verify` tests the full chain across the new kinds.

### 17.2 Acceptance for §17

- [ ] All 11 new event kinds emit reliably and verify in the chain.
- [ ] `argo audit verify` on a 10k-record corpus including new events succeeds.
- [ ] No event in the canonical list above can occur without a corresponding audit record (instrumented test).

---

## 18. Telemetry — `argo-telemetry` (v0.2 — adds 7 new spans)

### 18.1 Span taxonomy (v0.2 additions)

```
agent.turn
├── agent.context.build
├── routing.classify          NEW v0.2
├── agent.provider.request
│   └── http.request
├── agent.provider.stream
├── agent.tools.plan
├── agent.tools.execute
│   ├── tool.invoke.<name>
│   └── plugin.rpc.<method>
├── judge.adjudicate          NEW v0.2 — when skill candidate detected
└── agent.persist

forge.run                     NEW v0.2 (separate trace tree)
├── forge.bench
├── forge.iterate.N
└── forge.promote

session.replay                NEW v0.2 (separate trace tree)
├── replay.load_source
├── replay.execute
└── replay.diff

marketplace.push              NEW v0.2
└── oci.push.layer.N

marketplace.pull              NEW v0.2
└── oci.pull.layer.N
```

### 18.2 New metrics

```
routing_decisions_total       — counter (by tier)
routing_classify_duration_ms  — histogram
judge_verdicts_total          — counter (by verdict)
judge_duration_ms             — histogram
forge_runs_total              — counter (by status)
forge_spend_usd_micros_total  — counter
spend_cap_blocks_total        — counter (by cap_kind)  — when a hard $ cap blocks a turn
lock_violations_total         — counter
```

### 18.3 Acceptance — unchanged shape; new spans in the test fixture

---

## 19. CLI surface — `argo-cli` (v0.2 — adds forge / replay / agents / workspace push-pull / costs)

```
argo                         # interactive chat
argo new                     # fresh session
argo resume [<id>]           # resume a session
argo agents                  # list named agents in this tenant (NEW v0.2)
argo agents create <name>    # define a new agent
argo agents delete <name>    # remove

argo gateway start|stop|status
argo mcp serve               # run as an MCP server (stdio)
argo acp serve               # run as an ACP server (NEW v0.2)
argo plugin list|install|enable|disable|remove|run
argo provider list|validate|test
argo model list|set|test
argo skill list|view|create|refine|install|lock|unlock   # `lock`/`unlock` NEW v0.2
argo memory list|view|edit|revert|forget|lock|unlock      # `lock`/`unlock` NEW v0.2
argo cron list|create|edit|delete|trigger
argo audit show|verify|export

argo workspace init|fork|diff
argo workspace push <oci_ref>           # NEW v0.2 — push to OCI registry
argo workspace pull <oci_ref>           # NEW v0.2 — pull from OCI registry
argo workspace publish                  # NEW v0.2 — interactive publish wizard

argo db migrate up|down|status
argo doctor

argo trace [--turn|--session]

argo costs                               # NEW v0.2 — current period spend + caps + projection
argo costs cap [--session|--day|--month] <usd>

argo session replay <id>                 # NEW v0.2 — time-travel replay
argo session replay <id> --model <m>     # replay against a different model
argo session replay <id> --tools <set>   # replay against a different toolset
argo session diff <id1> <id2>            # NEW v0.2
argo session bisect <id> <hypothesis>    # NEW v0.2

argo forge optimize <skill> --examples <jsonl>   # NEW v0.2
argo forge bench <skill>                          # NEW v0.2
argo forge promote <skill> --version <v>          # NEW v0.2
argo forge runs list                              # NEW v0.2

argo judge verdict <judge_record_id>     # inspect a verdict
argo judge override <id> --to success    # override a verdict (RBAC: owner only)

argo route inspect <session_id>          # show routing decisions for a session
argo route bench                         # benchmark router on the eval suite

argo steer <session_id> "<text>"         # NEW v0.2 — inject a steer

argo import hermes <path>
argo update
argo --version
argo --help
```

Each subcommand implemented in `crates/argo-cli/src/cmd/<name>.rs`, ≤ 300 LOC each.

### 19.1 Acceptance for §19

- [ ] Every subcommand has `--help` with concrete examples.
- [ ] Tab completion bundled for bash, zsh, fish, PowerShell.
- [ ] `argo doctor` reports config issues, missing dependencies, port conflicts, sandbox availability, **and forge/judge/router subsystem health**.

---

## 20. TUI — `argo-tui`

(Unchanged from v0.1: ratatui + crossterm; sticky composer; live streaming; OSC-52 clipboard.)

NEW v0.2: subagent-spawn observability overlay (Hermes parity feature). Forge run progress overlay. Cost meter in status bar.

---

## 21. Gateway — `argo-gateway` (v0.2 — adds Telegram in-tree + RBAC routing + multi-agent routing)

### 21.1 Adapter trait — unchanged

### 21.2 v1 adapters

| Adapter | Crate | LOC budget |
|---|---|---|
| Slack | `argo-gateway/adapters/slack.rs` | ≤ 1,500 |
| Discord | `argo-gateway/adapters/discord.rs` | ≤ 1,500 |
| **Telegram** | `argo-gateway/adapters/telegram.rs` | ≤ 1,500 |

Email moves to a community plugin (see §27.5 trade-offs).

### 21.3 Multi-tenant + multi-agent routing

```
inbound message (adapter, channel_id, sender_id)
  → tenant resolved via routing table
  → role resolved via RBAC bindings
  → agent_name resolved via [gateway.agent_routing.<adapter>]
  → AIAgent for (tenant_id, agent_name) created or fetched from cache
  → run_turn
```

Cross-platform conversation continuity: same `(tenant_id, agent_name, sender_id)` resumes the same session across adapters.

### 21.4 Acceptance for §21

- [ ] All three adapters survive 24-hour soak (10k messages, no leaks).
- [ ] Cross-platform continuity works.
- [ ] RBAC denial on a tool dispatch surfaces a polite "you need role X" message to the user.
- [ ] Multi-agent routing: same channel routed to different agents based on `[gateway.agent_routing]` config; verified with two distinct named agents in a single test session.

---

## 22. Cron — `argo-cron`

(Unchanged from v0.1.)

---

## 23. Workspace bundles — `argo-marketplace` (v0.2 — adds OCI push/pull)

### 23.1 Workspace = OCI artifact

A workspace bundles `argo.toml`, the four memory files, `skills/`, `cron/`, `secrets.example.toml`, and `personas/`. The `argo workspace push` command:

1. Computes a deterministic tarball with stable byte order.
2. Wraps as an OCI artifact (media type `application/vnd.argo.workspace.v1+tar`).
3. Pushes to the registry via `oci-distribution` Rust crate.
4. Records the push in `marketplace_artifact` and audit log.

Pull verifies the digest, expands into the local workspace path, and offers a 3-way merge if the user has local changes.

### 23.2 Marketplace conventions

- Public registry: `ghcr.io/argo-agents/<workspace-name>:<tag>`
- Naming: lowercase kebab-case workspace names; semver tags + `latest`
- Required README in workspace root if pushing public

### 23.3 Acceptance for §23

- [ ] Workspace round-trips through fork → modify → push → pull on a fresh machine; result is byte-identical.
- [ ] Pulling an already-installed workspace surfaces a 3-way merge UI.
- [ ] Push to GHCR, Docker Hub, and Artifact Registry verified in integration tests against test accounts.

---

## 24. Cost-router — `argo-routing` (NEW v0.2 — Amendment 1)

### 24.1 The trait

```rust
// SPEC
#[async_trait]
pub trait Router: Send + Sync {
    async fn route(&self, ctx: &RoutingContext) -> ArgoResult<RoutingDecision>;
}

pub struct RoutingContext<'a> {
    pub session_id: SessionId,
    pub turn: u32,
    pub messages: &'a [Message],
    pub user_message: &'a str,
    pub tools_in_scope: &'a [String],
    pub default_model: ProviderModel,
    pub policy: RoutingPolicy,
    pub deadline: Instant,
}
```

### 24.2 Built-in routers

- `FixedRouter` — always returns the default model. Zero overhead.
- `TieredRouter` — runs the configured `classifier_model` with a fixed prompt:

> Classify the following user request as `simple`, `medium`, or `complex`. Reply with the single word only. Request: """<truncated to 800 tokens>"""

  Routes to the matching tier. If classifier exceeds `classify_threshold_ms`, falls through to `default_model` and logs a warning.

- `PolicyPluginRouter` — delegates to a plugin via `route_hint` RPC.

### 24.3 Caching

The router caches `(session_id, last_user_message_hash) → tier` for 60 seconds to avoid re-classifying retries.

### 24.4 Acceptance for §24

- [ ] `routing.classify` p95 ≤ 80 ms on a Haiku-class classifier model.
- [ ] Eval task suite `routing-correctness` (20 tasks) passes ≥ 80% with the default Tiered config.
- [ ] Classifier crash → falls through to default; never halts the loop.
- [ ] Routing decisions persisted in `routing_decision` table for every turn when policy ≠ Fixed.

---

## 25. Judge — `argo-judge` (NEW v0.2 — Amendment 2)

### 25.1 The trait

(See §14.3 for the full surface.)

### 25.2 Default rubric

`rubrics/default.yaml`:

```yaml
# SPEC v0.2 — default judge rubric
checks:
  - id: task_completed
    weight: 0.4
    prompt: |
      Did the agent's output complete the user's task as stated?
      Compare the task statement and the final output. If the agent
      claims success but the output doesn't match the task, this is FAIL.
  - id: tool_calls_grounded
    weight: 0.2
    prompt: |
      Are the agent's claims grounded in actual tool results?
      Hallucinated success (claiming a tool returned X when it returned Y or
      errored) is FAIL.
  - id: skill_quality
    weight: 0.3
    prompt: |
      If a skill is being created/refined, is the proposed skill content
      generic enough to apply to similar future tasks AND specific enough
      to provide value? Boilerplate-only skills are FAIL.
  - id: side_effects
    weight: 0.1
    prompt: |
      Did the agent cause unintended side effects (deleted files,
      destructive commands, leaked credentials)? Any unintended
      destructive side effect is FAIL.

verdict_thresholds:
  success: 0.8
  partial: 0.5
  # below 0.5 → failure
```

The judge model receives the rubric and structured evidence, returns a per-check score and rationale. Aggregate score is the weighted sum; verdict from thresholds.

### 25.3 Acceptance for §25

- [ ] Eval suite `judge-disagreement` (15 tasks where the worker model would self-judge wrongly) shows the judge correctly downgrades to `Partial` or `Failure` ≥ 80% of the time.
- [ ] Judge latency p95 ≤ 1.5 s with Haiku-class.
- [ ] Judge overrides surfaced in audit log; require role `owner` to invoke `argo judge override`.

---

## 26. Argo Forge — `argo-forge` (NEW v0.2 — Phase 7.5)

### 26.1 What it is

A standalone optimization subsystem that improves Argo skills using **DSPy + GEPA** (Genetic-Pareto Prompt Evolution; ICLR 2026 Oral). No GPU. ~$2-10 per run. Works with as few as 3 examples.

### 26.2 Surface

```
argo forge optimize <skill> --examples <eval/<task>.jsonl> [--iterations 12] [--budget 5]
argo forge bench <skill>
argo forge promote <skill> --version <candidate_version>
argo forge runs list
argo forge runs show <run_id>
argo forge runs abort <run_id>
```

### 26.3 Pipeline

1. Load skill at base version.
2. Load examples from JSONL.
3. Run baseline evaluation: `argo eval run --skill <skill>` against examples; record baseline score.
4. GEPA loop:
   - Mutate the skill body, frontmatter, or tool descriptions per genetic operators (paraphrase, prune, expand, swap-example, reorder).
   - Evaluate each candidate against examples.
   - Select top-K on the Pareto front of (score, length).
   - Read execution traces from failed runs to understand WHY (DSPy reflective component).
   - Iterate.
5. Output: `<skill>-N+1` candidate version + score delta + diff.
6. If `final_score >= [forge].auto_promote_threshold`, auto-promote (write to active set with judge approval). Else, hold as draft.

### 26.4 Implementation

`argo-forge` is a separate binary spawned as a subprocess (so the CLI doesn't block). Communicates with the core via `forge_request` RPC + audit events.

Heavy lifting reuses Python via `argo-plugin-py`'s SDK in a "forge worker" plugin. The Python implementation can pull DSPy directly. Argo provides the orchestration; DSPy provides the algorithm.

### 26.5 Acceptance for §26

- [ ] Optimize a skill with 5 examples in ≤ 8 min p95, ≤ $5.
- [ ] Optimization improves the eval score on the held-out set in ≥ 70% of runs (matching reported GEPA performance).
- [ ] Forge runs are budget-capped: hitting the budget cleanly aborts and persists partial state.
- [ ] Auto-promote requires both `final_score >= threshold` AND a passing judge verdict on the candidate.

---

## 27. Phased roadmap (v0.2 — adds Phase 4.5, Phase 6.5, Phase 7.5)

### Phase 0 — Skeleton (week 1)

(Unchanged from v0.1. Adds the 6 new crate stubs.)

**Gate:** `cargo test --workspace` green; CI green on three OSes; LOC budget enforced.

### Phase 1 — Walking skeleton + budget framework (weeks 2-3)

- v0.1 walking skeleton items
- **NEW v0.2:** Budget tree with hard $$$ caps, `on_max_iterations` policy plumbed through the loop

**Gate:** end-to-end test (user → file_read → answer); cold boot < 200 ms; spending $0.05 → block at $0.05.

### Phase 2 — Provider breadth + cost router (weeks 4-6)

- 4 v1 manifests
- 6 tool-call format extractors
- Streaming, retry, fallback, **adaptive heartbeat timeouts (Amendment 6)**
- **NEW v0.2:** `argo-routing` with Fixed + Tiered + PolicyPlugin; eval suite `routing-correctness`
- **NEW v0.2:** `model = "latest"` resolution (Amendment 8)

**Gate:** ≥ 85% pass on `core` and `routing-correctness`; provider switch is a config edit.

### Phase 3 — Tools, memory, skills, judge (weeks 7-9)

- 8 in-core tools
- Memory store with locks
- Skill store with locks
- **NEW v0.2:** `argo-judge` adjudicator; eval suite `judge-disagreement`
- Workspace fork/diff

**Gate:** ≥ 85% pass on `core` + `memory` + `judge-disagreement`. Bad-edit guard catches synthetic corruption.

### Phase 4 — Plugins + MCP (weeks 10-12)

- Plugin protocol stable; Python + TS SDKs
- 3 reference plugins
- MCP client (stdio/SSE/HTTP)
- MCP server (12 tools)
- **NEW v0.2:** Plugin protocol additions (`steer`, `transform_skill`, `judge_outcome`)

**Gate:** plugin chaos test green; off-the-shelf MCP servers work; external client (Claude Code) drives Argo.

### Phase 4.5 — ACP + bundled MCP servers (weeks 13-14) — NEW v0.2

- **NEW v0.2:** `argo-acp` server with multi-harness support (Amendment 11)
- **NEW v0.2:** Browser MCP server with local Playwright default (Amendment 10)
- **NEW v0.2:** Voice MCP server with whisper.cpp + piper-rs (Amendment 12)
- **NEW v0.2:** Image-gen MCP server (3 backends)

**Gate:** VS Code/Zed/JetBrains all drive Argo via ACP; voice round-trip works on macOS/Linux/Windows/Android; browser tools work locally with no Browserbase.

### Phase 5 — Delegation + sandboxes (weeks 15-16)

- Token budget tree
- COW filesystem
- Local + Docker sandboxes
- **NEW v0.2:** `delegate_task` inherits parent fallback chain (Amendment 7)

**Gate:** budget tree property tests pass; sibling-merge tests pass.

### Phase 6 — Gateway + multi-agent + RBAC (weeks 17-19)

- Slack, Discord, **Telegram (promoted in-tree v0.2)**
- Cross-platform continuity
- Cron with delivery
- Multi-tenant routing
- **NEW v0.2:** Multi-agent per tenant (Amendment 5)
- **NEW v0.2:** Gateway RBAC tiers (Amendment 4)
- **NEW v0.2:** Mid-run `/steer` (Amendment 14)
- **NEW v0.2:** Resume interrupted on restart (Amendment 13)

**Gate:** 24-hour soak across all three adapters with multi-agent routing; ≥ 1,000 concurrent sessions; RBAC denial UX clean.

### Phase 6.5 — Workspace marketplace (week 20) — NEW v0.2

- `argo-marketplace` push/pull to OCI
- 3-way merge UI on conflict pull
- Public reference workspace published

**Gate:** push → fresh machine pull → byte-identical.

### Phase 7 — Polish + replay + perf + docs (weeks 21-22)

- Ratatui TUI
- All NFR targets met
- Docs site complete; auto-extracted compiled examples
- `argo doctor` covers all known failure modes
- `argo import hermes` round-trips a real Hermes installation
- **NEW v0.2:** Time-travel session replay (`argo session replay`, `diff`, `bisect`)

**Gate:** all NFRs green; eval ≥ 90% on core + multi-turn + memory + judge-disagreement + routing-correctness.

### Phase 7.5 — Argo Forge (weeks 23-24) — NEW v0.2

- `argo-forge` standalone binary
- Python forge worker plugin (DSPy + GEPA)
- `argo forge optimize/bench/promote/runs`
- Default off; explicit opt-in; budget caps

**Gate:** optimize a reference skill with ≥ 70% improvement on held-out set in ≤ 8 min and ≤ $5; auto-promote requires judge approval; full audit chain.

### Phase 8 — Post-1.0 (open-ended)

- v1.1: Email gateway adapter; Go plugin SDK; SSH/Modal/Daytona/Singularity sandboxes as plugins; remaining 14 messaging platforms as plugins maintained by community partners
- v1.5: Full web dashboard; native iOS/Android; expanded marketplace
- v2.0: Federated opt-in skill sharing network; OCI workspace marketplace federation; capability negotiation between agents over MCP

### 27.5 v0.1 → v0.2 trade-offs (what we cut to make room)

To accept the +4-5 weeks scope additions (Phase 4.5 + Phase 6.5 + Phase 7.5 = 5 weeks) without an unbounded slip, we cut from v1.0 in-tree:

| Dropped from v1.0 in-tree | Where it ships instead | Why this is safe |
|---|---|---|
| Email gateway adapter | v1.1 (or community plugin) | Lower-frequency channel; SMTP is "everywhere" via plugin |
| Go plugin SDK | v1.1 | Python + TS cover ≥ 95% of plugin authors |
| SSH sandbox | v1.1 plugin | Adoption of SSH backend is small per Hermes telemetry |
| Singularity sandbox | v1.1 plugin | HPC-specific; tiny audience |
| Modal sandbox | v1.1 plugin | Relies on a single SaaS; better as a plugin |
| Daytona sandbox | v1.1 plugin | Same |

Net: same `cargo build --release` time, smaller binary, more focused initial release, more headroom for the new subsystems.

What we **promoted** in v0.2 to in-tree (justifies the LOC budget bump):

| Promoted | From | Why |
|---|---|---|
| Telegram gateway | Plugin | Highest-usage messaging platform; users expect it |
| Voice MCP server | Plugin idea | We can ship a working voice mode that even Hermes can't (Android works) |
| Browser MCP server | Plugin idea | Issue #374 is one of the longest-standing user pains |
| ACP server | Out of scope | VS Code/Zed/JetBrains adoption is real; easier to ship multi-harness now than retrofit |

---

## 28. Coding-agent dispatch plan (v0.2 — adds Agents V through CC)

The universal system prompt (in `argo-day-0-kickoff.md` §1) applies to every agent. The phase-by-phase agent list:

### Phase 0 (Agent A): Repo scaffolder

(Unchanged from v0.1; bootstrap.sh now creates 26 crate stubs.)

### Phase 1 (Agents B, C, D): Type system, store, capability

(Unchanged from v0.1 in shape; budget tree gets new fields from §5.4; capability engine gets the role table from §10.1.)

### Phase 2 (Agents E, F, V): Provider manifests, transports, ROUTING (NEW)

- Agent E: `argo-provider` manifests + `openai-compatible` transport (unchanged)
- Agent F: Anthropic + Bedrock + Responses transports (unchanged; adds heartbeat-driven timeout per §6.4)
- **Agent V (NEW v0.2): `argo-routing`** — see prompt in §28.V below

### Phase 3 (Agents G, H, W): Tools, memory/skills, JUDGE (NEW)

- Agent G: 8 in-core tools (unchanged)
- Agent H: memory + skills with locks (additions in §14)
- **Agent W (NEW v0.2): `argo-judge`** — see prompt in §28.W

### Phase 4 (Agents I, J): Plugins, MCP

(Unchanged; plugin protocol additions in §11.2/§11.3 baked in.)

### Phase 4.5 (Agents Y, Z, AA): ACP, voice, browser (NEW)

- **Agent Y: `argo-acp`** — see §28.Y
- **Agent Z: voice MCP server** — see §28.Z
- **Agent AA: browser MCP server** — see §28.AA

### Phase 5 (Agents K, K2): Delegation, sandboxes

(Unchanged; child fallback inheritance per §16.3.)

### Phase 6 (Agents L, L2): Gateway, RBAC, multi-agent

- Agent L: Slack/Discord/Telegram adapters
- **Agent L2 (NEW v0.2):** RBAC role engine wiring + multi-agent routing + `/steer` + resume-on-restart

### Phase 6.5 (Agent CC): Workspace marketplace (NEW)

- **Agent CC: `argo-marketplace`** — see §28.CC

### Phase 7 (Agents M, N, O, BB): TUI, eval, telemetry, REPLAY (NEW)

- Agent M: TUI (unchanged)
- Agent N: eval harness — adds `judge-disagreement` and `routing-correctness` suites
- Agent O: telemetry + audit (unchanged shape; new spans/events)
- **Agent BB (NEW v0.2): `argo-replay`** — see §28.BB

### Phase 7.5 (Agent X): Argo Forge (NEW)

- **Agent X: `argo-forge`** — see §28.X

### 28.V — Agent V — Cost-tier router

```
TASK: Implement argo-routing per §24 of argo-build-plan.md.

DEPENDS ON: argo-types (B), argo-store (C).

WHAT TO DO:
1. Implement Router trait with three concrete impls: FixedRouter, TieredRouter, PolicyPluginRouter.
2. TieredRouter: classifier prompt is a fixed template (see §24.2). Use the configured small model. p95 latency must hit ≤ 80 ms NFR (§1.3).
3. Cache (session_id, last_user_message_hash) → tier for 60s.
4. Persist every non-Fixed decision to routing_decision table.
5. Author 20 routing-correctness eval tasks under eval/tasks/routing-correctness/.
6. Hook into argo-core::Agent::run_turn so that LoopState::Routing fires before AwaitingProvider when policy != Fixed.

GATE — §24.4:
- [ ] Classify p95 ≤ 80 ms with Haiku-class
- [ ] routing-correctness eval ≥ 80% pass
- [ ] Classifier crash falls through to default; never halts
- [ ] All non-Fixed decisions persisted

Stop at the gate.
```

### 28.W — Agent W — External-judge adjudicator

```
TASK: Implement argo-judge per §14.3, §25 of argo-build-plan.md.

DEPENDS ON: argo-types (B), argo-store (C), argo-provider (E).

WHAT TO DO:
1. Implement Judge trait with one concrete impl: ModelBackedJudge.
2. Build the default rubric at rubrics/default.yaml (§25.2).
3. Wire into argo-skill: every `skill_manage` write of a "reflective" candidate
   MUST round-trip through Judge::adjudicate before persistence.
4. On Partial: write to skill_draft (separate table); skills_list does not surface drafts by default.
5. On Failure: discard candidate; persist judge_record for audit.
6. Author 15 judge-disagreement eval tasks under eval/tasks/judge-disagreement/ —
   designs where the worker would self-judge incorrectly.
7. Implement `argo judge override` (RBAC: owner only).

GATE — §25.3:
- [ ] judge-disagreement eval ≥ 80% correct downgrades
- [ ] Latency p95 ≤ 1.5 s with Haiku-class judge
- [ ] Judge overrides require role 'owner' and emit audit record
```

### 28.X — Agent X — Argo Forge

```
TASK: Implement argo-forge per §26 of argo-build-plan.md.

DEPENDS ON: Phase 0-7 complete (this is Phase 7.5).

WHAT TO DO:
1. Implement argo-forge as a SEPARATE BINARY (not a library inside the core).
   The CLI subcommands `argo forge ...` exec into it.
2. The optimization algorithm runs in Python (DSPy + GEPA). Ship a Python forge
   worker as a plugin (sdk/argo-plugin-py-based). The Rust binary orchestrates;
   Python does the heavy lifting.
3. Pipeline per §26.3.
4. Budget cap: every run has a hard $ cap (default $5). Hitting it aborts cleanly.
5. Auto-promote ONLY if final_score >= threshold AND judge approves.
6. Persist every run to forge_run table; emit forge.run.start/end audit.
7. Telemetry per §18.1.

GATE — §26.5:
- [ ] Optimize a reference skill, 5 examples, ≤ 8 min p95, ≤ $5
- [ ] Improvement on held-out ≥ 70% of runs
- [ ] Hitting budget aborts cleanly; partial state persisted
- [ ] Auto-promote requires both score threshold AND judge approval
```

### 28.Y — Agent Y — ACP server

```
TASK: Implement argo-acp per §12 of argo-build-plan.md and the ACP spec.

DEPENDS ON: argo-core (G), argo-capability (D).

WHAT TO DO:
1. Implement an ACP-compliant server speaking the protocol from spec
   (see acp.spec at agentclientprotocol.com or equivalent).
2. Multi-harness from day 1: GitHub Copilot CLI, Cursor agent CLI, generic ACP.
   No hardcoded single-harness limitation. Configuration drives which harnesses
   to advertise capabilities for.
3. Authentication: support Copilot OAuth, Cursor token, generic bearer.
4. Capabilities exposed: send message, receive stream, list sessions, fork session.
5. Map ACP "Permissions" requests to Argo's CapabilityEngine.

GATE:
- [ ] VS Code, Zed, JetBrains all drive Argo via ACP
- [ ] Cursor agent CLI works (resolves Hermes #16282)
- [ ] Permission requests resolve through the same approval flow as CLI
```

### 28.Z — Agent Z — Voice MCP server

```
TASK: Implement mcp-servers/voice/ per §12.2.2.

WHAT TO DO:
1. Pure Rust. Bindings: whisper-rs (whisper.cpp) for STT, piper-rs for TTS,
   opus crate for OGG/Opus encoding.
2. Cloud TTS optional backends (configured per server.toml): OpenAI TTS
   (with `instructions` field exposed), ElevenLabs, MiniMax.
3. MCP tools: transcribe, synthesize, voice_chat.
4. Audio output by default OGG/Opus; MP3 fallback.

GATE — §12.2.2:
- [ ] Voice round-trip on macOS, Linux, Windows, Android (Termux)
- [ ] Matrix iOS/Android Element clients render replies cleanly
- [ ] OpenAI TTS instructions field exposed in tool schema
```

### 28.AA — Agent AA — Browser MCP server

```
TASK: Implement mcp-servers/browser/ per §12.2.1.

WHAT TO DO:
1. Pure Rust + chromiumoxide (or headless_chrome). Local Playwright is the default.
2. Auto-detect Chrome with --remote-debugging-port=9222 (resolves Hermes #6780).
3. Cloud backends configurable: Browserbase, BrowserUse.
4. 12 tools per Hermes parity.
5. LOC budget ≤ 6,000.

GATE — §12.2.1:
- [ ] All 12 tools work without cloud credentials (resolves Hermes #374)
- [ ] If Chrome runs with debug port, attach instead of spawn
- [ ] Cloud backends opt-in only
```

### 28.BB — Agent BB — Time-travel replay

```
TASK: Implement argo-replay per §19 (CLI) + new functionality.

DEPENDS ON: argo-store (C), argo-provider (E), argo-core (G).

WHAT TO DO:
1. Replay a session deterministically from session_db state.
2. Per-call overrides:
   --model <other model>: rebuild requests against a different provider/model
   --tools <toolset>: rebuild context with different tools advertised
   --workspace <path>: replay against a different workspace state
3. argo session diff <id1> <id2>: structural diff (turn count, tool calls, output)
4. argo session bisect <id> "<hypothesis>": find first turn where the hypothesis
   is violated by walking turns and re-running each through the judge

GATE:
- [ ] Replay reproduces original verbatim when no overrides given
- [ ] Replay with model override succeeds and surfaces score delta via judge
- [ ] Diff is human-readable
- [ ] Bisect terminates correctly on synthetic divergence cases
```

### 28.CC — Agent CC — Workspace marketplace

```
TASK: Implement argo-marketplace per §23.

DEPENDS ON: argo-store (C), argo-skill (H), argo-memory (H).

WHAT TO DO:
1. Implement deterministic workspace tarball builder (stable byte order,
   stripped timestamps, sorted entries).
2. Wrap as OCI artifact with media type application/vnd.argo.workspace.v1+tar.
3. Push via oci-distribution Rust crate; pull verifies digest.
4. 3-way merge UI when pulling onto a modified local workspace.
5. Publish a reference workspace to ghcr.io/argo-agents/example/quickstart:1.0
   as part of the release pipeline.

GATE — §23.3:
- [ ] Round-trip push → fresh machine pull → byte-identical
- [ ] 3-way merge surfaces clearly
- [ ] Push verified against GHCR, Docker Hub, Artifact Registry test accounts
```

### Coordinator (you)

> Track phase gates from §27. Hold the line on the five invariants. Reject any PR that:
> - adds a tool to in-core that should be an MCP server
> - adds a provider as code instead of a manifest
> - exceeds an LOC budget
> - violates the capability model
> - introduces a global
> - misses an acceptance criterion
> - skips the judge for reflective skill writes
> - bypasses the budget tree
>
> Subtract before adding.

---

## 29. Risk register (v0.2 — adds 6 new risks)

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Manifest model can't express a real provider quirk | Medium | High | Hooks escape hatch in §8.1 |
| Plugin protocol drift between SDKs | Medium | High | Single source-of-truth schema; SDKs generated |
| MCP ecosystem instability | Low | Medium | Pin versions; ship reference critical tools |
| Agent self-edit corrupts memory/skills | Low | High | Capability gating + locks + judge + reversible audit |
| Capability model too restrictive → users disable it | Medium | High | Sensible defaults; one-line policy bundles |
| Rust onboarding deters Python contributors | Medium | Medium | Python plugin SDK first-class |
| LOC budgets fail under real implementation | Medium | Medium | Re-baseline at Phase 3; peel off sub-crates |
| Multi-tenant performance overhead | Low | High | Bench from Phase 1; per-tenant DBs |
| Schema migration bugs | Low | High | Round-trip property tests on every migration |
| Audit-chain perf at scale | Low | Medium | Append-only batch writes; bench at 100k |
| Windows-native parity gaps | Medium | Medium | CI runs Windows from Phase 0 |
| **Cost-router classifier wrong on edge cases** | Medium | Medium | Per-task overrides; route_hint plugin RPC |
| **Judge disagrees too often, blocks skill creation** | Medium | Medium | Three-state outcome (Partial → draft path) |
| **Forge runs expensive without supervision** | Medium | High | Default off; budget caps; sample-only mode |
| **Multi-agent confusion ("which agent answered?")** | Low | Medium | Always tag agent name in gateway responses |
| **OCI marketplace abuse (malicious workspaces)** | Medium | High | Workspace install requires explicit `--trust <ref>` to grant elevated capabilities; default install is sandboxed |
| **Telegram in-tree promotion increases adapter drift surface** | Medium | Low | LOC budget enforced; protocol changes tracked via heartbeat tests |

---

## 30. Definition of "stop working"

You stop only when **every** condition is true:

1. Every box in §2 (Definition of Done) is checked, including all v0.2 additions.
2. Every gate in §27 has its acceptance criteria green.
3. Every NFR in §1.3 is met or beaten, including all 6 new v0.2 NFRs.
4. Eval harness reports ≥ 90% on `core` + `multi-turn` + `memory` + `judge-disagreement` + `routing-correctness`.
5. New contributor: install → first conversation in < 90 s.
6. `argo import hermes <path>` round-trips a real Hermes installation.
7. `argo workspace pull ghcr.io/argo-agents/example-quickstart:1.0` works on a fresh machine.
8. **Argo Forge optimizes a reference skill end-to-end** with judge approval and audit chain integrity.

If you find yourself wanting to stop earlier, you have not stopped — you have given up. **Ship.**

---

## Appendix A — Glossary (v0.2 additions)

- **Argo Forge** — the DSPy/GEPA-style automated skill optimizer subsystem.
- **Argo Replay** — the time-travel debug subsystem.
- **Argo Marketplace** — workspace bundles as OCI artifacts pushed/pulled via standard registries.
- **Cost-tier router** — the subsystem that classifies prompt complexity and routes to a per-tier model.
- **Judge** — the external-LLM adjudicator that approves or rejects reflective skill creation/refinement.
- **Lock** (on a skill or memory entry) — `none` | `user-protected` | `content-locked`. Controls whether the agent may edit it.
- **Role** — a named capability set bound to principals (gateway users, ACP harnesses, the Forge subsystem).
- **Steer** — a mid-run injection of guidance the running agent sees after its next tool call without breaking prompt cache.

## Appendix B — References

- Hermes Agent — github.com/NousResearch/hermes-agent
- Hermes Agent Self-Evolution (DSPy + GEPA, ICLR 2026 Oral) — github.com/NousResearch/hermes-agent-self-evolution
- Model Context Protocol — modelcontextprotocol.io
- agentskills.io — skill format compatibility target
- OpenTelemetry — opentelemetry.io
- ratatui — ratatui.rs
- DSPy — dspy.ai
- GEPA paper — ICLR 2026
- OCI distribution spec — github.com/opencontainers/distribution-spec

## Appendix C — Document discipline

This document is normative. Edits to spec sections must:
1. Bump the document version.
2. List the affected spec sections.
3. Cross-reference a passing eval-suite delta or NFR-bench delta.

If §2's Definition of Done is changed, every coding-agent prompt in §28 is reviewed for impact.

---

**End of plan v0.2. Begin execution.**
