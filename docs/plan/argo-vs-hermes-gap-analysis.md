# Argo vs Hermes — Gap Analysis & User-Validated Plan Amendments

**Date:** April 29, 2026
**Author:** Prepared for Mark Hallam (Acmeco)
**Purpose:** Pressure-test the Argo build plan against real Hermes user signal. Identify gaps (both directions). Propose specific, line-item amendments to the plan.

> "The most dangerous requirements come from smart people, because you're less likely to question them. Question them anyway."

---

## 0. Executive summary

Argo's architectural thesis — Rust core, multi-tenant, capability-based, manifest-driven, MCP-first — is **directly validated** by the most-cited Hermes user pain points. The token-overhead crisis (~73% fixed overhead, ~13.9 K tokens before the user has typed anything), the Honcho 60-second startup block, the WSL2-only Windows situation, the Tirith hard-block UX, and the Browserbase paywall are all consequences of structural choices Argo deliberately rejects.

But the plan as written **misses six concrete things** users are explicitly asking for, and **underweights three classes of feature** the Hermes team itself is trying to retrofit:

1. **Self-evaluation honesty** — the agent thinks it succeeded when it didn't. Highest-frequency user complaint after cost. Argo's plan does not currently solve this.
2. **Cost-routing tiers built-in** — community has shipped NadirClaw to fix this; users pay €90/mo where they could pay €15. Argo's budget tree handles caps but not intelligent routing.
3. **Skill protection from agent self-edit** — "self-improvement clobbers my customizations" is the second-most-voted complaint on skills.
4. **Gateway-level RBAC tiers** (Owner/Admin/User/Guest) — open issue #527, community has built `hermes-control-interface` to fill this gap.
5. **Native multi-agent in one gateway** — open issue #7517 — multiple named agents per tenant, isolated state.
6. **Adaptive provider timeouts** — bug #5889 (180-s reconnect kills long-running calls).

And three classes of "beyond Hermes" opportunity backed by **explicit user evidence and citations**:

- **A. Agent self-evaluation by external judge** (solves complaint #1).
- **B. DSPy/GEPA-style automated skill optimization** (Nous's own ICLR 2026 Oral sibling repo `hermes-agent-self-evolution`; nobody ships this in-tree).
- **C. Federated, opt-in skill sharing** — workspace marketplace as OCI artifacts.

Section 4 lists 14 amendments to the Argo build plan, each tied to a cited user issue or thread. Section 5 ranks the beyond-Hermes opportunities by user-evidence + impact. Section 6 is the **delta diff against `argo-build-plan.md`** — the lines to change.

---

## 1. Methodology and sources

### 1.1 What I did

I pulled signal from six categories of source over the past hour: the Hermes GitHub issue tracker, GitHub Discussions, Reddit and Hacker News threads, X/Twitter discussion, third-party blog reviews (DEV.to, Medium, Substack, Hostinger, Bluehost, Kingy, Hermes Atlas), and the sibling open-source projects Hermes Research itself maintains (`hermes-agent-self-evolution`, `hermes-control-interface`).

The cited issues that follow are real numbered issues on `github.com/NousResearch/hermes-agent`. Quotes are paraphrased from the search-result excerpts; if you want to verify a specific quote, the issue number is in every citation.

### 1.2 What I did *not* do

- I did not access Discord (not searchable).
- I did not access private user Slack channels.
- I did not run a quantitative survey.
- I did not interview Hermes users directly.

If you want a level-up on this, I'd run a 10-question quantitative survey across the Hermes Discord with 100+ responses and re-baseline. That's a 1-2 day add. Worth doing before locking the v1 scope.

### 1.3 Confidence calibration

- **High confidence** on token overhead, startup latency, install pain, RBAC ask, multi-agent ask, provider-fallback bug — all cited by name with issue numbers and reproduced across multiple sources.
- **Medium confidence** on the "self-evaluation thinks it succeeded" theme — confirmed by Reddit threads, a Medium "switched from OpenClaw" article, and a Substack analysis, but I'd want quantitative survey data before committing platform-level fixes.
- **Lower confidence** on "skill auto-overwrite" frequency — confirmed as a complaint pattern but I don't know if it's 5% or 50% of skill-using users.

---

## 2. What Hermes users actually ask for

I sorted ~50 distinct signals into nine themes, each ranked by **citation frequency × severity × my confidence**.

### 2.1 The token overhead crisis (Severity: critical, Confidence: high)

**Issue #4379 — "Token overhead analysis: 73% of each API call is fixed overhead (~13.9K tokens)"**. Quoted analysis from the issue: total baseline ~19K tokens per request *before the user types anything*. Sent on every call regardless of model. 31 tools × full schema ≈ 8,700 tokens. A simple weather query reaches 21,000 tokens because the model decides to use the terminal tool.

User-reported real costs:
- "72M tokens consumed in 2 days of testing" (one OpenRouter user)
- "$150-200 in 2 weeks debugging skills"
- "4M tokens in 2 hours of light usage" (Telegram gateway debug loop)
- "21,000 tokens just asking about the weather"
- 2-3× cost difference between CLI (6-8K tokens/req) and Telegram gateway (15-20K tokens/req)

Community fix: NadirClaw, a third-party LLM proxy that classifies prompt complexity and tier-routes (cheap models for simple, expensive only for complex). Brings €90/mo down to €15-30/mo.

**This is the single largest user-experienced cost of Hermes.** It is structural — driven by 31 in-tree tools, a fat memory provider chain, and a system prompt with all skills' frontmatter loaded.

### 2.2 The "agent thinks it succeeded when it didn't" problem (Severity: critical, Confidence: medium-high)

Reddit and Medium quotes, paraphrased:
- *"It always thinks it did a good job. ALWAYS. I had it pull water test results from the Indiana DNR site and it jumbled up everything... It thought it kicked ass!"*
- *"The reflective phase generates skills from runs that didn't work — the agent's self-assessment is unreliable, so the skill library accumulates noise."*

Mechanism: the "Reflective Phase" that auto-generates skills relies on the same model self-evaluating its output. It's a closed loop with no external grounding. The agent self-judges biased-positive, writes a skill anyway, and now future tasks pull a noisy skill into context.

This is **not** something Hermes addresses today, and the architecture makes it hard to retrofit (the reflective LLM call is the same instance as the worker LLM).

### 2.3 Skills auto-overwrite user customizations (Severity: high, Confidence: medium)

Quoted: *"If I spent time tuning a specific skill for my smart home or a workflow, having an agent 'self-improve' it back into a jumbled mess sounds like a nightmare."*

Mechanism: `tools/skill_manager_tool.py` exposes write/edit to the model. `tools/skills_guard.py` does some sanity checks but does not honour user "this is hand-tuned, don't auto-edit" markers. Power users who tune skills are punished by the self-improvement loop.

### 2.4 Setup and provider auth complexity (Severity: high, Confidence: high)

Quoted: *"the auth model is the thing to learn first" — 19 first-class providers, 3 distinct auth paths, and "the setup docs assume knowledge that beginners don't have."*

Sub-issues:
- Native Windows install attempted instead of WSL2 (most common stumble).
- Python 3.13 incompatibility (pathlib, tiktoken pyo3 errors).
- `~/.zshrc` permission-denied after install.
- Gateway PATH capture stale after later tool installs.
- Provider auto-detection ignores `model.provider` in config (issue #4172) — users billed for OpenRouter when they configured a local endpoint.

### 2.5 Browser automation gated by Browserbase paywall (Severity: high, Confidence: high)

**Issue #374 — "Local Browser Backend via Playwright/CDP — Zero-Cost Alternative to Browserbase".** When Browserbase credentials are absent, *all 10 browser tools fail silently*. The agent has Playwright as a dependency but the fallback path is broken.

**Issue #6780 — "Auto-detect local Chrome CDP on port 9222"**. When user has Chrome with `--remote-debugging-port=9222`, the agent ignores the live browser and launches a separate headless Chromium.

Community alternative: Pinchtab (12 MB Go binary, Chrome over HTTP). Issue #493 proposes integrating it.

### 2.6 RBAC and multi-tenant gateway gaps (Severity: high, Confidence: high)

**Issue #527 — "Gateway Permission Tiers — Owner/Admin/User/Guest"**. Asked for explicitly. Community workaround: `xaspx/hermes-control-interface` (self-hosted dashboard with multi-user auth + RBAC).

**Issue #3897 — "Per-user tool restrictions in gateway (RBAC)"**. Asked for; not shipped.

**Issue #7517 — "Native Multi-Agent Support"**. Single gateway process, multiple named agents with isolated sessions, personas, memory, tool configurations. Asked for; not shipped.

### 2.7 Provider/transport reliability bugs (Severity: critical for affected users, Confidence: high)

- **Issue #15914 (P1)** — "Fallback chain collapses when API key unavailable in `os.environ` but present in `credential_pool`". Major-feature-broken, no workaround.
- **Issue #4172** — "Provider auto-detection ignores `model.provider` in config.yaml — stale API keys override explicit custom endpoint." Users billed wrong.
- **Issue #5889** — "Hermes reconnects after 180s of provider silence even though oMLX is still processing." Long-running provider work gets killed mid-call.
- **Issue #7481** — "Per-delegation fallback provider chain": `delegate_task` does not inherit `fallback_providers`. Single point of failure.

### 2.8 Memory / startup blocking (Severity: high for affected users, Confidence: high)

- **Issue #5726** — "Honcho memory provider blocks initialization for ~60s+ per step". 2+ minute cold starts. Mitigation: switch to local provider.
- **Issue #15206** — "Hermes 0.11 still missing critical WhatsApp features": no read-receipt double-check, no chat reactions, no arbitrary file types via WhatsApp.

### 2.9 Specific feature requests (open issues, asked but not shipped)

| Issue | Ask |
|---|---|
| #16068 | Configurable auto-continue when max tool-call iterations reached |
| #16173 | Auto-use latest model for a configured provider (no manual pin) |
| #16282 | Support ACP harnesses beyond Copilot CLI (Cursor agent CLI etc.) |
| #16696 | Expose `limit` parameter and document query operators in `web_search` |
| #9673  | Resume interrupted tasks on restart |
| #6642  | Unified telemetry + analytics for latency, cost, completion/failure rates |
| #6741  | Structured session tracing with start/end timestamps |
| #14841 | Matrix voice replies sent as MP3 instead of OGG/Opus (mobile broken) |
| #14196 | Expose OpenAI TTS `instructions` field on `text_to_speech` tool |
| #15470 | Built-in suggestion window in the UI |
| #404   | Symphony-style autonomous issue-resolution workflow |
| #850   | Full Docker deployment (production-grade) |
| #15750 | Security disclosure email bouncing — meta but indicative |

### 2.10 What users *love* (validation — keep doing it)

- **Persistence** — *"Hermes feels different after 2-3 weeks in a way that other agents don't."*
- **Skills do produce results** — multiple users cite a 40% speed-up on repeated research tasks once skills accumulate.
- **Single gateway across messaging platforms** — users genuinely use Telegram + Discord + Slack interchangeably; the cross-platform continuity is a moat.
- **`MEMORY.md` / `USER.md` as plain-text user-owned files** — repeatedly cited as the right design.
- **Run-anywhere — VPS, GPU box, cloud sandbox** — *"$5 VPS"* line from the README is not marketing fluff; users actually use it.
- **Cost is reasonable for *light* use** — $2/mo for personal, $5/mo for active dev with cheap models.

These are the things Argo must not regress. Everything in §3.1 is a "must keep" parity item.

---

## 3. The three-way gap matrix

### 3.1 Hermes has it / Argo plans to match (parity table — keep watching these)

| Capability | Argo plan section | Status |
|---|---|---|
| Multi-provider support (≥ 16) | §8 manifests | Planned via 5 manifests covering 16+ providers |
| Persistence (sessions, memory) | §7 + §14 | Planned, structurally stronger |
| `MEMORY.md`/`USER.md`/`SOUL.md`/`AGENTS.md` | §14.1 | Planned 1:1 |
| Skills (agentskills.io-compat) | §14.2 | Planned 1:1 |
| MCP client + server | §12 | Planned, MCP-first not afterthought |
| Cron scheduler | §22 | Planned |
| Subagent delegation | §16 | Planned with budget tree (stronger than Hermes' iteration count) |
| Capability/approval flow | §10 | Planned, **structurally stronger** |
| Audit trail | §17 | Planned with hash chain (stronger) |
| OTel telemetry | §18 | Planned native (Hermes has it via plugin) |
| Six exec backends (local, Docker, SSH, Modal, Daytona, Singularity) | §15 v1 + §27 v1.1 | v1: local + Docker; v1.1: rest as plugins |
| 17 messaging platforms | §21 v1 + plugins | v1: Slack/Discord/Email; rest as plugins by launch (per §1.4 plan) |
| Self-improving skills loop | §14.2 (`refine`) | Planned with provenance + revertibility |
| Workspace = profile concept | §23 | Planned, **stronger** (content-addressable, fork/push/pull) |
| ACP server (VS Code/Zed/JetBrains) | not explicitly in plan | **GAP** — see §4 amendment |
| TUI | §20 | Planned (ratatui, not Ink) |
| Voice mode (TTS + STT) | not in plan | **GAP** — see §4 amendment |
| Image generation | not in plan | Plugin / MCP server, see §4 amendment |
| Honcho/Mem0/Supermemory plugin memory backends | §11 plugin protocol | Planned via plugin host |
| Hermes-Function-Calling format support | §8.5 (`xml_tool_call`) | Planned |
| Atropos RL training environments | excluded from v1 | Deliberately out of scope (§1.4) |

### 3.2 Hermes has it / Argo deliberately doesn't ship in v1 (with the plan's rationale)

Read these against §1.4 of the build plan ("What we deliberately don't build").

| Hermes capability | Argo's answer |
|---|---|
| 17 messaging platforms in-tree | 3 in-tree (Slack/Discord/Email); 14 as plugins available at launch |
| Voice synthesis in core | MCP server (`mcp-servers/tts/`) |
| Browser automation in core | MCP server (`mcp-servers/browser/`) — local Playwright by default, fixing #374 |
| Web dashboard | Out for v1.0; v1.5 milestone |
| Native iOS/Android | Out for v1.0; v1.5 milestone |
| Atropos RL training | Out of scope |
| 70+ tool modules | 8 in-core; rest as MCP servers |
| Voice mode native | MCP server with `whisper.cpp` (no ctranslate2 wheel issue → fixes Android) |

**Each "no" is a deliberate trade-off justified in the plan.** Some I should reconsider after this research — see §4.

### 3.3 Argo plans to ship that Hermes doesn't have (delta features)

These are net-new from Argo. Not all are equally validated by Hermes-user signal.

| Argo feature | User-evidence in Hermes corpus | Impact |
|---|---|---|
| **Manifest-driven providers** (TOML, not Python) | Strong (#4172 provider-detection bug class is structural in Hermes) | Eliminates a class of bug; lets non-Rust users add providers |
| **Capability-based security** with per-tool declared capabilities | Strong (Tirith hard-block UX complaints; #15750 disclosure bounce) | Replaces brittle pattern-matching with formal model |
| **Multi-tenant by construction** (tenant_id everywhere) | Strong (#527, #3897, #7517 all require this) | Solves three of the most-asked features in one architectural choice |
| **Token budget tree** (subagent-tree-aware caps) | Strong (#7481 fallback-not-inherited; cost complaints) | Caps absolute spend; mitigates §2.1 |
| **Plugin protocol = subprocess + JSON-RPC, not Python imports** | Strong (#5726 Honcho 60s startup block) | Plugin can't block boot; can't crash agent |
| **Native multi-OS Rust binary** | Strong (Python 3.13 incompat, WSL2-only, ctranslate2 wheels) | Eliminates an entire class of install pain |
| **Schema migrations with up + down** | Medium (no specific issue cited, but Hermes' "_reconcile_columns adds-only" is a known maintenance risk) | Future-proofs DB evolution |
| **Hash-chained audit log** | Medium (no Hermes issue, but enterprise compliance requirement) | Required for SOC2/HIPAA story |
| **Workspace bundles content-addressable + forkable** | Medium (Hermes profiles are dirs; sharing is ad-hoc) | Enables marketplace; team workspace sharing |

### 3.4 Hermes has it that Argo plan **misses** entirely — REAL GAPS

These are not deliberate omissions; they are oversights in the build plan that the user research surfaces.

| Missing in Argo plan | Hermes ships | User-evidence | Severity |
|---|---|---|---|
| ACP server (`acp_adapter/`) | Yes — exposes agent as ACP for VS Code/Zed/JetBrains | High; #16282 asks for *more* harnesses, implying high adoption | High |
| Voice mode (TTS + STT) | Yes (`tts_tool.py`, `voice_mode.py`, `transcription_tools.py`) | High (#14841, #14196, Termux limitation, #8098) | Medium |
| Web dashboard | Yes (`web/` Vite app) | High (community has built `hermes-control-interface`) | Medium (we deferred to v1.5; but 'something is needed') |
| Image generation | Yes (`image_generation_tool.py` + 3 backends) | Medium (less complaint volume but heavily used) | Medium |
| Honcho dialectic memory provider | Yes (built-in plugin) | Strong **negative** signal — buggy. Don't replicate same flaw. | Low (we ship as plugin anyway) |
| Activity heartbeats to prevent gateway timeouts | Yes (v0.11 #10501) | Medium | Medium |
| Mid-run agent steering (`/steer`) | Yes (v0.11 #12116) | High (this is a popular feature) | Medium |
| Shell-script lifecycle hooks | Yes (v0.11 #13296) | Medium | Low |

These are §4 amendments.

---

## 4. Amendments to the Argo build plan (line-item)

Each amendment is a specific change to `argo-build-plan.md`. I've numbered them; format is *Where → What → Why → Effort*.

### Amendment 1 — Cost-routing tier in the agent loop

**Where:** `argo-build-plan.md` §6 (the loop) and §13 (config)
**What:** Add a `RoutingPolicy` type in `argo-types`, declarable in config:

```toml
[routing]
policy = "tiered"            # "fixed" | "tiered" | "policy_plugin"

[routing.tiered]
classifier = "small"          # name of a small/cheap model used to classify
simple = "openai-compatible:gpt-4o-mini"
medium = "anthropic:claude-haiku-4-5"
complex = "anthropic:claude-opus-4-6"
classify_threshold_ms = 50    # max time for classifier; falls through to default if slow
```

**Why:** Direct fix for §2.1. NadirClaw exists because Hermes doesn't ship this. We can ship a built-in tiered router and outright eliminate the largest cost-pain category. This is a small, principled feature with outsized user impact.

**Effort:** Half-day spec, 3-5 days implement. Add Agent V (router) to §28.

### Amendment 2 — Self-evaluation by external judge

**Where:** §14.2 (Skills) and a new §14.3 ("Outcome adjudication")
**What:** Every "reflective" skill creation/refinement must be approved by an **independent verifier model** running with a structured rubric, not by the same model that did the work. Verifier sees:

- Original task statement
- Final agent output
- Tool calls executed
- Optional: ground-truth checks (file diffs, test runs)

Verifier returns one of `Success` / `PartialSuccess` / `Failure` with a structured rationale. Only `Success` triggers skill-write. `PartialSuccess` writes a "draft" skill marked unverified. `Failure` discards.

**Why:** Direct fix for §2.2. The largest unsolved problem in Hermes's reflective loop. Argo can be the first agent framework to ship this honestly.

**Effort:** 1 week. Add Agent W (adjudicator). Eval-suite tasks must extend with judge-disagreement scenarios.

### Amendment 3 — User-locked skills and memory entries

**Where:** §14
**What:** Add a `lock` field to skill frontmatter and memory entries:

```yaml
---
name: smart-home-rituals
version: "1.4.0"
lock: user-protected         # values: none | user-protected | content-locked
---
```

`user-protected`: agent CANNOT edit; only the user can via `argo skill edit --force`. `content-locked`: edits allowed but body hash must remain in lineage; agent must propose changes via PR-style diff.

**Why:** Direct fix for §2.3. Power users tune skills; auto-improvement nukes their work. This restores user control without disabling self-improvement globally.

**Effort:** 1 day. Capability check + frontmatter parse change.

### Amendment 4 — Gateway-level RBAC

**Where:** §10 (capability) + §21 (gateway)
**What:** Built-in role tiers with capability presets:

```toml
[gateway.rbac]
default_role = "guest"
roles.owner   = { capabilities = ["*"] }
roles.admin   = { capabilities = ["fs_read:**", "fs_write:**", "tool_dispatch:*", "spawn:*", "memory_write", "skill_write"] }
roles.user    = { capabilities = ["fs_read:./**", "tool_dispatch:web_*,memory,skill_view,skills_list"] }
roles.guest   = { capabilities = ["tool_dispatch:web_search,web_extract"] }

[gateway.rbac.bindings]
"slack:U01ABCD"  = "owner"
"slack:U02EFGH"  = "user"
"discord:#dev-help" = "user"
```

**Why:** Issues #527 and #3897. Community already built `hermes-control-interface` to fill this. Solves the biggest single barrier to enterprise/team deployment in one config change.

**Effort:** 2-3 days. The capability engine already supports this; bindings + role presets are the new bits.

### Amendment 5 — Native multi-agent per tenant

**Where:** §6 (loop), §13 (config), §21 (gateway)
**What:** Each tenant has one or more **named agents**, each with its own persona, memory namespace, default toolset, default model. The gateway routes by `(adapter, channel) → agent_name → tenant`. Single binary; multiple agents alive at once in the same process.

```toml
[agents.atlas]
persona_file = "personas/atlas.md"
default_model = "openai-compatible:gpt-4o"
default_toolset = "research"

[agents.scribe]
persona_file = "personas/scribe.md"
default_model = "anthropic:claude-haiku-4-5"
default_toolset = "writing"
```

**Why:** Issue #7517. Asked for explicitly. This is "1 binary, N named agents with N personas." Hermes' workaround is N profiles → N processes — wasteful.

**Effort:** 3-4 days. Agent identity becomes a routing key in the gateway and a context binding for the loop.

### Amendment 6 — Adaptive provider timeouts via heartbeats

**Where:** §8 (provider transport)
**What:** Replace the fixed reconnect timeout (Hermes' 180s) with a **heartbeat-aware** timeout. SSE streams: any data byte resets the timer. Codex Responses streaming: `event: ping` resets. For non-streaming providers: chunked HTTP keep-alive packets reset.

For providers that support it, request explicit `Keep-Alive`-style heartbeats. For providers that don't, use a 30s grace window from last byte before declaring dead.

**Why:** Bug #5889. Killing long-running provider work mid-call is bad UX and wastes money.

**Effort:** 1-2 days in `argo-provider`.

### Amendment 7 — Per-delegation fallback chain inheritance

**Where:** §16 (delegate)
**What:** Subagent inherits parent's `fallback_chain` by default. Configurable per-call:

```rust
delegate_task(
    prompt: "...",
    toolset: "code",
    inherit_fallback: true,          // default true
    fallback_override: None,
);
```

**Why:** Issue #7481. Currently `delegate_task` is a single point of failure when the parent chain succeeds elsewhere.

**Effort:** Half-day.

### Amendment 8 — `latest` model resolution

**Where:** §8 (manifest) + §13 (config)
**What:** Manifest declares a `[discovery]` block with an endpoint that returns the provider's current model list. `model = "latest"` resolves at startup (and refreshed every 24h) to the newest model matching configurable filters.

```toml
[provider.openai-compatible.discovery]
endpoint = "{base_url}/v1/models"
include_pattern = "^gpt-[0-9]"
exclude_pattern = "preview|legacy"
prefer = "highest_minor"
```

```toml
[provider.default]
model = "latest"
```

**Why:** Issue #16173. Manual pinning is friction; users miss new models for weeks.

**Effort:** 1 day.

### Amendment 9 — `auto_continue_at_max_iterations` policy

**Where:** §6 (loop) + §13 (config)
**What:** When `IterationBudget` hits zero, configurable policy:

- `halt` (default): emit a halted message, persist snapshot, return.
- `prompt_user`: surface a "continue with N more iterations?" approval to the user.
- `auto_continue`: silently grant another N iterations, up to a hard ceiling.

```toml
[budget]
on_max_iterations = "prompt_user"
auto_continue_max_extensions = 3
auto_continue_extension_size = 30
```

**Why:** Issue #16068.

**Effort:** Half-day.

### Amendment 10 — Local browser as default; cloud as opt-in

**Where:** `mcp-servers/browser/` (currently a one-line stub in the plan)
**What:** Spec the browser MCP server explicitly: pure-Rust + Playwright via headless Chromium **as the default**. Optional cloud backends (Browserbase, BrowserUse, etc.) configured per-server.

Auto-detect Chrome with `--remote-debugging-port=9222` and attach to it instead of launching a fresh headless. Honour user-level CDP endpoints.

**Why:** Issues #374, #6780, #493 — all asked for this for over a year in Hermes. Argo can ship it correctly on day 1.

**Effort:** 4-5 days. This is the biggest single MCP server.

### Amendment 11 — ACP server (Phase 4 add)

**Where:** `crates/argo-acp/` (new crate; see §3 plan repo layout — adds one entry)
**What:** Implement the Agent Client Protocol server so Argo is usable from VS Code, Zed, JetBrains, and any future ACP harness. Multi-harness support from day 1 (don't repeat Hermes' Copilot-only mistake — issue #16282).

**Why:** Hermes ships ACP and users use it (VS Code adoption is significant). Argo plan currently doesn't list it. It's a real Hermes feature; we should match.

**Effort:** 1 week. Add to Phase 4.

### Amendment 12 — Voice mode as MCP server, but spec'd properly

**Where:** `mcp-servers/voice/` (currently a one-line stub)
**What:** Spec the voice MCP server. Pure Rust pipeline:

- STT: `whisper.cpp` via Rust bindings (no ctranslate2; works on Android)
- TTS: `piper-rs` (default), with optional cloud backends (OpenAI TTS, ElevenLabs, MiniMax) configurable
- Audio encoding: `opus-rs` (fixes Matrix mobile playback bug #14841 by default)

**Why:** Hermes voice is broken on Android (ctranslate2 wheels), broken on Matrix mobile (#14841), and feature-incomplete (`instructions` field not exposed — #14196). Argo can ship it correctly.

**Effort:** 1-1.5 weeks for solid v1.

### Amendment 13 — Resume interrupted task on restart

**Where:** §6 (loop) + §17 (audit) + §22 (cron)
**What:** On daemon startup, list every session whose last record is `tool.call` without a paired `tool.result`. Treat as "interrupted." Surface a list to the user; default policy `prompt_user` (auto-resume with re-prompt, halt, or discard).

**Why:** Issue #9673. Hermes added partial support in v0.11; we should ship it solid from v1.0.

**Effort:** 2 days.

### Amendment 14 — Mid-run steering (`/steer`)

**Where:** §19 (CLI) + §6 (loop) + §11 (plugin protocol — `steer` is a plugin RPC)
**What:** A `/steer <text>` slash command (and equivalent gateway DM) injects a note that the running agent sees after its **next tool call**, without breaking the prompt cache. Implementation: enqueue into a `pending_steers` channel; the loop drains it after every tool result and prepends `<steer>` content to the next user message synthesizer.

**Why:** Hermes shipped this in v0.11 and users like it. Course-correcting an agent in-flight is genuinely useful.

**Effort:** 2-3 days.

---

## 5. Beyond-Hermes opportunities (ranked by user-evidence × impact)

These are net-new directions that go past Hermes parity. Ranked by user-evidence behind them.

### 5.1 ★★★★★ Agent Forge — automated skill optimization (DSPy + GEPA, in-tree)

Nous itself maintains a sibling repo `NousResearch/hermes-agent-self-evolution` that uses **DSPy + GEPA** (Genetic-Pareto Prompt Evolution; ICLR 2026 Oral) to automatically evolve and optimize Hermes skills, prompts, and tool descriptions. ~$2-10 per optimization run, no GPU required, works with as few as 3 examples by reading execution traces.

**They built it as a separate repo because retrofitting it into Hermes is hard.** Argo can ship it as a first-class subsystem from day 1.

Surface:
```
argo forge optimize --skill data-extraction --examples eval/data-extraction.jsonl
argo forge bench --skill data-extraction
argo forge promote --version 1.4.2
```

**Impact:** Argo becomes the first agent framework to ship continuous, principled skill improvement instead of "let the model edit itself and hope." Solves §2.2 *and* §2.3 simultaneously.

**Effort:** 2-3 weeks for v1. Make it a v1.0 differentiator, not a v1.5 add. Add Agent X (Forge).

### 5.2 ★★★★ Cost-aware routing as a plugin SDK feature

Beyond Amendment 1 (built-in tiered router), expose **policy plugins** so the community can ship sophisticated routers: cost-classifier model + per-task spend caps + provider-affinity rules + per-customer (multi-tenant) routing rules. NadirClaw exists because nobody else shipped this; in Argo it's a 200-LOC plugin.

**Impact:** The community fills this in within weeks of v1.0. Cost-conscious users have a one-flag fix to their largest pain point.

**Effort:** Built into Amendment 1's plugin point. Negligible additional.

### 5.3 ★★★★ Time-travel debugging

Replay any past session against a different model, a different toolset, or a modified workspace. The session DB already stores everything; expose it as:

```
argo session replay <session_id> --model anthropic:claude-opus-4-7 --tools different
argo session diff <id1> <id2>
argo session bisect <id> "find where the agent went wrong"
```

**Impact:** Solves the "why did the agent do that?" debugging crisis. No agent framework offers this today. Pairs naturally with Amendment 2 (external judge — replay through the judge to score what *would* have worked).

**Effort:** 1-2 weeks. Builds on existing session store.

### 5.4 ★★★★ Workspace marketplace via OCI artifacts

Workspace = `MEMORY.md` + `USER.md` + skills/ + cron/ + argo.toml. Already content-addressable in Argo. Push to any OCI registry (GHCR, Docker Hub, Artifact Registry):

```
argo workspace push ghcr.io/argo-agents/sales-bdr:latest
argo workspace pull ghcr.io/argo-agents/sales-bdr:latest
```

**Impact:** Workspace = container-image equivalent for agent identities. Sharing is `docker pull` not "clone my dotfiles." Enables a real marketplace and community library — the Skills Hub but at the **whole-agent** layer, not just procedural-memory layer.

**Effort:** 1-2 weeks. The bundle format already exists.

### 5.5 ★★★ Federated, opt-in skill-sharing learning

Users can opt to publish *anonymized successful* skill executions back to a public Argo learning network. The judge from Amendment 2 vetoes uncertain ones. The community's skill corpus improves continuously without compromising any individual user's data.

**Impact:** Compounding network effect. Hermes' Skills Hub is static (you install a skill or you don't); Argo's would learn.

**Effort:** Spec is 1 week; implementation 3-4 weeks (the network endpoint + privacy guarantees are nontrivial). Better as v1.5/v2.0.

### 5.6 ★★★ Native mobile (iOS + Android)

Two native apps with a thin client + remote daemon. Push notifications. Voice mode. Share-sheet integration. The agent is reachable from anywhere; the work runs on your gateway.

**Impact:** Hermes' Termux story is a hack; iOS has no story. Argo ships a real experience.

**Effort:** 6-8 weeks per platform. Not v1.0; v1.5 or v2.

### 5.7 ★★★ Built-in cost ceiling enforcement (hard $$$ caps)

Argo's budget tree already supports this. Make it ergonomic:

```toml
[budget]
default_spend_per_session_usd = 1.00
default_spend_per_day_usd = 10.00
default_spend_per_month_usd = 100.00
```

Block before spend, not after. Send a notification ("you have $X left this month") at 80% / 95%.

**Impact:** Solves the "$200 in 2 weeks debugging" complaint at the framework level.

**Effort:** 2 days. Built atop existing budget tree.

### 5.8 ★★★ Adaptive context compression

Today, compression triggers when context fills. Better: predictive compression based on running model's context window, current spend, and historical "this session is going long" signal. Compress *earlier*, save tokens *before* you need to.

**Impact:** Cuts §2.1 token bills further. Stacks with Amendment 1.

**Effort:** 1 week.

### 5.9 ★★ A "show your work" mode

Every assistant turn gets a structured "intent + plan + evidence" preamble. Optional, off by default. For users who want to *audit* what the agent is doing — security teams, accountants, lawyers. Hash-chained into audit log.

**Impact:** Enterprise compliance feature; no Hermes equivalent.

**Effort:** 1 week.

### 5.10 ★★ Decentralized / E2E-encrypted gateway

For privacy-sensitive teams: gateway-to-gateway federation with E2E encryption. No central Argo server. Like Matrix federation but for agents.

**Impact:** Privacy positioning; long-tail enterprise.

**Effort:** Out of v1; v2+.

---

## 6. The Argo plan delta — exact lines to change

### 6.1 Adds to `argo-build-plan.md` §1.2 (the five invariants)

Keep five. Adjust language on invariant 3 to read:

> 3. **Capability-based security with first-class RBAC roles.** Every tool, skill, plugin, and provider declares an explicit capability set. The core enforces capabilities at call time. Roles are a top-level configuration concept; gateway and CLI both consume them.

(Change recognises Amendment 4.)

### 6.2 Adds to §1.3 (NFRs)

Add three rows:

| Target | Number | Measurement |
|---|---|---|
| Idle agent token overhead (system + tools schema) | ≤ 4,000 tokens with 8 in-core tools | telemetry counter |
| Cost-routing tier classifier latency (when enabled) | ≤ 80 ms p95 | tracer span |
| Spend cap precision (predicted vs actual) | within 5% | NFR check |

### 6.3 Adds to §2 (Definition of Done)

- [ ] Built-in tiered cost router (Amendment 1) shipped and documented.
- [ ] External-judge skill verification (Amendment 2) shipped; eval suite includes judge-disagreement tasks.
- [ ] User-protected skill/memory locks (Amendment 3) implemented with `lock:` frontmatter.
- [ ] Gateway RBAC tiers (Amendment 4) shipped with sensible default presets.
- [ ] Multi-agent per tenant (Amendment 5) supported and documented.
- [ ] Adaptive provider timeouts (Amendment 6) replace any fixed-timeout reconnect logic.
- [ ] `delegate_task` inherits fallback chain by default (Amendment 7).
- [ ] `model = "latest"` resolution works against every v1 manifest (Amendment 8).
- [ ] `auto_continue_at_max_iterations` policy implemented with three modes (Amendment 9).
- [ ] Local-Playwright browser MCP server is the default (Amendment 10).
- [ ] ACP server crate ships with multi-harness support (Amendment 11).
- [ ] Voice MCP server ships pure-Rust whisper.cpp + piper-rs (Amendment 12).
- [ ] Resume-interrupted-task at startup with a policy switch (Amendment 13).
- [ ] `/steer` mid-run agent nudges supported in CLI, TUI, and gateway (Amendment 14).
- [ ] Argo Forge (DSPy/GEPA-style skill optimizer; §5.1) shipped as `argo forge` subcommand.
- [ ] Hard $$$ cost ceiling per session/day/month (§5.7) enforced.

### 6.4 New crate in §3 layout

Add: `crates/argo-acp/` (Amendment 11 — ACP server)

### 6.5 New phase additions in §27

Add Phase 4.5 ("ACP + Voice + Browser MCP server hardening") between Phase 4 and Phase 5. ~2 weeks.

Add Phase 7.5 ("Forge — automated skill optimization") between Phase 7 and v1.0 ship. ~2-3 weeks. Yes, this **delays v1.0 by ~2-3 weeks**; that's a deliberate trade-off to ship Argo Forge as a v1.0 differentiator rather than a v1.5 retrofit.

### 6.6 New agents in §28 dispatch plan

- **Agent V** — Routing and cost-tier classifier (Amendment 1)
- **Agent W** — External-judge adjudicator (Amendment 2)
- **Agent X** — Argo Forge (DSPy/GEPA optimizer)
- **Agent Y** — ACP server (Amendment 11)
- **Agent Z** — Voice MCP server (Amendment 12)
- **Agent AA** — Browser MCP server (Amendment 10)
- **Agent BB** — Time-travel debug (§5.3)
- **Agent CC** — Workspace marketplace (§5.4)

### 6.7 Updated risk register

Add four risks:

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Cost-router classifier wrong on edge cases | Medium | Medium | Per-task overrides in config; `route_hint` argument on tools |
| External judge disagrees too often, blocking skill creation | Medium | Medium | Three-state outcome (Success/Partial/Failure) with `partial → draft skill` path |
| Argo Forge runs become expensive without supervision | Medium | High | Default off; explicit opt-in; budget caps; sample-only mode |
| Multi-agent confusion (which agent answered) | Low | Medium | Always tag agent name in gateway responses; per-agent persona styling |

---

## 7. What I'd do tomorrow if it were my project

Decide today on these in priority order:

1. **Approve the 14 amendments** (or push back on specific ones). They're all small individually; together they shift Argo from "Hermes-shaped, smaller" to "Hermes-superset, smaller."
2. **Decide on Argo Forge for v1.0** (§5.1). This is the single biggest "beyond Hermes" lever. Adds 2-3 weeks; pays for itself if it lets you market Argo as the first framework to ship principled skill optimization. **Recommendation: yes, ship in v1.0.**
3. **Run the user survey** (10 questions, Hermes Discord, target N=100). One week to design + run + analyze. Re-baseline this gap analysis with quantitative weights. **Recommendation: yes, before final scope lock.**
4. **Cut scope ruthlessly elsewhere** to make room. Candidates:
   - Drop SSH + Singularity sandboxes from v1.0 entirely (push to v1.1 plugin pack).
   - Drop Email gateway adapter from in-tree v1.0 (community plugin within first month).
   - Drop the Go plugin SDK from v1.0 (Python + TS only; Go in v1.1).

Net result of doing 1-3: **same v1.0 ship date, with the 14 amendments and Argo Forge**, by trading off three lower-value items.

---

## 8. Sources cited

GitHub issues (NousResearch/hermes-agent unless noted):

- [#374](https://github.com/NousResearch/hermes-agent/issues/374) Local browser via Playwright/CDP
- [#404](https://github.com/NousResearch/hermes-agent/issues/404) Symphony-style autonomous resolution
- [#493](https://github.com/NousResearch/hermes-agent/issues/493) Pinchtab local browser HTTP API
- [#527](https://github.com/NousResearch/hermes-agent/issues/527) Gateway RBAC tiers
- [#850](https://github.com/NousResearch/hermes-agent/issues/850) Full Docker deployment
- [#3897](https://github.com/NousResearch/hermes-agent/issues/3897) Per-user tool restrictions
- [#4172](https://github.com/NousResearch/hermes-agent/issues/4172) Provider auto-detection bug
- [#4379](https://github.com/NousResearch/hermes-agent/issues/4379) Token overhead 73%/13.9k
- [#5726](https://github.com/NousResearch/hermes-agent/issues/5726) Honcho 60s+ startup block
- [#5889](https://github.com/NousResearch/hermes-agent/issues/5889) 180s reconnect bug
- [#6642](https://github.com/NousResearch/hermes-agent/issues/6642) Unified telemetry/analytics
- [#6741](https://github.com/NousResearch/hermes-agent/issues/6741) Structured session tracing
- [#6780](https://github.com/NousResearch/hermes-agent/issues/6780) Auto-detect Chrome CDP 9222
- [#7481](https://github.com/NousResearch/hermes-agent/issues/7481) Per-delegation fallback chain
- [#7517](https://github.com/NousResearch/hermes-agent/issues/7517) Native multi-agent
- [#8098](https://github.com/NousResearch/hermes-agent/issues/8098) Voice mode silent first-run
- [#9673](https://github.com/NousResearch/hermes-agent/issues/9673) Resume interrupted on restart
- [#14196](https://github.com/NousResearch/hermes-agent/issues/14196) Expose OpenAI TTS instructions
- [#14841](https://github.com/NousResearch/hermes-agent/issues/14841) Matrix MP3-vs-OGG bug
- [#15206](https://github.com/NousResearch/hermes-agent/issues/15206) WhatsApp missing features
- [#15470](https://github.com/NousResearch/hermes-agent/issues/15470) Suggestion list for Hermes team
- [#15750](https://github.com/NousResearch/hermes-agent/issues/15750) Security disclosure email bouncing
- [#15914](https://github.com/NousResearch/hermes-agent/issues/15914) Fallback chain collapse (P1)
- [#16068](https://github.com/NousResearch/hermes-agent/issues/16068) Auto-continue at max iterations
- [#16173](https://github.com/NousResearch/hermes-agent/issues/16173) Auto-use latest model
- [#16282](https://github.com/NousResearch/hermes-agent/issues/16282) ACP harnesses beyond Copilot
- [#16696](https://github.com/NousResearch/hermes-agent/issues/16696) web_search limit/operators

Other primary sources:

- [NousResearch/hermes-agent-self-evolution](https://github.com/NousResearch/hermes-agent-self-evolution) — DSPy + GEPA self-improvement (ICLR 2026 Oral)
- [xaspx/hermes-control-interface](https://github.com/xaspx/hermes-control-interface) — community-built dashboard with multi-user auth + RBAC

Reviews / commentary:

- [Hermes Agent Token Overhead: The Hidden Cost](https://hermes-agent.ai/blog/hermes-agent-token-overhead)
- [How to Run Hermes Agent at 60% Lower Cost (DEV.to)](https://dev.to/dor_amir_dbb52baafff7ca5b/how-to-run-hermes-agent-at-60-lower-cost-without-touching-its-code-19ck)
- [How to Configure OpenRouter with Hermes Without Burning Credits (BSWEN)](https://docs.bswen.com/blog/2026-04-19-hermes-openrouter-cost-optimization/)
- [I Switched from OpenClaw to Hermes — Sathish Raju (Medium)](https://medium.com/@sathishkraju/i-switched-from-openclaw-to-hermes-agent-heres-what-nobody-told-me-5f33a746b6ca)
- [OpenClaw vs Hermes: What Reddit Says (Kilo)](https://kilo.ai/articles/openclaw-vs-hermes-what-reddit-says)
- [Six weeks running Hermes — Harlem World](https://www.harlemworldmagazine.com/ive-been-running-hermes-agent-for-six-weeks-what-actually-surprised-me/)
- [Hermes Agent Troubleshooting (xugj520)](https://www.xugj520.cn/en/archives/hermes-agent-troubleshooting-errors-guide.html)
- [Observability and Debugging — Claude Code vs Hermes (kenhuangus Substack)](https://kenhuangus.substack.com/p/chapter-9-observability-and-debugging)
- [State of Hermes Agent — April 2026 (Hermes Atlas)](https://hermesatlas.com/reports/state-of-hermes-april-2026)

Hermes documentation referenced for parity claims:

- [Hermes Agent docs](https://hermes-agent.nousresearch.com/docs/)
- [v0.11 Release notes](https://github.com/NousResearch/hermes-agent/blob/main/RELEASE_v0.11.0.md)

---

*Plan amendments are non-binding until approved. Once approved, the line-items in §6 are the spec deltas; §28 of the build plan adds Agents V through CC.*
