# Argo Roadmap

The canonical phase plan lives in `docs/plan/argo-build-plan.md` §27.

## Current phase

**Library + integration tier complete. Live agent loop with tool
dispatch + spend tracking running across all four provider
families. Streaming end-to-end (chat + gateway).** Phases 0–7 +
4.5 + 6.5 + 7.5 all ship at the library tier across **196+ PRs**,
with zero LOC-budget violations across the ~37k-line workspace.

`argo chat <id> [--user "..."] [--stream] [-i]` drives a real
multi-turn agent loop against a recorded session. Each round
calls the provider, persists the assistant message, and — if
the response carried `tool_calls` — dispatches each through
`argo_tool::ToolRegistry` (capability-checked via
`CapabilityEngine`), appends a tool-role message with the
result, and runs another round. Capped at `MAX_TOOL_ROUNDS = 8`
to bound runaway loops. Every turn is hash-chained into the
audit log (`TurnStart → AssistantMessage → ToolCall → ToolResult
→ TurnEnd`) and recorded into `spend_ledger` from the model's
`price_per_1k_*` fields × the provider-reported token counts.
Dispatches across **all four** v1 families now: Anthropic /
OpenAI Compatible / OpenAI Responses / Bedrock (the last via
`aws_config::load_defaults`). The same `run_turn` helper backs
the gateway's `POST /sessions/<id>/runs`. `argo provider test`
runs the same Transport ladder for a one-shot ping with token
counts + reply snippet.

The CLI binary (`argo`) drives ~55 subcommands. Beyond the read-
side surface (`session list/show/diff`, `memory list/view`,
`skill list/view`, `cron list`, `provider show`, `audit
show/verify`, `costs show` — now also rolling up actual spend
across today / month-to-date / all-time, plus `--by-model` /
`--by-session` breakdowns and `costs export` for NDJSON ledger
dumps — all `--json`-capable), `argo new` /
`argo resume` / `argo chat` / `argo steer` cover the full session
lifecycle. `argo session show` + `argo session list` surface
per-session tool_call_count + spend, and the gateway's
`GET /sessions` and `GET /sessions/<id>` mirror those numbers.
`argo session replay <id> [--model X] [--provider Y]` re-runs the
recorded user prompts against a different target. `argo audit
show --tail` annotates each row with its payload detail
(provider/model, tool name + duration, etc.) and `--session <id>`
prints the full per-session audit chain. `argo resume` and the
TUI both render `tool_use` / `tool_result` content blocks as
readable arrows instead of escaped JSON. `skill verify --all` +
`memory verify --all` give hash-chain CI hooks; `argo health`
is a one-shot meta verifier.

`argo gateway start` is a real signal-clean HTTP daemon listening
on 127.0.0.1:7400 with PID-file lifecycle (start/stop graceful
shutdown via SIGTERM/SIGINT). It exposes **18 endpoints** —
`/health`, `/version`, `GET /sessions` (with per-row spend),
`GET /sessions/<id>` (+ tool_call_count + spend), `GET
/sessions/<id>/messages`, `GET /sessions/<id>/spend`,
`GET /skills`, `GET /memory`, `GET /audit`,
`GET /audit/sessions/<id>`, `GET /costs`,
`GET /costs/by-model`, `GET /costs/by-session`,
`POST /sessions`, `POST /sessions/<id>/messages`,
`POST /sessions/<id>/runs` (accepts `max_tool_rounds`),
`POST /sessions/<id>/runs/stream` (chunked-NDJSON live event
stream), `POST /sessions/<id>/steer` — all wired against the
same `SessionStore` the CLI uses.

Voice MCP has all four cloud TTS backends wired (`backend ∈ {piper,
openai, elevenlabs, minimax}`); OpenAI Whisper transcribe is also
live. The browser MCP has all 14 spec tools wired against raw CDP
(HTTP `/json*` + WebSocket `Runtime.evaluate` / `Page.*` /
`Network.*`) without chromiumoxide.

`argo workspace init` registers a workspace row in the DB so memory
subcommands work straight after init; `argo doctor` + `argo health`
report DB schema + workspace + row counts + cron daemon state +
provider creds. Ten end-to-end integration tests cover workspace+
memory, provider+workspace, cron CRUD, voice + browser MCP stdio
loops, the `--json` schema pins, and the gateway's read+write
surface. The CI matrix went back to green via the 4-PR hotfix lane
(#121–#124) that fixed release.yml's YAML parse, the 500-LOC
budget, the Windows test gating, and the deny advisory ignore-list.

What's left:

- **Heavy native bindings** — whisper-rs / piper-rs for the
  voice MCP, chromiumoxide for stateful browser sessions
  (sticky tab ids, persistent WS, batch commands).
- **§23.3 marketplace** — cross-registry integration test
  (GHCR / Docker Hub / Artifact Registry) against real test
  accounts.
- **Daemon-coupled live commands** — `forge optimize/promote`,
  `judge override`.
- **TUI v2** — sticky composer + live streaming + OSC-52
  clipboard.
- **Channel-based real-time gateway streaming** — the current
  `POST /runs/stream` polls `audit_record` every 100ms; an
  `mpsc::Sender<RunEvent>` wired through `run_turn` would
  shave that latency to zero.

## Status

| Phase | Status | Gate |
|---|---|---|
| 0 — Skeleton | ✅ shipped | `cargo test --workspace` green; CI green on Linux/macOS/Windows |
| 1 — Walking skeleton | ✅ shipped (PR #13) | End-to-end test (user → file_read → answer) green |
| 2 — Provider breadth + cost router | ✅ shipped (PRs #1, #6–#9) | All 4 v1 manifests deserialize; cost router persists non-Fixed decisions |
| 3 — Tools, memory, skills, judge | ✅ shipped (PRs #10–#12) | 4 in-core tools end-to-end; locks; judge three-state |
| 4 — Plugins + MCP | ✅ shipped (PRs #14, #15, #41) | Plugin host + MCP client + MCP server |
| 4.5 — ACP server + voice + browser | ✅ shipped (PRs #23, #57, #58) | ACP multi-harness; voice + browser MCP scaffolds with full schemas |
| 5 — Delegation + sandboxes | ✅ shipped (PRs #16, #43) | `Sandbox` trait + real `DockerSandbox` |
| 6 — Gateway + multi-agent + RBAC | ✅ shipped (PR #17) | `Adapter` trait + multi-tenant routing |
| 6.5 — Workspace marketplace | ✅ shipped (PR #21) | Deterministic tar + OCI seam |
| 7 — Audit + telemetry + replay | ✅ shipped (PRs #18, #19, #22, #38, #39, #45, #56) | Hash chain + ndjson + trace timeline + replay |
| 7.5 — Argo Forge | ✅ shipped (PR #20) | Optimization orchestration with budget caps |

## v0.2 amendments + capability items

| Item | Status | PR |
|---|---|---|
| §22 — cron scheduler core | ✅ shipped | PR #24 |
| §22 — cron daemon (file lock + SIGHUP) | ✅ shipped | PRs #44, #47 |
| A1 — cost-tier router | ✅ shipped | PR #8 |
| A2 — external-judge adjudicator | ✅ shipped | PR #12 |
| A3 — user-locked skills + memory | ✅ shipped | PR #11 |
| A4 — gateway RBAC tiers | ✅ shipped | PR #17 |
| A6 — adaptive provider timeouts (HeartbeatMonitor) | ✅ shipped | PR #32 |
| A8 — `model = "latest"` discovery | ✅ shipped | PR #33 |
| A9 — iteration extension policy | ✅ shipped | PR #31 |
| A11 — ACP server | ✅ shipped | PR #23 |
| A13 — resume on restart | ✅ shipped | PR #27 |
| A14 — mid-run `/steer` | ✅ shipped | PR #25 |
| C — hard $$$ cost ceilings (primitive + ledger) | ✅ shipped | PRs #26, #34 |
| F — Argo Forge | ✅ shipped | PR #20 |
| R — time-travel replay | ✅ shipped | PR #22 |
| M — workspace OCI marketplace | ✅ shipped | PR #21 |
| Voice MCP scaffold (§28.Z, schema only) | ✅ shipped | PR #57 |
| Browser MCP scaffold (§28.AA, 12 tools) | ✅ shipped | PR #58 |

## CLI surface (`argo`) — wired commands

| Subcommand | Status | PR |
|---|---|---|
| `argo doctor` | ✅ | #35 |
| `argo db status / migrate-up / migrate-down` | ✅ | #35 |
| `argo workspace init / push / pull (parse)` | ✅ | #35 |
| `argo audit verify [--session] / show [--tail] / export` | ✅ | #35, #36, #38, #45, #52 |
| `argo cron list / create / delete / trigger / tick / daemon` | ✅ | #35, #42, #44, #47 |
| `argo costs show / cap` | ✅ | #35 |
| `argo session list / show / export / search / replay` | ✅ | #35, #45, #53, #54 |
| `argo session diff <id1> <id2>` (offline structural diff) | ✅ | #80 |
| `argo provider list / validate / show / validate-all` | ✅ | #36, #74, #75 |
| `argo skill list / view / install / lock / unlock` | ✅ | #37, #40 |
| `argo skill verify <name>` (provenance hash check) | ✅ | #77 |
| `argo memory list / view / append / revert / forget / lock / unlock` | ✅ | #37, #40, #46 |
| `argo memory verify <uuid>` (entry hash check) | ✅ | #78 |
| `argo memory edit <uuid>` (`$EDITOR` + supersede) | ✅ | #81 |
| `argo mcp serve` (3 read tools) | ✅ | #41 |
| `argo tui` (ratatui session viewer + messages pane) | ✅ | #48, #55 |
| `argo route inspect <session_id>` | ✅ | #49 |
| `argo plugin list` | ✅ | #50 |
| `argo forge runs list` | ✅ | #50 |
| `argo trace --session <id>` | ✅ | #56 |
| `argo gateway status` + `argo agents list` | ✅ | #59 |
| `argo judge verdict <session_id>` | ✅ | #60 |
| `argo model list` | ✅ | #62 |
| `argo session delete [--force]` | ✅ | #63 |
| `argo db prune --older-than-days N` | ✅ | #64 |
| `argo eval validate <path>` | ✅ | #65 |
| `argo session search --session <id>` | ✅ | #66 |
| `argo workspace status / verify` | ✅ | #70, #76 |
| `argo workspace init` (registers workspace row) | ✅ | #79 |
| `argo db status / prune / vacuum` | ✅ | #69, #64, #71 |
| `argo route bench` (FixedRouter micro-benchmark) | ✅ | #82 |
| `argo doctor` reports DB schema + workspace + row counts | ✅ | #85 |
| `argo session list / show --json` (NDJSON) | ✅ | #86, #89 |
| `argo memory list --json [--file <name>]` | ✅ | #87, #88 |
| `argo skill list / view --json` | ✅ | #87, #90 |
| `argo cron list --json` | ✅ | #90 |
| `argo session diff --json` | ✅ | #92 |
| `argo doctor` reports cron lock + provider creds | ✅ | #94 |
| `argo provider show <id> --json` | ✅ | #95 |
| `argo audit show --json [--tail N]` | ✅ | #99 |
| `argo costs show --json` | ✅ | #100 |
| `argo session show` prints first + last message snippets | ✅ | #101 |
| `argo audit verify --json` | ✅ | #107 |
| `argo memory view --json` | ✅ | #108 |
| `argo skill verify --all` (CI hook) | ✅ | #109 |
| `argo memory verify --all` (CI hook) | ✅ | #111 |
| `argo health` (one-shot meta verifier) | ✅ | #127 |
| `argo gateway start` (real HTTP listener) | ✅ | #131 |
| `argo gateway stop` (SIGTERM via PID file) | ✅ | #132 |
| `argo gateway start` graceful SIGTERM/SIGINT shutdown | ✅ | #133 |
| `gateway` `GET /sessions` | ✅ | #134 |
| `gateway` `GET /skills` + `GET /memory` | ✅ | #135 |
| `gateway` `POST /sessions` | ✅ | #136 |
| `gateway` `POST /sessions/<id>/messages` | ✅ | #138 |
| `gateway` `GET /sessions/<id>` + `.../messages` | ✅ | #139 |
| `argo new --provider <p> --model <m> [--title <t>]` | ✅ | #140 |
| `argo resume <id>` (re-render recorded session) | ✅ | #141 |
| `gateway` `GET /audit` + `GET /costs` | ✅ | #143 |
| `argo chat <id> [--user <text>]` (single-turn live) | ✅ | #144 |
| `argo chat` dispatches by `provider.family` | ✅ | #145 |
| `argo steer <session_id> <text>` | ✅ | #146 |
| `argo chat --stream` (token-by-token) | ✅ | #147 |
| `gateway` `POST /sessions/<id>/runs` (live agent turn) | ✅ | #148 |
| **Tool dispatch in the agent loop** | ⏳ pending | next big chunk |

## MCP servers shipped as standalone bins

| Server | Status | PR |
|---|---|---|
| `argo-mcp-voice` (transcribe / synthesize / voice_chat) | ✅ schema + dispatcher | #57 |
| `argo-mcp-voice` — real OpenAI TTS backend (`backend="openai"`) | ✅ | #93 |
| `argo-mcp-voice` — real OpenAI Whisper transcribe (`backend="openai"`) | ✅ | #96 |
| `argo-mcp-voice` stdio integration test | ✅ | #97 |
| `argo-mcp-voice` — real ElevenLabs TTS (`backend="elevenlabs"`) | ✅ | #129 |
| `argo-mcp-voice` — real MiniMax TTS (`backend="minimax"`) | ✅ | #130 |
| `argo-mcp-browser` (12 tools) | ✅ schema + dispatcher | #58 |
| `argo-mcp-browser` — Chrome :9222 auto-detect (A10) | ✅ | #73 |
| `argo-mcp-browser` — `list_tabs` (HTTP probe of `/json`) | ✅ | #103 |
| `argo-mcp-browser` — `chrome_version` (HTTP probe of `/json/version`) | ✅ | #104 |
| `argo-mcp-browser` stdio integration test | ✅ | #105 |
| `argo-mcp-browser` — `eval` via raw CDP WebSocket | ✅ | #113 |
| `argo-mcp-browser` — `screenshot` (Page.captureScreenshot) | ✅ | #114 |
| `argo-mcp-browser` — `read_page` (Runtime.evaluate) | ✅ | #115 |
| `argo-mcp-browser` — `extract_links` | ✅ | #116 |
| `argo-mcp-browser` — `navigate` + `scroll` | ✅ | #117 |
| `argo-mcp-browser` — `pdf` + `submit` | ✅ | #118 |
| `argo-mcp-browser` — `click` + `fill` | ✅ | #119 |
| `argo-mcp-browser` — `wait_for` + `cookies` (all 14 spec tools wired) | ✅ | #120 |

## Pending integration tier

- **Tool dispatch in the agent loop** — `argo chat` (#144/#145/#147)
  + `POST /sessions/<id>/runs` (#148) drive a single round-trip
  to the provider. Tool calls in the response are parsed (via
  argo-provider extractors) but ignored; the next chunk runs them
  via `argo-tool::ToolRegistry` and feeds results back to the
  provider until no `tool_calls` remain. After that come cost
  accounting + audit emission for every turn.
- **Voice MCP native backend** — whisper-rs (STT) + piper-rs (TTS)
  + opus (OGG/Opus). All four cloud backends already wired (OpenAI
  TTS + Whisper #93/#96; ElevenLabs #129; MiniMax #130).
- **Browser MCP — stateful sessions** — all 14 spec tools are wired
  against raw CDP via tokio-tungstenite (#113–#120). What's still
  pending is the *stateful-session* upgrade: chromiumoxide (or a
  hand-rolled CDP session manager) for sticky tab ids, persistent
  WebSocket connections, and batch commands. Cloud opt-ins
  (Browserbase / BrowserUse) are also still pending.
- **§23.3 marketplace integration test** — `oci-distribution` push
  / pull against GHCR / Docker Hub / Artifact Registry test
  accounts.
- **TUI v2** — sticky composer, live streaming, OSC-52 clipboard.
- **Daemon-coupled live commands** — `argo session replay <id>
  --model X` (live), `argo forge optimize / promote`,
  `argo provider test`, `argo judge override`. (`argo chat` /
  `argo steer` already shipped.)
- **HTTP streaming for `POST /sessions/<id>/runs`** — current
  endpoint always runs non-streaming. Chunked-JSON / SSE follow-up
  will let HTTP clients see token-by-token output the way
  `argo chat --stream` does.
