# Argo Roadmap

The canonical phase plan lives in `docs/plan/argo-build-plan.md` §27.

## Current phase

**Phases 0–7 + 4.5 + 6.5 + 7.5 are all shipped** as a stack of 28
PRs against `main`. The remaining work is the **integration tier**:
the daemon binary that wires the libraries together and the heavy
MCP servers (voice, browser).

## Status

| Phase | Status | Gate |
|---|---|---|
| 0 — Skeleton | ✅ shipped | `cargo test --workspace` green; CI green on Linux/macOS/Windows |
| 1 — Walking skeleton | ✅ shipped (PR #13) | End-to-end test (user → file_read → answer) green against scripted transport |
| 2 — Provider breadth + cost router | ✅ shipped (PRs #1, #6, #7, #8, #9) | All 4 v1 manifests deserialize; cost router persists non-Fixed decisions |
| 3 — Tools, memory, skills, judge | ✅ shipped (PRs #10, #11, #12) | 4 in-core tools end-to-end; memory + skill stores with locks; judge with mock-driven success/failure/partial |
| 4 — Plugins + MCP | ✅ shipped (PRs #14, #15) | Plugin host over JSON-RPC stdio; MCP client (stdio, version 2024-11-05) |
| 4.5 — ACP server | ✅ shipped (PR #23) | ACP wire envelopes + 6-harness multi-harness + permission mapping; voice/browser MCP servers still pending |
| 5 — Delegation + sandboxes | ✅ shipped (PR #16) | `Sandbox` trait + `WorkspaceFork` |
| 6 — Gateway + multi-agent + RBAC | ✅ shipped (PR #17) | `Adapter` trait + multi-tenant routing + 3 stubs |
| 6.5 — Workspace marketplace | ✅ shipped (PR #21) | Deterministic tar + OCI seam (real `oci-distribution` push/pull is the §23.3 follow-up) |
| 7 — Audit + telemetry + replay | ✅ shipped (PRs #18, #19, #22) | Hash-chained audit log; 13 spans + 8 metrics; time-travel replay (verbatim + diff + bisect) |
| 7.5 — Argo Forge | ✅ shipped (PR #20) | Skill-optimization orchestration with budget caps |

## v0.2 Amendments + capability items

| Item | Status | PR |
|---|---|---|
| §22 — cron scheduler core | ✅ shipped | PR #24 |
| A14 — mid-run `/steer` | ✅ shipped | PR #25 |
| C — hard $$$ cost ceilings (per-session/day/month) | ✅ shipped | PR #26 |
| A13 — resume on restart (orphan tool-call detection) | ✅ shipped | PR #27 |
| `argo-eval` — eval harness runtime | ✅ shipped | PR #28 |
| A1 — cost-tier router | ✅ shipped | PR #8 |
| A2 — external-judge adjudicator | ✅ shipped | PR #12 |
| A3 — user-locked skills + memory | ✅ shipped | PR #11 |
| A4 — gateway RBAC tiers | ✅ shipped | PR #17 |
| A11 — ACP server | ✅ shipped | PR #23 |
| F — Argo Forge | ✅ shipped | PR #20 |
| R — time-travel replay | ✅ shipped | PR #22 |
| M — workspace OCI marketplace | ✅ shipped | PR #21 |

## Crate inventory

| Crate | Phase | What's in it |
|---|---|---|
| `argo-types` | 0 | Core IDs, capabilities, budgets, errors, lock + judge + routing types, **cost ceilings** |
| `argo-store` | 0 | SQLite + FTS5 session store, migrations, `persist_routing_decision` |
| `argo-capability` | 0 | RBAC engine, default role presets, dynamic grants |
| `argo-provider` | 2 | Manifest loader + 4 transports (openai-compat / Anthropic / Bedrock / Responses) + 6 tool-call extractors + Hermes XML corpus |
| `argo-routing` | 2 | `Router` trait, `FixedRouter` / `TieredRouter` / `PolicyPluginRouter`, `TierCache`, `PersistingRouter` |
| `argo-tool` | 3 | `Tool` trait + `ToolRegistry`; 4 end-to-end tools (`file_read`, `file_write`, `file_patch`, `terminal`) + 4 schema-only stubs |
| `argo-memory` | 3 | Content-addressed memory entries with `UserProtected` / `ContentLocked` locks |
| `argo-skill` | 3 | YAML-frontmatter skill store with locks; install / supersede / list |
| `argo-judge` | 3 | `Judge` trait + `LlmJudge`, §25.2 default rubric, hash-chained evidence, persistence |
| `argo-core` | 1 | Walking-skeleton `Agent::run_turn`, hash-chained `AuditSink`, **`SteerChannel`**, **resume-on-restart** |
| `argo-plugin` | 4 | Plugin host over JSON-RPC stdio |
| `argo-mcp` | 4 | MCP client (stdio, version 2024-11-05) |
| `argo-sandbox` | 5 | `Sandbox` trait + `WorkspaceFork` |
| `argo-delegate` | 5 | Subagent delegation primitives |
| `argo-gateway` | 6 | `Adapter` trait + multi-tenant routing + 3 stubs |
| `argo-audit` | 7 | Hash-chained log + 17 event kinds |
| `argo-telemetry` | 7 | 13 spans + 8 metrics + sinks |
| `argo-forge` | 7.5 | Skill-optimization orchestration with budget caps |
| `argo-marketplace` | 6.5 | Deterministic workspace tarball + OCI seam |
| `argo-replay` | 7 | Time-travel replay: verbatim + diff + bisect |
| `argo-acp` | 4.5 | ACP wire envelopes + multi-harness + permission mapping |
| `argo-cron` | — | Cron expression parser + tenant-aware scheduler |
| `argo-eval` | — | Eval harness runtime (forge bench / forge optimize input) |

## Pending integration tier

These libraries are ready to wire; the surrounding daemon + binary work is what's left.

- **`argo-cli`** — argument parsing surface, subcommand dispatch, daemon mode (cron tick under fcntl/msvcrt lock, `argo session replay`, `argo workspace push/pull`, `argo forge optimize`, `argo steer <id> "<text>"`, `argo resume`, `argo acp serve`).
- **`argo-tui`** — interactive terminal UI.
- **`argo-config`** — `argo.toml` loader; binds the on-disk `[budget]` / `[loop.on_restart]` blocks to the in-memory primitives.
- **`mcp-servers/voice/`** (§28.Z) — whisper-rs + piper-rs; OGG/Opus.
- **`mcp-servers/browser/`** (§28.AA) — chromiumoxide; 12 tools; ≤6,000 LOC.
- §23.3 marketplace integration test against GHCR / Docker Hub / Artifact Registry.
