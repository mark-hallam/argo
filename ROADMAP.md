# Argo Roadmap

The canonical phase plan lives in `docs/plan/argo-build-plan.md` §27.

## Current phase

**Library tier complete; integration tier in progress.** Phases 0–7 +
4.5 + 6.5 + 7.5 are all shipped at the library tier across **50 PRs**.
The CLI binary (`argo`) now drives ~25 subcommands against the
workspace store, and the TUI ships a read-only session viewer. The
remaining work is heavy-binding MCP servers (voice / browser) and
cross-registry marketplace integration.

## Status

| Phase | Status | Gate |
|---|---|---|
| 0 — Skeleton | ✅ shipped | `cargo test --workspace` green; CI green on Linux/macOS/Windows |
| 1 — Walking skeleton | ✅ shipped (PR #13) | End-to-end test (user → file_read → answer) green against scripted transport |
| 2 — Provider breadth + cost router | ✅ shipped (PRs #1, #6–#9) | All 4 v1 manifests deserialize; cost router persists non-Fixed decisions |
| 3 — Tools, memory, skills, judge | ✅ shipped (PRs #10–#12) | 4 in-core tools end-to-end; memory + skill stores with locks; judge with mock-driven success/failure/partial |
| 4 — Plugins + MCP | ✅ shipped (PRs #14, #15, #41) | Plugin host + MCP client + MCP server (`argo mcp serve`) |
| 4.5 — ACP server | ✅ shipped (PR #23) | ACP wire envelopes + 6-harness multi-harness + permission mapping |
| 5 — Delegation + sandboxes | ✅ shipped (PRs #16, #43) | `Sandbox` trait + `WorkspaceFork` + real `DockerSandbox` impl |
| 6 — Gateway + multi-agent + RBAC | ✅ shipped (PR #17) | `Adapter` trait + multi-tenant routing |
| 6.5 — Workspace marketplace | ✅ shipped (PR #21) | Deterministic tar + OCI seam (real registry push lands with §23.3) |
| 7 — Audit + telemetry + replay | ✅ shipped (PRs #18, #19, #22, #38, #39, #45) | Hash-chained audit log + ndjson export + tail; OTLP-ready `TracingTelemetry` sink; time-travel replay |
| 7.5 — Argo Forge | ✅ shipped (PR #20) | Skill-optimization orchestration with budget caps |

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
| C — hard $$$ cost ceilings (primitive) | ✅ shipped | PR #26 |
| C — `spend_ledger` persistence | ✅ shipped | PR #34 |
| F — Argo Forge | ✅ shipped | PR #20 |
| R — time-travel replay | ✅ shipped | PR #22 |
| M — workspace OCI marketplace | ✅ shipped | PR #21 |

## CLI surface (`argo`) — wired commands

| Subcommand | Status | PR |
|---|---|---|
| `argo doctor` | ✅ wired | #35 |
| `argo db status / migrate-up / migrate-down` | ✅ wired | #35 |
| `argo workspace init / push / pull (parse)` | ✅ wired | #35 |
| `argo audit verify / show [--tail N] / export [--out]` | ✅ wired | #35, #36, #38, #45 |
| `argo cron list / create / delete / trigger / tick / daemon` | ✅ wired | #35, #42, #44, #47 |
| `argo costs show / cap` | ✅ wired | #35 |
| `argo session list / show / diff / bisect / replay (stub)` | ✅ wired | #35, #36, #45 |
| `argo provider list / validate` | ✅ wired | #36 |
| `argo skill list / view / install / lock / unlock` | ✅ wired | #37, #40 |
| `argo memory list / view / append / revert / forget / lock / unlock` | ✅ wired | #37, #40, #46 |
| `argo mcp serve` (3 read tools: sessions/list, sessions/get, messages/count) | ✅ wired | #41 |
| `argo tui` (ratatui session viewer) | ✅ wired | #48 |
| `argo route inspect <session_id>` | ✅ wired | #49 |
| `argo plugin list` | ✅ wired | #50 |
| `argo forge runs list` | ✅ wired | #50 |
| `argo new / resume / agents / gateway / chat / steer / trace` | ⏳ deferred | needs daemon binding |

## Pending integration tier

- **`mcp-servers/voice/`** (§28.Z) — whisper-rs + piper-rs; OGG/Opus.
- **`mcp-servers/browser/`** (§28.AA) — chromiumoxide; 12 tools; ≤6,000 LOC.
- **§23.3 marketplace integration test** — `oci-distribution` push/pull against GHCR / Docker Hub / Artifact Registry test accounts.
- **TUI v2** — sticky composer, live streaming, OSC-52 clipboard (the §20 surface beyond the read-only viewer).
- **Daemon-coupled CLI** — `argo gateway start`, `argo session replay <id> --model X` (live), `argo steer <id>`, `argo forge optimize / promote`, `argo provider test`, `argo judge override`.
