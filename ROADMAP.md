# Argo Roadmap

The canonical phase plan lives in `docs/plan/argo-build-plan.md` §27.

## Current phase

**Phase 4 — Plugins + MCP** is the next territory. Phases 0–3 plus the
§6 Phase-1 walking skeleton are complete (subject to merging the
stacked PRs against `main`).

## Status

| Phase | Status | Gate |
|---|---|---|
| 0 — Skeleton | ✅ shipped | `cargo test --workspace` green; CI green on Linux/macOS/Windows |
| 1 — Walking skeleton | ✅ shipped (PR #13) | End-to-end test (user → file_read → answer) green against scripted transport |
| 2 — Provider breadth + cost router | ✅ shipped (PRs #1, #6, #7, #8, #9) | All 4 v1 manifests deserialize; cost router persists non-Fixed decisions |
| 3 — Tools, memory, skills, judge | ✅ shipped (PRs #10, #11, #12) | 4 in-core tools end-to-end; memory + skill stores with locks; judge with mock-driven success/failure/partial |
| 4 — Plugins + MCP | ⏳ next | Plugin chaos test green; off-the-shelf MCP servers work |
| 4.5 — ACP + bundled MCP servers | ⏳ | VS Code/Zed/JetBrains drive Argo via ACP |
| 5 — Delegation + sandboxes | ⏳ | Budget tree property tests; sibling-merge tests |
| 6 — Gateway + multi-agent + RBAC | ⏳ | 24-hour soak; ≥ 1,000 concurrent sessions |
| 6.5 — Workspace marketplace | ⏳ | Push → fresh machine pull → byte-identical |
| 7 — Polish + replay + perf + docs | ⏳ | All NFRs green |
| 7.5 — Argo Forge | ⏳ | Optimize ref skill ≥ 70% improvement, ≤ 8 min, ≤ $5 |

## Phase 1–3 deliverables (cross-reference)

| Crate | Phase | What's in it |
|---|---|---|
| `argo-types` | 0 | Core IDs, capabilities, budgets, errors, lock + judge + routing types |
| `argo-store` | 0 | SQLite + FTS5 session store, migrations, `persist_routing_decision` |
| `argo-capability` | 0 | RBAC engine, default role presets, dynamic grants |
| `argo-provider` | 2 | Manifest loader + 4 transports (openai-compat / Anthropic / Bedrock / Responses) + 6 tool-call extractors + Hermes XML corpus |
| `argo-routing` | 2 | `Router` trait, `FixedRouter` / `TieredRouter` / `PolicyPluginRouter`, `TierCache`, `PersistingRouter` |
| `argo-tool` | 3 | `Tool` trait + `ToolRegistry`; 4 end-to-end tools (`file_read`, `file_write`, `file_patch`, `terminal`) + 4 schema-only stubs |
| `argo-memory` | 3 | Content-addressed memory entries with `UserProtected` / `ContentLocked` locks |
| `argo-skill` | 3 | YAML-frontmatter skill store with locks; install / supersede / list |
| `argo-judge` | 3 | `Judge` trait + `LlmJudge`, §25.2 default rubric, hash-chained evidence, persistence |
| `argo-core` | 1 | Walking-skeleton `Agent::run_turn`, hash-chained `AuditSink` |

## Next gate

Phase 4 — Agent I (plugin host, §11) and Agent J (MCP integration, §12).
Acceptance: plugin chaos test green; off-the-shelf MCP servers work
without recompile.
