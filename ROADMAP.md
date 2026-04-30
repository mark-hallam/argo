# Argo Roadmap

The canonical phase plan lives in `docs/plan/argo-build-plan.md` §27.

## Current phase

**Library tier complete; integration tier mostly shipped.** Phases
0–7 + 4.5 + 6.5 + 7.5 are all shipped at the library tier across
**109+ PRs**. The CLI binary (`argo`) drives ~50 subcommands against
the workspace store; the read-side surface (`session list/show/diff`,
`memory list`, `skill list/view`, `cron list`, `provider show`) all
support `--json` NDJSON output for scripting; the TUI ships a session
viewer with messages preview, `/`-filter and PgUp/PgDn navigation;
voice MCP now has a real OpenAI TTS + Whisper backend behind
`backend = "openai"` (heavy native whisper-rs / piper-rs bindings
still pending); browser MCP scaffolds advertise full schemas + Chrome
:9222 auto-detect; `argo workspace init` registers a workspace row in
the DB so memory subcommands work straight after init; `argo doctor`
reports DB schema + workspace + row counts + cron daemon state +
provider creds; four end-to-end integration tests cover workspace+
memory, provider+workspace, cron CRUD, and the voice MCP stdio loop.
What's left is the heavy native bindings (whisper-rs, piper-rs,
chromiumoxide), cross-registry marketplace integration, and the
daemon-coupled commands that need a live gateway / router / judge.

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
| `argo new / resume / chat / steer (live)` | ⏳ deferred | needs daemon binding |

## MCP servers shipped as standalone bins

| Server | Status | PR |
|---|---|---|
| `argo-mcp-voice` (transcribe / synthesize / voice_chat) | ✅ schema + dispatcher | #57 |
| `argo-mcp-voice` — real OpenAI TTS backend (`backend="openai"`) | ✅ | #93 |
| `argo-mcp-voice` — real OpenAI Whisper transcribe (`backend="openai"`) | ✅ | #96 |
| `argo-mcp-voice` stdio integration test | ✅ | #97 |
| `argo-mcp-browser` (12 tools) | ✅ schema + dispatcher | #58 |
| `argo-mcp-browser` — Chrome :9222 auto-detect (A10) | ✅ | #73 |
| `argo-mcp-browser` — `list_tabs` (HTTP probe of `/json`) | ✅ | #103 |
| `argo-mcp-browser` — `chrome_version` (HTTP probe of `/json/version`) | ✅ | #104 |
| `argo-mcp-browser` stdio integration test | ✅ | #105 |

## Pending integration tier

- **Voice MCP native backend** — whisper-rs (STT) + piper-rs (TTS)
  + opus (OGG/Opus). OpenAI cloud TTS + Whisper transcribe already
  shipped (#93, #96); ElevenLabs / MiniMax cloud paths still
  pending.
- **Browser MCP backend** — chromiumoxide / local Playwright;
  cloud (Browserbase / BrowserUse) opt-in. Pure-HTTP probes
  (`list_tabs`, `chrome_version`) are wired (#103, #104); the
  CDP-over-WebSocket driver for the 12 spec tools is the
  remaining lift.
- **§23.3 marketplace integration test** — `oci-distribution` push
  / pull against GHCR / Docker Hub / Artifact Registry test accounts.
- **TUI v2** — sticky composer, live streaming, OSC-52 clipboard.
- **Daemon-coupled CLI** — `argo gateway start`, `argo session
  replay <id> --model X` (live), `argo steer <id>`, `argo forge
  optimize / promote`, `argo provider test`, `argo judge override`.
