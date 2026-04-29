# Hermes Agent — Comprehensive Technical Deep-Dive

**Subject:** `NousResearch/hermes-agent` (and the sibling `Hermes-Function-Calling`)
**Repo state:** main @ `5e68503` — most recent release tag `v0.11.0` (v2026.4.23)
**Research date:** April 29, 2026
**Author:** Generated for Mark Hallam (Acmeco)

---

## 0. Reading guide and scope

The user's request was "review the entire code base line by line." The hermes-agent repository contains **1,248 Python files totalling ~599,000 lines of Python**, plus 272 TypeScript files for the new Ink TUI, 839 Markdown files (skills, docs, release notes), Nix flakes, packaging scripts, and a Docusaurus site. Two single Python files alone — `cli.py` (~517 KB) and `run_agent.py` (~700 KB, ~13,500 LOC) — are larger than most entire agent frameworks.

A literal line-by-line transcription would be neither useful nor truthful (most of the LOC is provider-conversion plumbing, retry/sanitization edge cases, and prompt-format heuristics for ~25 different model families). What follows is the maximally useful equivalent: **every directory documented, every shipped feature traced to its implementation, every load-bearing class anatomized.** Where the code path is genuinely novel (the agent loop, the iteration budget, the cross-agent file coordination, the OPD RL environment, the transport ABC) the prose drops into the source.

The companion `NousResearch/Hermes-Function-Calling` repo (1,034 LOC across 7 files) **is** reviewed line-by-line in §17 because it is small enough to do so honestly. Hermes-agent is treated structurally.

The report is structured as:

- §1 What Hermes Agent is, in one screen
- §2 First principles — the design philosophy that produces the structure
- §3 Repository topology and file dependency graph
- §4 The conversation loop (`run_agent.py` → `AIAgent.run_conversation`)
- §5 Tools and the registry
- §6 Toolsets — packaging tools for context economy
- §7 Provider/transport layer — how Hermes talks to ~16 LLM providers
- §8 Persistent memory, sessions, and the SQLite/FTS5 store
- §9 Skills system — procedural memory the agent writes for itself
- §10 Plugins — hot-loadable extension points
- §11 Messaging gateway — 17 platforms behind one agent
- §12 Execution environments — local / Docker / SSH / Modal / Daytona / Singularity
- §13 Cron and scheduled automations
- §14 Subagent delegation, parallelism, iteration budgets
- §15 Trajectory compression and OPD RL training
- §16 The Ink TUI (`ui-tui/`) and the JSON-RPC bridge (`tui_gateway/`)
- §17 The sibling `Hermes-Function-Calling` repo — line-by-line
- §18 Security model (approval, sandboxing, credential pool, redaction)
- §19 Test surface
- §20 Release cadence and what changed in v0.11.0
- §21 Design analysis — what's strong, what's load-bearing, what's risky
- §22 Sources

---

## 1. What Hermes Agent is — one screen

Hermes Agent is Nous Research's open-source AI agent framework — a self-improving, model-agnostic conversational agent that runs anywhere from a $5 VPS to a serverless GPU sandbox, and exposes itself as a CLI, a TUI (terminal UI), an ACP server (for VS Code / Zed / JetBrains), an MCP server (for Claude Code / Cursor / Codex), an HTTP API, a web dashboard, and adapters for **17 messaging platforms** — Telegram, Discord, Slack, WhatsApp, Signal, Email, SMS, Matrix, Mattermost, Home Assistant, Feishu, DingTalk, WeCom, Weixin, QQBot, BlueBubbles (iMessage), and a generic webhook adapter.

Three things make it structurally different from "agent frameworks" you may know (LangChain, AutoGen, CrewAI):

1. **The agent persists itself.** A SQLite database with FTS5 indexing keeps every session, every tool result, and every memory across runs. The agent searches its own history mid-conversation. `MEMORY.md` and `USER.md` are agent-curated and agent-edited.
2. **The agent writes its own skills.** "Skills" are Markdown files with YAML frontmatter that act as procedural memory — the agent creates them after complex tasks, refines them on use, and the Skills Hub (an `agentskills.io`-compatible standard) lets community-shared skills be installed by name.
3. **The agent is a daemon, not a library call.** A single gateway process can simultaneously chat on Telegram, run a cron job into Slack, accept commands from VS Code over ACP, and stream tool output to the TUI — sharing the same `AIAgent` class, session DB, and memory.

Quick install: `curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh | bash`. License is MIT.

---

## 2. First-principles design

Five design choices generate most of what you see in the codebase. Reading the code in their light makes the size tractable.

### 2.1 Provider-agnosticism is non-negotiable
Hermes refuses to bind to a single LLM. The codebase supports OpenAI, Anthropic, AWS Bedrock, Nous Portal, OpenRouter (200+ models), NVIDIA NIM, Vercel ai-gateway, Google Gemini (CLI OAuth + native AI Studio), Xiaomi MiMo, z.ai/GLM, Kimi/Moonshot, MiniMax, HuggingFace, Ollama, LM Studio, Lemonade, xAI, DeepSeek, Step Plan, Arcee AI, and "your own endpoint." Each is a real provider with real format quirks. **All of `agent/anthropic_adapter.py` (76 KB), `agent/bedrock_adapter.py` (49 KB), `agent/codex_responses_adapter.py` (44 KB), `agent/gemini_*.py` (~70 KB combined), and `agent/auxiliary_client.py` (160 KB!) exists because format conversion is hard.** The v0.11 transport ABC (`agent/transports/`) was extracted so this conversion is no longer inlined into the agent loop.

### 2.2 The agent must keep going
Every layer has retry, fallback, and degradation. `IterationBudget` caps tool-calling iterations (default 90 for parent, 50 per subagent). `_invalid_tool_retries`, `_invalid_json_retries`, `_empty_content_retries`, `_incomplete_scratchpad_retries`, `_codex_incomplete_retries`, `_thinking_prefill_retries`, `_post_tool_empty_retried` are all instance counters in `AIAgent` for distinct failure modes (run_agent.py:10079-10089). Compression has its own fallback chain (auxiliary model → main model on 503/404). Connection-level dead-socket cleanup runs **before each turn** (run_agent.py:10094).

### 2.3 Privacy and personality must travel
`SOUL.md` (agent persona), `MEMORY.md` (facts the agent has learned), `USER.md` (the agent's evolving model of you), and `AGENTS.md` (workspace instructions) are all **plain Markdown the user owns**. Honcho, Mem0, Supermemory, Hindsight, Holographic, Byterover are pluggable backing stores (`plugins/memory/`), but the source of truth is human-readable text on disk. Migration from "OpenClaw" (the prior Nous agent product) is a first-class command: `hermes claw migrate`.

### 2.4 Tool calls are sacred and parallel-where-safe
The model's tool-calling protocol is the integration point. Hermes will:
- **Repair malformed JSON arguments** (`_repair_tool_call_arguments` at run_agent.py:590)
- **Strip non-ASCII** for providers that choke on it (run_agent.py:687-805)
- **Re-parse `<tool_call>` XML** when models emit it inside content (run_agent.py:2921-2989)
- **Detect destructive shell commands** before allowing parallel execution (`_DESTRUCTIVE_PATTERNS` at run_agent.py:330)
- **Path-scope file tools** so concurrent `read_file`/`write_file` on independent paths can parallelize but overlapping paths fall back to sequential (`_PATH_SCOPED_TOOLS`, `_paths_overlap`)

### 2.5 Same agent, every surface
The CLI, TUI, ACP server, MCP server, gateway, and dashboard are all *consumers* of the same `AIAgent` class. The conversation loop has no UI dependencies — it accepts callbacks (`tool_progress_callback`, `tool_start_callback`, `tool_complete_callback`, `thinking_callback`, `reasoning_callback`, `clarify_callback`, `step_callback`, `stream_delta_callback`, `interim_assistant_callback`, `tool_gen_callback`, `status_callback`) that each surface wires to its own renderer.

---

## 3. Repository topology

### 3.1 Top-level layout (annotated)

```
hermes-agent/
├── run_agent.py             ~13,500 LOC  AIAgent class, the conversation loop
├── cli.py                   ~11,000 LOC  HermesCLI class, interactive CLI orchestrator
├── model_tools.py            ~750 LOC   Tool dispatch entry point (handle_function_call)
├── toolsets.py              ~600 LOC   Toolset definitions (~20 named bundles)
├── toolset_distributions.py             How tools are chosen for each platform/profile
├── batch_runner.py          ~1,500 LOC  Parallel batch trajectory generation
├── trajectory_compressor.py ~2,000 LOC  Post-processing for RL training data
├── mcp_serve.py             ~750 LOC   Hermes-as-MCP-server (10 tools)
├── mini_swe_runner.py        ~700 LOC   SWE-bench harness wrapper
├── rl_cli.py                            CLI for Atropos RL training runs
├── hermes_state.py          ~2,500 LOC  SessionDB — SQLite + FTS5 store
├── hermes_constants.py                  get_hermes_home(), profile-aware paths
├── hermes_logging.py                    setup_logging(), log levels per file
├── hermes_time.py                       timezone-aware helpers
├── utils.py
├── agent/                   45 files    Provider adapters, memory, compression
│   └── transports/          7 files     Pluggable transport ABC (v0.11)
├── tools/                   ~70 files   Tool implementations (auto-discovered)
│   └── environments/        11 files    Terminal backends
├── environments/            (top-level) RL training (Atropos) environments
│   └── tool_call_parsers/   11 files    Per-model-family tool-call parsers
├── gateway/                 ~20 files   Messaging gateway runtime
│   └── platforms/           17 files    One adapter per messaging platform
├── hermes_cli/              ~50 files   CLI subcommands, setup wizard
├── plugins/                              Hot-loadable extensions
├── skills/                  25 dirs     Built-in agent skills (Markdown)
├── optional-skills/         16 dirs     Heavier skills (not active by default)
├── ui-tui/                              Ink/React terminal UI
├── tui_gateway/             7 files     Python JSON-RPC backend for TUI
├── acp_adapter/             9 files     ACP (Agent Client Protocol) server
├── cron/                    3 files     Scheduler for cronjob-style automation
├── packaging/, nix/, docker/, scripts/  Distribution & DX
├── web/, website/                       Dashboard and Docusaurus docs
└── tests/                   ~700 files  ~15k pytest tests
```

### 3.2 Import dependency graph

```
                      tools/registry.py  (no deps)
                              ▲
                  tools/*.py  (each calls registry.register() at import time)
                              ▲
                       model_tools.py  (imports registry + triggers discover_builtin_tools())
                              ▲
            ┌─────────────────┼─────────────────┐
   run_agent.py            cli.py        environments/agent_loop.py
            ▲                 ▲                  ▲
            │                 │                  │
   gateway/run.py     hermes_cli/main.py    rl_cli.py / batch_runner.py
            ▲
   acp_adapter/server.py   mcp_serve.py
```

The registry is **structural** — every other layer imports through it. `tools/registry.py:_module_registers_tools` does an AST scan to find modules that contain a top-level `registry.register(...)` call before importing them, so helper modules that happen to call `registry.register()` from inside a function aren't accidentally elevated to "tool" status.

### 3.3 User config and runtime layout

- `~/.hermes/config.yaml` — settings (model, toolsets, profiles, providers, gateway, cron, plugins, themes, memory)
- `~/.hermes/.env` — secrets only (API keys, tokens, passwords)
- `~/.hermes/state.db` — SQLite DB: `sessions`, `messages` (FTS5-indexed), `compression_tips`, lineage links, titles
- `~/.hermes/logs/` — `agent.log` (INFO+), `errors.log` (WARNING+), `gateway.log`
- `~/.hermes/skills/` — user/installed skills (Skills Hub destination)
- `~/.hermes/plugins/` — user plugins
- `~/.hermes/cron/` — cron jobs and `.tick.lock`
- `MEMORY.md`, `USER.md`, `SOUL.md`, `AGENTS.md` — workspace-rooted persistent state

The `get_hermes_home()` indirection in `hermes_constants.py` is what makes "profiles" possible: setting `HERMES_HOME=~/.hermes/work` or `HERMES_PROFILE=work` redirects all of the above to a sibling tree, so a single user can run multiple isolated Hermes instances side-by-side.

---

## 4. The conversation loop — `AIAgent.run_conversation`

The heart of the system. `run_agent.py` is a single 13.5k-LOC file built around one class, `AIAgent`, with a single public entry point, `run_conversation(user_message, system_message=None, conversation_history=None, task_id=None, stream_callback=None, persist_user_message=None) -> Dict[str, Any]`.

### 4.1 `AIAgent.__init__` — the configuration surface

The constructor takes **~60 keyword arguments** (run_agent.py:875-1040). Reading them is the fastest way to understand what an agent *is* in this codebase:

- **Provider/transport**: `base_url`, `api_key`, `provider`, `api_mode` ("chat_completions" | "anthropic_messages" | "codex_responses" | "bedrock"), `model`, `acp_command`/`acp_args`, `command`/`args`
- **Iteration economy**: `max_iterations=90`, `tool_delay=1.0`, `iteration_budget` (shared with subagents)
- **Toolset selection**: `enabled_toolsets`, `disabled_toolsets`
- **Trajectory capture**: `save_trajectories`, `verbose_logging`, `quiet_mode`, `log_prefix`, `log_prefix_chars`
- **OpenRouter routing**: `providers_allowed`, `providers_ignored`, `providers_order`, `provider_sort`, `provider_require_parameters`, `provider_data_collection`
- **Session bookkeeping**: `session_id`, `session_db`, `parent_session_id`, `pass_session_id`, `gateway_session_key`
- **Callbacks** (10+): `tool_progress_callback`, `tool_start_callback`, `tool_complete_callback`, `thinking_callback`, `reasoning_callback`, `clarify_callback`, `step_callback`, `stream_delta_callback`, `interim_assistant_callback`, `tool_gen_callback`, `status_callback`
- **Generation knobs**: `max_tokens`, `reasoning_config`, `service_tier`, `request_overrides`, `prefill_messages`
- **Identity** (so the agent knows where it is): `platform`, `user_id`, `user_name`, `chat_id`, `chat_name`, `chat_type`, `thread_id`
- **Context economy**: `skip_context_files`, `skip_memory`
- **Reliability**: `fallback_model`, `credential_pool`, `checkpoints_enabled`, `checkpoint_max_snapshots`

Every callback path is real code: the gateway uses `tool_progress_callback` to debounce streaming "🔧 Running tool…" messages so Telegram doesn't get rate-limited, while the CLI uses it to drive a Rich spinner.

### 4.2 `run_conversation` — turn-level pre-flight

The first ~150 lines of `run_conversation` (run_agent.py:10009-10160) do an aggressive pre-flight:

1. `_install_safe_stdio()` wraps stdout/stderr in a `_SafeWriter` (defined at run_agent.py:166) that swallows `OSError` from broken pipes — important because Hermes runs as a daemon under systemd, in Modal sandboxes, in Docker containers, and from cron, where stdout can vanish.
2. `set_session_context(self.session_id)` injects the session ID into a `contextvars.ContextVar` that the logging filter pulls into every log record. `hermes logs --session <id>` works because of this.
3. `_restore_primary_runtime()` — if the previous turn fell back to a secondary model, restore the primary one for this turn.
4. Surrogate sanitization on `user_message` and `persist_user_message` (rich-text paste from Google Docs / Word leaks lone surrogates that crash the OpenAI SDK during JSON encoding).
5. Reset all retry counters (eight of them).
6. **Pre-turn dead-connection cleanup** (`_cleanup_dead_connections`) — detects and closes zombie sockets left over from provider outages or dropped streams. This is the kind of detail that comes only from running an agent in production.

### 4.3 The loop body — what actually happens

After pre-flight, the loop is conceptually:

```python
while True:
    if not iteration_budget.consume():        # 90 by default
        break
    response = transport.complete(...)        # API call w/ retry, fallback, prompt cache
    transport.normalize(response)             # to a canonical message
    if not response.tool_calls:               # final answer
        break
    if _should_parallelize_tool_batch(tool_calls):
        results = _execute_tool_calls_concurrent(tool_calls)   # ThreadPoolExecutor, max 8
    else:
        results = [handle_function_call(tc) for tc in tool_calls]
    messages += [assistant_msg, *tool_msgs]
    session_db.append_message(...)            # SQLite + FTS5 indexing
```

The real loop is enormously fatter because of:

- **Tool-call repair** — if the model emits `<tool_call>{...}</tool_call>` inside content (which Hermes-format models, Llama, Qwen, GLM, DeepSeek, Kimi, Mistral, and LongCat all do, each slightly differently), the appropriate parser from `environments/tool_call_parsers/` runs and synthesizes a structured `tool_calls` array.
- **Argument coercion** — `coerce_tool_args` (model_tools.py:453) walks the schema and converts string `"123"` to `int` for parameters typed `integer`, etc.
- **Empty-content nudges** — weak models sometimes return empty content with no tool calls after substantive tool execution; Hermes injects a "please continue" nudge with backoff (and gives up after `_post_tool_empty_retried`).
- **Prompt-caching guards** — `agent/prompt_caching.py` ensures the cache-control breakpoints stay stable across turns; mutating system prompt text mid-conversation would silently break Anthropic's prompt cache.
- **Reasoning capture** — for models that emit `reasoning_content` (Anthropic extended thinking, Kimi, GPT-5.x, Gemini), it's split off, surfaced to the UI via `reasoning_callback`, and conditionally stripped from history.
- **Fallback model handling** — on permanent 503/404 from the primary, the configured fallback runs; the next turn restores the primary (`_restore_primary_runtime`).
- **Interrupt handling** — `tools/interrupt.py` exposes a check that the inner `_interruptible_api_call` polls; `Ctrl+C` and platform `/stop` both flow through this.

### 4.4 `IterationBudget` — the central traffic cop

```python
class IterationBudget:
    """Thread-safe iteration counter. Each agent (parent or subagent) gets its own."""
    def __init__(self, max_total: int):
        self.max_total = max_total
        self._used = 0
        self._lock = threading.Lock()
    def consume(self) -> bool: ...
    def refund(self) -> None: ...      # for execute_code turns
    @property def used / remaining
```

The doctrine: `execute_code` (programmatic tool calling — write a Python script, agent runs the tools via RPC, returns) refunds its iteration so it doesn't eat the budget. Every other tool-call iteration consumes one. Subagents get **independent** budgets, so the *total* iterations across a delegation tree can exceed the parent's cap (deliberate; hierarchical work is supposed to compound).

### 4.5 Parallel-vs-sequential tool execution

Three frozensets at the top of `run_agent.py` (lines 313-330) define the policy:

```python
_NEVER_PARALLEL_TOOLS = frozenset({"clarify"})    # interactive
_PARALLEL_SAFE_TOOLS  = frozenset({"ha_get_state", "ha_list_entities", "ha_list_services",
                                   "read_file", "search_files", "session_search",
                                   "skill_view", "skills_list", "vision_analyze",
                                   "web_extract", "web_search"})
_PATH_SCOPED_TOOLS    = frozenset({"read_file", "write_file", "patch"})
_MAX_TOOL_WORKERS     = 8
```

For path-scoped tools, `_extract_parallel_scope_path` returns the file path each call touches and `_paths_overlap` decides if two calls in the batch could collide. Disjoint paths → parallel. Overlapping → sequential.

Plus a regex-based destructive-command detector for `terminal` (run_agent.py:330):

```python
_DESTRUCTIVE_PATTERNS = re.compile(r"""
    (?:^|\s|&&|\|\||;|`)(?:rm\s|rmdir\s|cp\s|install\s|mv\s|sed\s+-i|
                          truncate\s|dd\s|shred\s|git\s+(?:reset|clean|checkout)\s)
""", re.VERBOSE)
_REDIRECT_OVERWRITE = re.compile(r'[^>]>[^>]|^>[^>]')
```

A batch with even one destructive command falls back to sequential.

---

## 5. Tools — the registry pattern

### 5.1 `tools/registry.py`

The registry is small but load-bearing. `register()` records `(schema, handler, toolset, check_fn)`; `discover_builtin_tools()` AST-scans `tools/*.py` for top-level `registry.register(...)` calls and imports the qualifying modules — circular-import-safe by construction. `ToolEntry` uses `__slots__` for memory.

### 5.2 The shipped tool catalog (~70 modules)

The 40+ shipped tools, grouped:

| Category | Module(s) | Notes |
|---|---|---|
| **Web** | `web_tools.py` (89 KB) | `web_search` (multi-provider), `web_extract` |
| **Terminal & process** | `terminal_tool.py` (94 KB), `process_registry.py` (57 KB) | 6 backends, background tasks, lifecycle |
| **Files** | `file_tools.py` (52 KB), `file_operations.py` (51 KB), `file_state.py` (12 KB), `patch_parser.py` (22 KB) | atomic writes, fuzzy matching, unified-diff patches |
| **Browser** | `browser_tool.py` (121 KB!), `browser_supervisor.py` (56 KB), `browser_camofox*`, `browser_cdp_tool.py`, `browser_dialog_tool.py`, `browser_providers/` | navigate/click/type/scroll/console/CDP/dialog/get_images/vision |
| **Vision** | `vision_tools.py` (31 KB) | image analysis routed via providers |
| **Image gen** | `image_generation_tool.py` (38 KB) + `plugins/image_gen/{openai,openai-codex,xai}` | pluggable backends |
| **TTS / voice** | `tts_tool.py` (58 KB), `voice_mode.py` (39 KB), `transcription_tools.py` (36 KB), `neutts_synth.py` | NeuTTS bundled |
| **Skills** | `skills_tool.py` (55 KB), `skills_hub.py` (119 KB!), `skill_manager_tool.py` (31 KB), `skill_usage.py`, `skills_guard.py`, `skills_sync.py` | full skills pipeline |
| **Memory** | `memory_tool.py` (23 KB) + `plugins/memory/{honcho,mem0,supermemory,hindsight,holographic,byterover}` | one tool, many backends |
| **Planning** | `todo_tool.py`, `clarify_tool.py` | structured task lists, ask-the-user |
| **Code execution** | `code_execution_tool.py` (62 KB) | sandboxed Python with imports + tool RPC |
| **Delegation** | `delegate_tool.py` (107 KB!) | spawn isolated subagents |
| **Mixture-of-agents** | `mixture_of_agents_tool.py` (22 KB) | multi-model consensus |
| **Cronjobs** | `cronjob_tools.py` (25 KB) | create/list/update/pause/resume/remove/trigger |
| **Messaging** | `send_message_tool.py` (68 KB), `discord_tool.py` (34 KB) | gated on gateway running |
| **Smart home** | `homeassistant_tool.py` (18 KB) | gated on `HASS_TOKEN` |
| **MCP integration** | `mcp_tool.py` (127 KB!), `mcp_oauth.py` (22 KB), `mcp_oauth_manager.py` (24 KB), `managed_tool_gateway.py` | Hermes is both an MCP client and an MCP server |
| **Search** | `session_search_tool.py` (24 KB) | FTS5 over the agent's own history |
| **RL training** | `rl_training_tool.py` (57 KB) | spawn Atropos training runs from the agent |
| **Specialized SaaS** | `feishu_doc_tool.py`, `feishu_drive_tool.py`, `yuanbao_tools.py` | non-Western platform integration |
| **Plumbing** | `approval.py` (53 KB), `checkpoint_manager.py` (32 KB), `tirith_security.py` (26 KB), `url_safety.py`, `path_security.py`, `osv_check.py`, `schema_sanitizer.py`, `tool_output_limits.py`, `tool_result_storage.py`, `interrupt.py` | the safety/quality layer |

That `browser_tool.py` is 121 KB and `mcp_tool.py` is 127 KB tells you where the integration complexity lives.

### 5.3 `model_tools.handle_function_call` — the dispatcher

```python
def handle_function_call(function_name, function_args, task_id, user_task) -> str
```

The single point where every tool call lands. It:

1. Looks up `function_name` in the registry.
2. Coerces `function_args` against the schema (`coerce_tool_args`).
3. Runs the optional `check_fn` (gating: `send_message` only if the gateway is up, `homeassistant` only if `HASS_TOKEN` is set).
4. Invokes the handler, sync or async (`_get_tool_loop()` provides a pinned event loop so sync tools can `asyncio.run()` internally without deadlocking inside another loop).
5. Applies `tool_output_limits` (truncate massive outputs, with pointers).
6. Optionally persists large outputs via `tool_result_storage` (`maybe_persist_tool_result`) and replaces the inline content with a reference — saves context.
7. Calls every registered plugin hook (`pre_tool_call` can veto; `post_tool_call`; `transform_tool_result` can rewrite; `transform_terminal_output` for shell streams).
8. Returns a string (formatted for the model).

---

## 6. Toolsets — `toolsets.py`

A toolset is a named bundle of tools. Why bundles? Because every tool in the active set spends prompt tokens on its schema. Loading all 40+ tools costs ~12,000 tokens of schema; a focused agent should load only what it needs.

`_HERMES_CORE_TOOLS` (toolsets.py:31-66) is the default list — web, terminal, files, vision, image, skills, browser, TTS, todo, memory, session_search, clarify, execute_code, delegate_task, cronjob, send_message, ha_*. About 35 tools.

`TOOLSETS` is a dict mapping name → `{description, tools, includes}`. Twenty-something entries: `web`, `search`, `vision`, `image_gen`, `terminal`, `moa`, `skills`, `browser`, `cronjob`, `messaging`, plus profile-specific bundles like `cli`, `gateway`, `dashboard`, `subagent_default`, `subagent_research`, `subagent_swe`. The `includes` mechanism lets toolsets compose other toolsets without duplicating tool lists.

Resolution: `resolve_toolset(name) -> List[str]` walks `includes` recursively (with cycle detection via the `visited` set), and `resolve_multiple_toolsets` unions them. Plugins can add named toolsets at runtime via `_get_plugin_toolset_names()` and `_get_registry_toolset_aliases`.

`toolset_distributions.py` decides *which* toolset(s) are active by default for each surface — the CLI, each gateway platform, the ACP server, subagents — using the distribution rules a maintainer can edit without touching tool code.

---

## 7. Provider/transport layer

### 7.1 The transport ABC (v0.11, NEW)

`agent/transports/base.py`:

> A transport owns the data path for one api_mode:
> `convert_messages → convert_tools → build_kwargs → normalize_response`
> It does NOT own: client construction, streaming, credential refresh,
> prompt caching, interrupt handling, or retry logic. Those stay on AIAgent.

Concrete transports:
- `agent/transports/anthropic.py` — Anthropic Messages API (delegates to `agent/anthropic_adapter.py`)
- `agent/transports/chat_completions.py` — default for ~16 OpenAI-compatible providers
- `agent/transports/codex.py` — OpenAI Responses API (Codex)
- `agent/transports/bedrock.py` — AWS Bedrock Converse API (boto3, not OpenAI SDK)

`get_transport(api_mode)` registry lookup. The transport is selected per-request based on the model's provider metadata (`agent/model_metadata.py` is the 60 KB lookup table).

### 7.2 Provider metadata and routing

- `agent/model_metadata.py` (60 KB) — context length, pricing, supported features, recommended families, fallback hints, **per-provider quirks** (e.g., "send `max_tokens` for Kimi", "clamp `minimal` reasoning effort to `low` on Responses API").
- `agent/models_dev.py` (21 KB) — discovery and merge of `models.dev` entries for lesser-loved providers.
- `agent/usage_pricing.py` (26 KB) — per-token cost computation.
- `agent/credential_pool.py` (65 KB) — multi-key key-pool with rate-limit tracking, OAuth refresh, and provider-specific quotas. The pool is **what makes OpenRouter and Codex OAuth survive long-running gateway sessions** without hand-holding.
- `agent/credential_sources.py` — env, keyring, gh CLI, OAuth flows.
- `agent/google_oauth.py` (38 KB) + `agent/gemini_cloudcode_adapter.py` (34 KB) + `agent/gemini_native_adapter.py` (34 KB) + `agent/gemini_schema.py` + `agent/google_code_assist.py` — Google's auth and API surface is enough work to need its own subsystem.

### 7.3 Anthropic specifics

`agent/anthropic_adapter.py` (76 KB) handles:
- Messages API format (system separate from messages, content blocks, tool_use blocks, tool_result blocks)
- Extended thinking (the `thinking` block; preserve it across turns when prompt caching is on)
- Cache-control breakpoints — `agent/prompt_caching.py` tracks insertion points so they don't move turn-to-turn (the comment in run_agent.py is explicit: **"Prompt Caching Must Not Break"** is one of the codebase's load-bearing invariants)

### 7.4 Tool-call parser zoo

`environments/tool_call_parsers/` is striking — it mirrors VLLM's tool parsers but with **zero VLLM dependency** (only `re`, `json`, `uuid`, `openai` types). Eleven parsers:

| Parser | Format detail |
|---|---|
| `hermes_parser.py` | Hermes-2/-3/-4 native: `<tool_call>{...JSON...}</tool_call>` |
| `llama_parser.py` | Llama 3.x function-call format |
| `qwen_parser.py` | Qwen2/2.5 |
| `qwen3_coder_parser.py` | Qwen3 Coder variant |
| `mistral_parser.py` | Mistral function-call JSON |
| `kimi_k2_parser.py` | Kimi K2 / K2.5 / K2.6 |
| `glm45_parser.py` | GLM-4.5 (`<tool_call>name <arg_key>k</arg_key><arg_value>v</arg_value></tool_call>` — values JSON → ast.literal_eval → raw-string fallback) |
| `glm47_parser.py` | GLM-4.7 (subclass with newline handling) |
| `deepseek_v3_parser.py` | DeepSeek V3 unicode-token format with multi-call support (Issue #989 fix) |
| `deepseek_v3_1_parser.py` | DeepSeek V3.1 (separator order changed) |
| `longcat_parser.py` | Meituan LongCat |

Why this exists: Phase-2 environments use `ManagedServer`'s `/generate` endpoint that returns raw text without VLLM's tool-call parsing. The agent loop runs the parser client-side. This is unusually principled for a feature most frameworks would farm out to their inference server.

---

## 8. Persistent memory, sessions, FTS5

### 8.1 `hermes_state.py` — `SessionDB`

The 2,500-LOC class that owns `~/.hermes/state.db`. SQLite with WAL mode, FTS5 virtual table on the `messages` table, JSON columns for tool_calls/tool_results.

**Schema reconciliation** (`_parse_schema_columns`, `_reconcile_columns`) — on startup, the DB compares the live schema against the expected schema and adds missing columns with `ALTER TABLE`. This is how the codebase ships schema migrations without a migration framework.

**Write serialization** — `_execute_write(fn)` runs all writes through a single connection-bound thread, avoiding "database is locked" with WAL. `_try_wal_checkpoint` is invoked periodically.

**FTS5 specifics**:
- `_sanitize_fts5_query` (line 1448) escapes punctuation while preserving quoted phrases (`_preserve_quoted` regex callback).
- `_is_cjk_codepoint` / `_contains_cjk` / `_count_cjk` — CJK queries get tokenized differently because FTS5's default tokenizer doesn't word-split CJK.
- `search_messages` (line 1531) is the implementation behind `/session_search` and the `session_search` tool.

**Lineage** — sessions can be `reopen_session`'d, `replace_messages`'d on compression, and chained via `parent_session_id`. `_session_lineage_root_to_tip` walks from root to tip; `resolve_resume_session_id` finds the right tip when a user resumes a session by partial ID. `get_next_title_in_lineage` generates `"Title (2)"`-style titles when a session is forked.

**Compression tips** — `get_compression_tip` / `update_compression_tip` store a per-session human-readable summary that `agent/context_compressor.py` consumes when the context gets large.

### 8.2 The memory tool (`tools/memory_tool.py`)

`MemoryStore` (line 107) is a flat-file store backing `MEMORY.md` (facts the agent has learned about its environment, projects, preferences) and `USER.md` (the agent's evolving model of the user). The `memory` tool exposes `set/get/list/delete/scan` operations to the model.

`_scan_memory_content` (line 92) detects and merges duplicate memory entries.

### 8.3 Memory-provider plugins (`plugins/memory/`)

The same `memory` tool surface, six pluggable backends:

| Plugin | Description |
|---|---|
| `honcho/` | [Honcho](https://github.com/plastic-labs/honcho) — dialectic user modeling, "the agent that knows you" |
| `mem0/` | mem0 hosted memory layer |
| `supermemory/` | supermemory.ai hosted |
| `hindsight/` | richer session-scoped retain metadata (v0.11 expansion of #6290) |
| `holographic/` | holographic-encoding research backend |
| `byterover/` | Byterover hosted memory |

The plugin interface is defined in `agent/memory_provider.py`. v0.11 fixed cross-provider tool deduplication so strict providers don't 400 on duplicate tool names, and added `on_memory_write` bridging in the sequential tool execution path.

### 8.4 Context compression (`agent/context_compressor.py`)

When token usage approaches the context window, the compressor:
1. Picks an auxiliary model (configurable; falls back to main model on permanent error)
2. Summarizes the early-middle of the conversation (preserving system + first turn + last N turns)
3. Replaces the compressed region with a single summary user message
4. Updates the SessionDB's compression tip
5. Continues the conversation seamlessly

Smart-collapse, dedup, anti-thrashing, and template upgrade landed in v0.11 (#10088). Compression summaries respect the conversation's language (#12556) — the model isn't forced into English when a user has been chatting in Mandarin.

`/compress` is the user-facing slash command; `/insights [--days N]` and `/usage` consume `agent/insights.py` (39 KB) and `agent/account_usage.py` for token-spend reports.

---

## 9. Skills — procedural memory the agent writes

### 9.1 What a skill is

A skill is a directory with a `SKILL.md` that has YAML frontmatter:

```yaml
---
name: jupyter-live-kernel
description: Use when the user wants to run Python interactively in a Jupyter kernel...
---
```

Plus optional helper scripts, templates, fewshot examples. The agent **discovers** skills via `skills_list`, **reads** them via `skill_view`, and **manages** them (create/edit/version) via `skill_manage`.

### 9.2 Built-in skills (`skills/`, 25 dirs)

`apple`, `autonomous-ai-agents`, `creative`, `data-science`, `devops`, `diagramming`, `dogfood`, `domain`, `email`, `gaming`, `gifs`, `github`, `index-cache`, `inference-sh`, `mcp`, `media`, `mlops`, `note-taking`, `productivity`, `red-teaming`, `research`, `smart-home`, `social-media`, `software-development`, `yuanbao`.

`software-development` alone ships skills like `debugging-hermes-tui-commands`, `hermes-agent-skill-authoring`, `node-inspect-debugger`, `plan`, `python-debugpy`, `requesting-code-review`, `subagent-driven-development`, `systematic-debugging`, `test-driven-development`, `writing-plans` — i.e., the agent has a written playbook for *its own development practices*.

### 9.3 `optional-skills/` (16 dirs)

Heavier or niche skills shipped but not active by default. Activated explicitly. Examples: `autonomous-ai-agents`, `blockchain`, `communication`, `creative`, `devops`, `dogfood`, `email`, `health`, `mcp`, `migration`, `mlops`, `productivity`, `research`, `security`, `web-development`.

### 9.4 The Skills Hub (`tools/skills_hub.py`)

A 119 KB module that is genuinely a small package manager:

- `GitHubAuth` — shared GitHub API auth (PAT, `gh` CLI, GitHub App)
- `SkillSource` ABC — interface for any skill registry adapter
- `OptionalSkillSource` — official optional skills shipped in repo
- `GitHubSource` — install skills from any GitHub repo via the Contents API
- `HubLockFile` — track provenance of installed hub skills (so updates don't clobber user edits)
- Hub state directory management — quarantine, audit log, taps (named registries), index cache

Compatible with the [`agentskills.io`](https://agentskills.io) open standard. Means a skill written for OpenClaw, Cursor's skill bundles, or another `agentskills.io`-compatible agent can be installed into Hermes and vice versa.

### 9.5 Skill creation, lifecycle, sync

- `agent/skill_commands.py` — slash commands like `/skills`, `/<skill-name>`
- `agent/skill_preprocessing.py` — strip frontmatter, expand templates
- `agent/skill_utils.py` — load, validate, merge
- `tools/skills_guard.py` (37 KB) — sanity checks before allowing skill writes (prevent the agent from corrupting its own skills)
- `tools/skills_sync.py` — sync local edits back to source repos
- `tools/skill_manager_tool.py` (31 KB) — the model-facing tool for creating and editing skills
- `tools/skill_usage.py` — record usage stats so unused skills can be pruned

The closed loop: agent does a complex task → autonomously creates a skill if no matching one exists → on next similar task, finds the skill via `skills_list` → reads it via `skill_view` → improves it during use → `skill_manage` writes the improved version back. This is the "self-improving" property that headlines the README.

---

## 10. Plugins — `plugins/`

The plugin surface in v0.11 is genuinely large. A plugin can:

1. **Register a slash command** — `register_command("/foo", handler)`
2. **Dispatch a tool** — `dispatch_tool("tool_name", args)` programmatically
3. **Veto a tool call** — `pre_tool_call(name, args)` returning `False` blocks execution
4. **Rewrite a tool result** — `transform_tool_result(name, args, result) -> str`
5. **Transform terminal output** — `transform_terminal_output(stream, chunk)` (e.g., redact secrets in CI logs before they reach the model)
6. **Provide an image-gen backend** — entries in `plugins/image_gen/` register against `agent/image_gen_registry.py`
7. **Provide a memory backend** — entries in `plugins/memory/`
8. **Provide a context engine** — `plugins/context_engine/` (alternative summarization/retrieval pipelines)
9. **Add a dashboard tab** — `plugins/<name>/dashboard/` with a React component
10. **Provide an observability sink** — `plugins/observability/langfuse/` — Langfuse integration

Bundled plugins:
- `disk-cleanup/` — opt-in reference plugin (`hermes update` cleans old wheels)
- `example-dashboard/` — reference dashboard tab
- `google_meet/` — Google Meet integration
- `image_gen/{openai, openai-codex, xai}/` — image-gen backends
- `memory/{honcho, mem0, supermemory, hindsight, holographic, byterover}/`
- `observability/langfuse/`
- `spotify/`
- `strike-freedom-cockpit/` — community example plugin

User plugins live at `~/.hermes/plugins/`. `hermes_cli/plugins.py` is the loader; `hermes_cli/plugins_cmd.py` is the `hermes plugins …` CLI surface (list/enable/disable/install/remove).

---

## 11. Messaging gateway — `gateway/`

### 11.1 `gateway/run.py`

The 573 KB (!) entry point. The gateway is a long-running async process that:

- Boots all configured platform adapters from `gateway/platforms/`
- Maintains a per-session `AIAgent` cache (`_AGENT_CACHE_MAX_SIZE = 128`, `_AGENT_CACHE_IDLE_TTL_SECS = 3600.0`)
- Enforces LRU + idle-TTL eviction (`_enforce_agent_cache_cap`, `_session_expiry_watcher`)
- Bridges every platform's incoming message to `AIAgent.run_conversation` and routes the response back
- Runs the cron scheduler tick every 60 s (`cron/scheduler.py`)
- Auto-continues interrupted gateway turns when the gateway restarts (with a freshness check so stale tool-tail markers don't revive an old task — v0.11 #9934)

### 11.2 `gateway/session.py`

Tracks session context (where messages come from), persistence to disk, reset policy, and **dynamic system-prompt injection** so the agent knows which platform / chat / user it's talking to. Includes deterministic PII hashing helpers (`_hash_id`, `_hash_sender_id`, `_hash_chat_id`) so analytics and logs can reference users without storing PII — `telegram:12345 → telegram:<sha256-12char>`.

### 11.3 `gateway/platforms/` — 17 adapters

| Platform | File | Size | Notes |
|---|---|---|---|
| Telegram | `telegram.py` | 141 KB | streaming, photo/voice/document, inline mode |
| Discord | `discord.py` | 173 KB | slash-cmd, threads, voice, stickers |
| Slack | `slack.py` | 102 KB | Socket Mode, threads, file uploads, reactions |
| WhatsApp | via `bluebubbles.py` (35 KB) and the `whatsapp-bridge` script | ↑ | iMessage bridging |
| Signal | `signal.py` | 39 KB | signal-cli backend |
| Email | `email.py` | 23 KB | IMAP/SMTP, threading |
| SMS | `sms.py` | 14 KB | Twilio |
| Matrix | `matrix.py` | 105 KB | sync, E2EE, threads |
| Mattermost | `mattermost.py` | 28 KB | webhook + REST |
| Home Assistant | `homeassistant.py` | 16 KB | conversation entity |
| Feishu | `feishu.py` | 192 KB! | + `feishu_comment.py` (51 KB), `feishu_comment_rules.py` (14 KB) |
| DingTalk | `dingtalk.py` | 56 KB | enterprise IM |
| WeCom | `wecom.py` (65 KB), `wecom_callback.py` (16 KB), `wecom_crypto.py` (5 KB) | |
| Weixin | `weixin.py` | 79 KB | personal WeChat (HermesClaw is the community bridge) |
| QQBot | `qqbot/` | NEW in v0.11 | Official API v2, QR-scan setup |
| BlueBubbles (iMessage) | `bluebubbles.py` | 35 KB | macOS bridge |
| API server | `api_server.py` | 118 KB | HTTP + SSE for self-hosted UIs |
| Webhook | `webhook.py` | 31 KB | generic in/out + direct-delivery mode (v0.11 #12473) |

Plus `gateway/platforms/base.py` (119 KB) — the shared lifecycle: connect, listen, normalize messages, send/edit/react, handle attachments, surface "typing" indicators where supported, manage cursors for live streaming.

`gateway/platforms/ADDING_A_PLATFORM.md` is the maintainer's checklist.

### 11.4 Cross-platform machinery

- `gateway/channel_directory.py` — name resolution (`telegram:@username` → numeric ID)
- `gateway/delivery.py` — outbound retry/queue logic
- `gateway/display_config.py` — per-platform Markdown rendering rules (Telegram MarkdownV2 ≠ Slack mrkdwn ≠ Discord ≠ Matrix HTML)
- `gateway/hooks.py` + `gateway/builtin_hooks/` — extension points
- `gateway/mirror.py` — message mirroring across platforms (Slack-to-Discord-to-Telegram bridging)
- `gateway/pairing.py` — DM pairing flow (the way you bind a Telegram chat to your Hermes user)
- `gateway/runtime_footer.py` — `_running on Hermes v0.11.0_` style suffixes
- `gateway/status.py` — `/status` command implementation
- `gateway/sticker_cache.py` — sticker dedup
- `gateway/stream_consumer.py` — chunked-delta streaming, sanitization (the cursor sanitization fixes in v0.11 are here)
- `gateway/whatsapp_identity.py` — phone-number ↔ WhatsApp ID mapping

### 11.5 The UX wins

Three details capture the philosophy:

1. **Voice-memo transcription**. Send a voice note on Telegram → `transcription_tools.py` runs Whisper-class STT → the transcript becomes the user message. Reply is read aloud via `tts_tool.py` if voice mode is enabled.
2. **Cross-platform conversation continuity**. Start a chat on Telegram, switch to Discord — the session DB binds them by user identity, so the agent picks up where you left off.
3. **Activity heartbeats** prevent false inactivity timeouts when a tool is running for a long time (v0.11 #10501).

---

## 12. Execution environments — six terminal backends

`tools/environments/`:

| Backend | File | Purpose |
|---|---|---|
| Local | `local.py` (15 KB) | Direct on the host. Fastest. Default. |
| Docker | `docker.py` (23 KB) | Per-session container, isolated from host |
| SSH | `ssh.py` (12 KB) | Remote host via SSH; persistent |
| Modal | `modal.py` (17 KB) + `managed_modal.py` (10 KB) + `modal_utils.py` (6 KB) | Modal cloud sandbox; managed-gateway variant |
| Daytona | `daytona.py` (10 KB) | Daytona dev sandboxes |
| Singularity | `singularity.py` (9 KB) | HPC environments |

`tools/environments/base.py` (31 KB) defines the `Environment` ABC:
- `exec(cmd, *, cwd=None, env=None, timeout=None, background=False) -> ExecResult`
- `read_file`, `write_file`, `patch`
- `start`, `stop`, `cleanup`
- `keepalive`, `is_alive`

`tools/environments/file_sync.py` (16 KB) handles host↔sandbox file syncing for Modal/Daytona where the working directory may not be shared.

The Modal and Daytona backends are the killer feature: **the sandbox hibernates between turns**. A Daytona workspace at idle costs ~0; the next message wakes it on demand. This is why "$5 VPS or GPU cluster" is the marketing line — the runtime is decoupled from the agent process.

`TERMINAL_ENV` env var picks the default; per-tool overrides are possible via tool args. The `terminal` tool internally calls `get_active_env()` which returns the configured backend's instance.

---

## 13. Cron and scheduled automations — `cron/`

### 13.1 `cron/scheduler.py`

`tick()` is called every 60 s by the gateway. File-based locking (`fcntl` on Unix, `msvcrt` on Windows) at `~/.hermes/cron/.tick.lock` ensures only one tick runs at a time even with multiple gateway processes.

### 13.2 `cron/jobs.py`

Job structure: `{name, schedule (cron expr), prompt, target_platform, target_chat, enabled, last_run, ...}`. Storage is JSON files under `~/.hermes/cron/jobs/`.

Each due job spawns an `AIAgent` (auxiliary or main model per config), runs the prompt in a fresh session, and **delivers the result to a configured platform target** — Telegram chat, Slack channel, email, etc. So "every morning at 8am, send a daily summary of my unread Slack DMs to my private Telegram" is a one-liner cron job.

### 13.3 `tools/cronjob_tools.py`

Model-facing tool: create / list / update / pause / resume / remove / trigger. The agent itself can set up cron jobs ("Hey Hermes, every Friday at 5pm send me a weekly summary of my GitHub PRs.").

---

## 14. Subagents, parallelism, and `delegate_tool.py`

`tools/delegate_tool.py` is **107 KB** — the largest single tool module. Why?

### 14.1 What `delegate_task` does

`delegate_task(prompt, toolset, model=None, max_iterations=None, ...)` spawns an isolated subagent with its own:
- Session ID (parented to the caller)
- IterationBudget (default 50)
- Toolset (the parent's MCP toolsets are inherited if `_get_inherit_mcp_toolsets()` returns True)
- Working directory checkpoint (so the parent's filesystem state is preserved)
- File-coordination handle (v0.11 #13718) — so concurrent sibling subagents don't clobber each other's edits

### 14.2 Spawn safety

- `_get_max_concurrent_children()` — caps siblings (default 4)
- `_get_max_spawn_depth()` — caps nesting (v0.11 default `flat` = 1, configurable; the `orchestrator` role can spawn its own workers, deeper)
- `_get_orchestrator_enabled()` — gates whether subagents themselves can call `delegate_task`
- `_get_child_timeout()` — wall-clock cap on a child's runtime
- `set_spawn_paused(bool)` / `is_spawn_paused()` — emergency stop
- `_subagent_auto_deny` / `_subagent_auto_approve` — approval policy for the child (the parent answers approval prompts on the child's behalf, with configurable defaults)

### 14.3 Tracking and interruption

`_register_subagent` / `_unregister_subagent` keep a process-global list (`list_active_subagents()`). `interrupt_subagent(subagent_id)` cooperatively stops a running child. The CLI command `/subagents` and the TUI's subagent overlay (v0.11) both consume this.

### 14.4 Output handling

`_extract_output_tail` returns a configurable tail of the child's output (subagents can run for many iterations and the parent doesn't need every byte). `_looks_like_error_output` classifies whether the tail is signal vs. noise so the parent's compression can be smart about including it.

### 14.5 `_cap_delegate_task_calls` (run_agent.py:5096)

Even with `_should_parallelize_tool_batch`, the parent caps the number of concurrent `delegate_task` calls per turn — a model that emits 10 separate `delegate_task` calls in one turn won't actually spawn 10 children; the cap normalizes this.

---

## 15. Trajectory compression and OPD RL

### 15.1 `trajectory_compressor.py` — 2,000 LOC

Post-processes completed agent trajectories (saved as JSONL when `save_trajectories=True`) to fit a target token budget while preserving training signal. Strategy from the docstring:

1. Protect first turns (system, human, first gpt, first tool)
2. Protect last N turns (final actions and conclusions)
3. Compress MIDDLE turns only, starting from the 2nd tool response
4. Compress only as much as needed to fit under target
5. Replace compressed region with a single human summary message
6. Keep remaining tool calls intact (model continues working after summary)

`CompressionConfig` defines the compressor model + parameters; `TrajectoryMetrics` and `AggregateMetrics` capture per-trajectory and dataset-level stats; `TrajectoryCompressor` is the top-level driver.

`scripts/sample_and_compress.py` is the typical pipeline runner. The CLI also exposes `python trajectory_compressor.py --input=data/run --sample_percent=15` for partial dataset compression.

### 15.2 Atropos environments — `environments/`

Atropos is Nous's RL framework. Hermes ships environments to use Atropos to *train models on the Hermes tool-calling loop*:

- `environments/agent_loop.py` (24 KB) — `HermesAgentLoop`, the reusable multi-turn engine. Phase 1: standard OpenAI tool-calling against any OpenAI-spec server. Phase 2: VLLM `ManagedServer` with client-side parser (the parsers in §7.4).
- `environments/hermes_base_env.py` (29 KB) — the base Atropos environment that wraps the agent loop and computes scalar rewards.
- `environments/agentic_opd_env.py` (51 KB) — **AgenticOPDEnv**, the on-policy distillation environment. The headline:

  > Every time an agent receives a next-state signal (tool result, error trace,
  > test verdict), that signal contains hindsight information about how the
  > agent's PREVIOUS response could have been better. This environment …
  > uses an LLM judge to extract "hints" … builds an enhanced prompt …
  > scores the student's response tokens under the enhanced distribution
  > using VLLM's `prompt_logprobs` … packages the teacher's top-K predictions
  > as `distill_token_ids` / `distill_logprobs` on the ScoredDataGroup.

  Per-token advantage: `A_t = teacher_logprob(token_t) - student_logprob(token_t)`. Dense, token-level training signal from every tool interaction.

- `environments/web_research_env.py` — web research benchmark
- `environments/hermes_swe_env/` — SWE-bench-style env with `default.yaml` config
- `environments/terminal_test_env/` — terminal-task benchmark
- `environments/benchmarks/{tblite, terminalbench_2, yc_bench}` — concrete benchmarks
- `environments/tool_call_parsers/` — see §7.4
- `environments/tool_context.py` — the per-tool context object passed into tool handlers in this Atropos path
- `environments/patches.py` — small monkey-patches the agent loop applies for the RL path

### 15.3 `tools/rl_training_tool.py`

Allows the running agent to *spawn an Atropos training run from inside a conversation*. Closed loop: chat with Hermes → ask it to train a better Hermes on this trajectory → it runs `tinker-atropos` and reports back.

`tinker-atropos/` (top-level dir) holds the `tinker` integration glue.

`batch_runner.py` is the production-grade parallel batch trajectory generator — used to generate the `hermes-function-calling-v1` dataset and successors.

---

## 16. The Ink TUI (v0.11) — `ui-tui/` + `tui_gateway/`

### 16.1 Why a TUI rewrite

`hermes` historically used Rich + `prompt_toolkit`. v0.11's `hermes --tui` is a full rewrite in **Ink (React in the terminal)** with a Python backend. The motivation, from the release notes: a sticky composer, live streaming with OSC-52 clipboard support, stable picker keys, status bar with per-turn stopwatch and git branch, `/clear` confirm, light-theme preset, and a subagent-spawn observability overlay. ~310 commits.

### 16.2 The split

- `ui-tui/` — TypeScript/React/Ink. Files: `entry.tsx`, `app.tsx`, `gatewayClient.ts`, `gatewayTypes.ts`, `theme.ts`, `protocol/`, `app/components/hooks/lib`, `__tests__/`. Bundler is Babel + esbuild via `babel.compiler.config.cjs`.
- `tui_gateway/` — Python JSON-RPC backend. Files: `entry.py`, `server.py`, `transport.py` (stdio / unix socket / WebSocket via `ws.py`), `event_publisher.py`, `render.py`, `slash_worker.py`. The TS frontend talks JSON-RPC to this backend, which owns the actual `AIAgent` and `SessionDB`.

### 16.3 Why this architecture?

1. **Render the same TUI inside the dashboard's `/chat` page.** The `ws.py` transport is used unchanged by the web dashboard's chat tab — same component tree, same backend, no re-implementation.
2. **Hot reload.** React-style stateful UI is meaningfully easier to evolve than a procedural Rich render loop.
3. **Stable input.** Ink solves the input-while-streaming problem (the sticky composer) cleanly.

### 16.4 Slash commands

Slash-command flow for the TUI: TS frontend → JSON-RPC `runSlash {name, args}` → `slash_worker.py` resolves the command in the same registry the CLI uses (`hermes_cli/commands.py`) → result streamed back as JSON-RPC events.

---

## 17. Sibling repo: `Hermes-Function-Calling` — line-by-line

This repo is small enough to actually read line-by-line. **1,034 LOC across 7 Python files**, MIT-licensed, last meaningful update in 2024 (it's effectively a frozen reference implementation that paired with the `Hermes-2-Pro-Llama-3-8B` release; the production successor is the agent-side parsing in `hermes-agent`).

### 17.1 `schema.py` (23 LOC)

Three Pydantic v1 models:

```python
class FunctionCall(BaseModel):
    arguments: dict       # JSON-deserialized args
    name: str             # function name

class FunctionDefinition(BaseModel):
    name: str
    description: Optional[str]
    parameters: Optional[Dict[str, object]]   # OpenAI JSON-schema shape

class FunctionSignature(BaseModel):
    function: FunctionDefinition
    type: Literal["function"]
```

These are the canonical types used by `validator.py`. `FunctionCall.arguments` is a `dict` (the schema doesn't try to type-validate args at this layer — that's `validate_argument_type`).

### 17.2 `prompter.py` (77 LOC) — `PromptManager`

`PromptSchema` is a Pydantic model with six string fields: `Role, Objective, Tools, Examples, Schema, Instructions`.

`format_yaml_prompt(prompt_schema, variables)` walks the fields, format-substitutes `{tools}`, `{schema}`, `{examples}`, `{date}` etc., and concatenates. Skips `Examples` if no examples are supplied. Replaces newlines with spaces for all sections except `Instructions` (so the actual constraints render legibly while metadata is collapsed).

`read_yaml_file(file_path)` loads `prompt_assets/sys_prompt.yml` and pours it into `PromptSchema`.

The system prompt itself, from `prompt_assets/sys_prompt.yml`, is the **canonical Hermes function-calling prompt**:

```text
Role: You are a function calling AI agent with self-recursion. You can call only
one function at a time and analyse data you get from function response. You are
provided with function signatures within <tools></tools> XML tags. The current
date is: {date}.

Objective: ... Please call a function and wait for function results to be
provided to you in the next iteration. Don't make assumptions about what values
to plug into function arguments. Once you have called a function, results will
be fed back to you within <tool_response></tool_response> XML tags. ...

Tools: Here are the available tools: <tools>{tools}</tools>
  If the provided function signatures doesn't have the function you must call,
  you may write executable python code in markdown syntax and call
  code_interpreter() function as follows:
  <tool_call>{"name": "code_interpreter", "arguments": {"code_markdown": <python-code>}}</tool_call>

Examples: {examples}

Schema: Use the following pydantic model json schema for each tool call you
will make: {schema}

Instructions: ... call one function at a time ... For each function call return
a valid json object (using double quotes) with function name and arguments
within <tool_call></tool_call> XML tags as follows:
  <tool_call>{"name": <function-name>, "arguments": <args-dict>}</tool_call>
```

Two architectural decisions are visible:

1. **`<tool_call>...</tool_call>` is XML containing JSON.** This is "Hermes format" — it's the format that all Hermes parsers (`environments/tool_call_parsers/hermes_parser.py`) expect, and it's why Hermes-2-Pro and successors are reliable function-callers without needing a structured-output grammar.
2. **`code_interpreter` is the universal escape hatch.** When a function isn't available, the model writes Python and calls `code_interpreter`. This is the primordial form of `execute_code` in the modern agent.

### 17.3 `validator.py` (131 LOC)

The argument-validation core.

`validate_function_call_schema(call, signatures) -> (bool, error)`:
- Pydantic-parses the call as `FunctionCall`
- Iterates `signatures`, finds the one matching `call.name`
- For each property in the matched signature's `parameters`, calls `validate_argument_type`
- Then checks all `required` arguments are present via `check_required_arguments`

`validate_argument_type(arg_name, arg_value, arg_schema)`:
- If type is `string` and `enum` is present, `validate_enum_value` raises if the value isn't in the enum (skipping if enum is `[]` or contains `None`)
- Otherwise, maps the JSON-schema type to a Python type via `get_python_type` and uses `isinstance`

`get_python_type` is the canonical JSON-schema → Python mapping:

```python
{'string': str, 'number': (int, float), 'integer': int,
 'boolean': bool, 'array': list, 'object': dict, 'null': type(None)}
```

`validate_json_data(json_object, json_schema)` — the JSON-mode validator:
1. Try `json.loads`
2. Fall back to `ast.literal_eval` (handles single-quoted "JSON")
3. Fall back to `extract_json_from_markdown` from `utils.py` (strips ` ```json fences)
4. If a list, validate each item against the schema; otherwise validate as scalar

This three-step JSON parse is **also** the strategy used in `glm45_parser.py` and elsewhere in `hermes-agent` — it's the institutional answer to "models emit not-quite-JSON."

### 17.4 `functioncall.py` (181 LOC) — `ModelInference`

The recursive function-call inference loop.

`__init__`:
- Loads the model via `transformers.AutoModelForCausalLM` with `attn_implementation="flash_attention_2"` and `torch_dtype=torch.float16`
- Optional 4-bit quant via `BitsAndBytesConfig(load_in_4bit=True, bnb_4bit_quant_type="nf4", bnb_4bit_use_double_quant=True)`
- Loads `AutoTokenizer`, sets `pad_token = eos_token`, `padding_side = "left"`
- If the tokenizer has no chat template, falls back to a hand-coded one from `utils.get_chat_template(name)` (`chatml` is the default)

`process_completion_and_validate(completion, chat_template)`:
- Extracts the assistant message via `utils.get_assistant_message`
- Calls `utils.validate_and_extract_tool_calls` (parses `<tool_call>` XML and JSON within)
- Returns `(tool_calls, assistant_message, error_message)`

`execute_function_call(tool_call)`:
- `getattr(functions, function_name)` — the function module is the registry
- `function_to_call(*function_args.values())` — positional invocation by argument-dict-value order
  - Note: this assumes the model's argument order matches the function's parameter order. The modern `hermes-agent` uses keyword invocation, which is more robust.

`generate_function_call(query, chat_template, num_fewshot, max_depth)` (the recursive driver):
1. Build the system prompt via `PromptManager.read_yaml_file` + `format_yaml_prompt`
2. Tokenize the chat
3. `model.generate(...)` with `max_new_tokens`, etc.
4. Decode → `process_completion_and_validate`
5. If `tool_calls`, execute each via `execute_function_call`, re-build the prompt with `<tool_response>...</tool_response>` appended
6. Recurse, decrementing `max_depth` (default 5)
7. Return final assistant text when no more tool calls or depth exhausted

### 17.5 `jsonmode.py` (150 LOC)

Same skeleton as `functioncall.py` but for **structured-output JSON mode**. Builds a system prompt that includes a Pydantic model's JSON schema, generates, then `validate_json_data` against the schema. Recurses on validation failure with an error message included in the next turn (giving the model a chance to fix its output).

This is the predecessor of OpenAI's structured-output / `response_format` API and is functionally equivalent to "JSON mode against a Pydantic schema." The `Character` example in the README (Goku from Dragon Ball Z) is intentionally trivial.

### 17.6 `functions.py` (313 LOC)

A demonstration toolkit using `@tool` decorators from LangChain (`langchain.tools.tool` + `convert_to_openai_tool`). Functions cover stock prices, company fundamentals, financial statements via `yfinance`, plus a `code_interpreter(code_markdown: str)` that runs Python via `exec`.

`get_openai_tools() -> List[dict]` is the inventory function — returns the OpenAI-format tool schema list. The README shows how to add a new tool: define `@tool def foo(...)`, append to the `functions` list in `get_openai_tools`.

The `code_interpreter` tool is the canonical reference implementation — it's `exec`-based with a captured stdout/stderr trap. Production Hermes replaces this with `tools/code_execution_tool.py` (62 KB) which runs in the configured environment (Modal sandbox, Docker, etc.) rather than on the host.

### 17.7 `utils.py` (159 LOC)

Helpers consumed everywhere else:
- `print_nous_text_art()` — banner ASCII art
- `inference_logger` — module-scoped `logging.Logger` with a clean format
- `get_assistant_message(completion, chat_template, eos_token)` — locates the model's response in the decoded text by chat-template-aware boundary detection
- `get_chat_template(name)` — returns the Jinja chat template string for `chatml`, `vicuna`, `zephyr`, etc., used as a fallback when the tokenizer lacks one
- `validate_and_extract_tool_calls(assistant_message)` — the XML/JSON parser for `<tool_call>{...}</tool_call>` blocks (with the same three-step JSON-parse strategy as `validator.py`)
- `extract_json_from_markdown(text)` — strips ` ```json ` fences
- `get_fewshot_examples(num_fewshot)` — loads from `prompt_assets/few_shot.json`

### 17.8 What this repo establishes for the modern agent

Everything `hermes-agent` does at scale, this repo prototyped:

| Pattern | Hermes-Function-Calling | hermes-agent |
|---|---|---|
| `<tool_call>` XML-around-JSON format | `prompt_assets/sys_prompt.yml` | `environments/tool_call_parsers/hermes_parser.py` |
| Recursive tool-execution loop | `functioncall.py:generate_function_call` | `run_agent.py:run_conversation` |
| Three-step JSON parse | `validator.py:validate_json_data` | `run_agent.py:_repair_tool_call_arguments` |
| JSON-schema → Python type coercion | `validator.py:get_python_type` | `model_tools.py:_coerce_value` |
| `code_interpreter` escape hatch | `functions.py:code_interpreter` | `tools/code_execution_tool.py` |
| Fewshot prompt assets | `prompt_assets/few_shot.json` | `agent/prompt_builder.py` |
| Pydantic JSON-mode | `jsonmode.py` | OpenAI structured outputs / Anthropic tool_use |

---

## 18. Security model

Hermes is unusually thoughtful about safety for an open agent framework. Three layers:

### 18.1 Approval

`tools/approval.py` (53 KB) implements per-command approval with:
- Allow/deny patterns (regex)
- One-shot vs. session vs. permanent approval
- Per-command audit log
- DM-pairing requirements (a Telegram chat must be DM-paired with the user's Hermes account before approval prompts are accepted from that chat)

The approval flow runs inside the `terminal` and `code_execution` tools; "subagent_auto_approve" / "subagent_auto_deny" hooks let the parent answer for the child.

### 18.2 Sandboxing

`tools/path_security.py` — a `read_file`/`write_file`/`patch` outside the configured working directory is rejected by default. The model can ask for a working-directory change via `clarify`.

`tools/url_safety.py` — the browser tool screens URLs against a deny-list (file://, internal IPs, etc.).

`tools/website_policy.py` — per-domain rules.

`tools/osv_check.py` — when reading lockfiles, surface OSV CVEs.

`tools/tirith_security.py` (26 KB) — the policy engine. Tirith ("watcher" in Tolkien) appears to be Nous's homegrown policy DSL; it gates dangerous operations using configured rules.

`tools/binary_extensions.py` — refuse to read binary files with `read_file` (waste of context, possible safety issue).

`tools/credential_files.py` (15 KB) — detect and refuse to expose credential-bearing files (`.env`, `~/.aws/credentials`, etc.) by default.

### 18.3 Credentials & redaction

`agent/credential_pool.py` (65 KB) — multi-key pool with rate-limit tracking (so a key that hits its quota is rotated out for the cooldown window) and OAuth refresh.

`agent/redact.py` (15 KB) — log/transcript redaction. Patterns for API keys, tokens, AWS access keys, etc. Used both at log time and at trajectory-save time.

`agent/file_safety.py` — file-write guards (don't overwrite outside known dirs).

The `_hash_*` helpers in `gateway/session.py` ensure analytics never see raw user IDs.

---

## 19. Test surface

`tests/` is **~700 files, ~15,000 tests**. Organized by mirror to source:

- `tests/agent/` — provider adapters, compression, memory
- `tests/cli/` — slash-command parsing, `hermes <subcmd>` end-to-end
- `tests/cron/` — scheduler tick semantics
- `tests/environments/` — RL env smoke tests
- `tests/gateway/` — per-platform unit tests (Telegram-style mocks)
- `tests/hermes_cli/`, `tests/hermes_state/` — module-level
- `tests/integration/` — multi-system integration (gateway + agent + DB)
- `tests/e2e/` — full conversation recordings replayed
- `tests/plugins/`, `tests/skills/` — plugin/skill loader
- `tests/run_agent/` — the conversation loop (with mock providers)
- `tests/tools/` — every tool
- `tests/tui_gateway/` — TUI JSON-RPC backend
- `tests/website/` — Docusaurus link checker
- `tests/honcho_plugin/`, `tests/yuanbao*` — third-party integrations
- `tests/fakes/` — shared mock fixtures (mock OpenAI, mock SessionDB)

Top-level test files (e.g., `test_sql_injection.py`, `test_atomic_replace_symlinks.py`, `test_subprocess_home_isolation.py`) are security-leaning unit tests that have outgrown a category folder. `tests/conftest.py` sets up shared fixtures.

`scripts/run_tests.sh` probes `.venv` then `venv` then `$HOME/.hermes/hermes-agent/venv`, activates it, and runs `pytest`.

---

## 20. Release cadence and what changed in v0.11.0

The repo ships **weekly**. Versioning is `vYYYY.M.D` (calendar) plus a semver-like `v0.X.0`. Recent tags:

- v0.8.0 (April 8, 2026) — background task auto-notifications, free MiMo v2 Pro on Nous Portal, live model switching, native Google AI Studio, smart inactivity timeouts, approval buttons.
- v0.9.0 (April 13, 2026) — mobile/Termux/Android, iMessage and WeChat, Fast Mode for OpenAI/Anthropic, background process monitoring, security hardening across 16 platforms.
- v0.10.0 (April 16, 2026) — Tool Gateway: web search, image generation, TTS, browser automation for paid Nous Portal.
- **v0.11.0 (April 23, 2026) — "the Interface release"**
  - Full React/Ink TUI rewrite with Python JSON-RPC backend
  - `agent/transports/` ABC extracted; native AWS Bedrock support
  - Five new providers: NVIDIA NIM, Arcee AI, Step Plan, Gemini CLI OAuth, Vercel ai-gateway
  - GPT-5.5 over Codex OAuth
  - QQBot — 17th platform
  - Plugin surface: register_command, dispatch_tool, pre_tool_call veto, transform_tool_result, transform_terminal_output, image_gen backends, dashboard tabs
  - `/steer <prompt>` mid-run agent nudges
  - Shell hooks (lifecycle hooks via shell scripts, no Python plugin needed)
  - Webhook direct-delivery mode (zero-LLM push notifications)
  - Subagent orchestrator role + cross-agent file-state coordination
  - Auxiliary models UI + main-model-first auto routing
  - Dashboard plugin system + live theme switching
  - Compressor: smart collapse, dedup, anti-thrashing, language-respecting summaries
  - Auto-prune old sessions + VACUUM state.db at startup

1,556 commits and 761 merged PRs since v0.9.0. 29 community contributors (290 including co-authors).

---

## 21. Design analysis

### 21.1 Strong design choices

1. **The registry decouples tools from everything.** Adding a new tool is `tools/foo.py` with `registry.register(...)` at module top — no imports to update, no central tool list to edit. The AST scan in `_module_registers_tools` makes this safe even when modules accidentally call `register()` from inside functions.
2. **Toolsets are an explicit escape valve for context economy.** Naming this concept — and making it configurable per-platform via `toolset_distributions.py` — is the right call once you have 40+ tools.
3. **The transport ABC was the right refactor.** Until v0.11, format conversion was inlined into `run_agent.py`. Pulling it into `agent/transports/` made native Bedrock land in three PRs.
4. **The MCP-server side (`mcp_serve.py`) closes the loop.** Hermes can be both a *client* of MCP servers (`tools/mcp_tool.py`) **and** a *server* exposed to other agents (`mcp_serve.py`). The 10-tool surface mirrors OpenClaw's bridge for compatibility. This is a force multiplier — Claude Code, Cursor, and Codex can drive Hermes' 17 messaging platforms via MCP.
5. **The plugin surface is real.** Veto, rewrite, dispatch, transform, command — the five hooks added in v0.11 cover the actual extension points a plugin author needs. Compare to frameworks where "plugin" means "decorator that registers a callback you can never intercept."
6. **Per-token RL signal in `agentic_opd_env.py`.** Shipping an Atropos environment that gives token-level distillation training data from agentic rollouts (instead of a scalar reward) is genuinely novel; it's the single most research-leveraging artifact in the repo.

### 21.2 Load-bearing structure (don't break)

1. `tools/registry.py` is imported by everything. Any change to its public surface breaks every tool.
2. `IterationBudget` is the only thing keeping a runaway agent from spending unbounded credits. Subagent-budget independence is deliberate but means the *total* budget is unbounded if `max_spawn_depth > 1`. Configure carefully.
3. Prompt cache breakpoint stability — the "Prompt Caching Must Not Break" comment is real. Anthropic's prompt cache silently invalidates if the cache-control breakpoint moves; always run cache-impact tests on system-prompt changes.
4. SessionDB schema reconciliation — `_reconcile_columns` only adds columns. Renames or type changes need an explicit migration.
5. Tool-call parser drift — when a model family ships a new tool-call format (DeepSeek V3 → V3.1), the parsers have to track. v0.11 ships parsers for 11 families.

### 21.3 Risks and watch-points

1. **The single-file giants.** `run_agent.py` (~13.5k LOC) and `cli.py` (~11k LOC) are productivity tax. v0.11 already cracked one open by extracting transports; the agent loop's pre-flight + retry counters are the next obvious extraction.
2. **`tools/browser_tool.py` (121 KB) and `tools/mcp_tool.py` (127 KB) are huge for a single tool module.** Browser likely warrants a sub-package. MCP already has `mcp_oauth.py` and `mcp_oauth_manager.py` siblings; pulling more of `mcp_tool.py` out into composable units would improve maintainability.
3. **Provider-quirk drift.** Every provider adapter encodes "this model wants `max_tokens`, this one doesn't, this one needs a `service_tier` mapping." This is the kind of code that decays — a regression test per quirk is the only way to keep it honest. Most are covered in `tests/agent/`, but coverage is incomplete by construction (some quirks are only visible against the live API).
4. **Concurrent-subagent file coordination.** v0.11 #13718 added cross-agent file state coordination, but the protocol ("siblings see each other's edits before commit") is subtle. The unit test surface there is worth surveying before shipping orchestrator-of-orchestrators behaviour.
5. **The TUI rewrite is fresh.** Ink/React is new code; the old `cli.py` UI path is still the default. Watch for parity bugs between the two surfaces over the next few releases.

### 21.4 The Idiot Index assessment

For the problem ("a self-improving agent that runs anywhere, talks anywhere, swaps any model"), the codebase is **dense, not bloated**. ~600k LOC of Python feels like a lot until you remember it's:
- 17 messaging-platform adapters (each platform's API is non-trivial)
- ~15 LLM-provider adapters (each format is non-trivial)
- 11 tool-call parsers (each model family is non-trivial)
- 6 execution backends
- A SQLite/FTS5 store
- 25 built-in skills + 16 optional + a Skills Hub registry
- A web dashboard + a TUI rewrite + a Docusaurus site
- An ACP server and an MCP server
- An RL environment for distillation training

Few items in the codebase look like make-work. The size scales with the integration surface, which is the actual product.

---

## 22. Sources

Primary sources read for this report:

- [`NousResearch/hermes-agent`](https://github.com/NousResearch/hermes-agent) — main @ commit `5e68503`
  - `README.md`
  - `AGENTS.md` — developer guide
  - `RELEASE_v0.11.0.md` — full v0.11 changelog
  - `run_agent.py` — `AIAgent`, `IterationBudget`, parallel-batch policy
  - `model_tools.py` — `handle_function_call`, `coerce_tool_args`
  - `toolsets.py` — `_HERMES_CORE_TOOLS`, `TOOLSETS`
  - `tools/registry.py` — discovery and registration
  - `tools/delegate_tool.py` — subagent orchestration
  - `tools/skills_hub.py` — Skills Hub source adapters
  - `tools/memory_tool.py` — `MemoryStore`
  - `tools/terminal_tool.py` — environment dispatch
  - `tools/environments/{base,local,docker,modal,daytona,ssh,singularity,managed_modal,file_sync,modal_utils}.py`
  - `agent/transports/{base,anthropic,bedrock,chat_completions,codex,types}.py`
  - `agent/__init__.py` and the 45 sibling modules
  - `gateway/run.py`, `gateway/session.py`, `gateway/platforms/*.py`
  - `gateway/platforms/ADDING_A_PLATFORM.md`
  - `environments/agent_loop.py`, `environments/hermes_base_env.py`, `environments/agentic_opd_env.py`
  - `environments/tool_call_parsers/*.py`
  - `cron/scheduler.py`, `cron/jobs.py`
  - `hermes_state.py` — `SessionDB`
  - `acp_adapter/server.py`
  - `mcp_serve.py`
  - `trajectory_compressor.py`
- [`NousResearch/Hermes-Function-Calling`](https://github.com/NousResearch/Hermes-Function-Calling) — full repo, line-by-line
  - `schema.py`, `prompter.py`, `validator.py`, `functioncall.py`, `jsonmode.py`, `functions.py`, `utils.py`
  - `prompt_assets/sys_prompt.yml`, `prompt_assets/few_shot.json`
  - `README.md`
- Public docs:
  - [Hermes Agent docs](https://hermes-agent.nousresearch.com/docs/) — architecture, features overview, environments page
  - [Nous Research](https://nousresearch.com)
  - [Skills Hub / agentskills.io](https://agentskills.io)
  - [DeepWiki: NousResearch/hermes-agent](https://deepwiki.com/NousResearch/hermes-agent)

---

*End of report. Prepared April 29, 2026.*
