# eval-in-core-tool-inventory-vs-mission

**Owner**: codex.
**Depends on**: none (read-only audit).
**Authored against**: origin/main `03515459c` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-in-core-tool-inventory-vs-mission`.

🟡 **P1 MISSION-CLAIM-INVARIANT — verify the "15 in-core tools" mission promise.** Source: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` (`argo-tool` row).

> "Tool capability declarations. Per MISSION.md 'new tools default to MCP unless they require direct AgentDeps'. **Gap: which tools are in-core vs MCP today? The 15-in-core list per MISSION.md needs verification.**"

The evaluator round-11 already noted: *"15 in-core tools claim VERIFIED — regression test `crates/argo-tool/src/tools/mod.rs::tests::builtin_tools_returns_unique_names` asserts `tools.len()==15` + pins alphabetical name list."* So the count + names are structurally pinned. **What's NOT yet audited**: per-tool capability declarations, per-tool tests, per-tool tenant scoping, and the boundary between in-core vs MCP at the rule level.

## Scope

Pure read-only audit. No production code changes. Output is a single Markdown report keyed by tool, plus an existing-test inventory.

### The 15 in-core tools (per round-11 pin)

clarify, code_execute, delegate_task, file_patch, file_read, file_write, memory, process, send_message, session_search, terminal, todo, vision_analyze, web_extract, web_search.

Confirm this list by reading `crates/argo-tool/src/tools/mod.rs` at the test in `tests::builtin_tools_returns_unique_names`. If it differs, surface the drift.

### Per-tool inventory cells

For each tool, fill these cells:

| Cell | Question | How to answer |
|---|---|---|
| **a. capability declared** | Does the tool declare a capability via `argo_caps::Capability`? Which one? | grep for `Capability::` in the tool's source. |
| **b. tenant-scoped** | Does the tool's dispatch path pass `tenant_id` through to any persistence/external call? | Trace `run(...)` to its first DB or filesystem touch. |
| **c. test exists** | Is there at least one `#[test]` exercising the tool's `run(...)`? | `ls crates/argo-tool/tests/` + grep for the tool name. |
| **d. MCP equivalent** | Is there an MCP-connector crate that does roughly the same thing? (e.g., `web_search` vs an MCP web-search connector.) | grep `crates/argo-mcp-connector-*` for tool name keywords. |
| **e. justification for in-core** | Why does this tool live in core rather than as MCP? Direct AgentDeps? Runtime access? File-system primitive? | Read the file header / module docstring. |
| **verdict** | JUSTIFIED-IN-CORE / SHOULD-MIGRATE-TO-MCP / NEEDS-MCP-PARITY | One line per tool. |

### MISSION.md cross-check

Read MISSION.md and find the line that asserts "15 in-core tools". Cite the exact text. If MISSION.md cites a different count or a different list, surface the drift as a P0 finding.

Cross-check the **invariant** in CLAUDE.md: *"new tools default to MCP unless they require direct AgentDeps or runtime access."* For each in-core tool, the report must state the AgentDeps/runtime-access justification. Any tool that fails this test = SHOULD-MIGRATE-TO-MCP.

### Where the artefacts live

- `NEW: docs/skycot/evals/scheduled/2026-05-23/eval-in-core-tool-inventory-vs-mission.md` — the matrix report. ~200-400 LOC.
- `NEW: crates/argo-tool/tests/in_core_tool_inventory_pin_test.rs` — assert each of the 15 tools has a declared capability + at least one test (the round-11 test already pins the names; this adds the *coverage* pin). ~80-150 LOC.

### Findings escalation

- If any tool has NO declared capability → file `mission-tool-<name>-add-capability-declaration` (P1).
- If any tool has NO test → file `eval-tool-<name>-add-test` (P2).
- If any tool is SHOULD-MIGRATE-TO-MCP → file `mission-tool-<name>-migrate-to-mcp` (P2; operator-decisional, surface but don't author the migration).
- If the MISSION.md "15 tools" claim has drifted (count or names) → file `mission-mission-md-tool-list-correction` (P0; mission claim integrity).

Don't author the follow-up briefs in-line — list them at the bottom of the report.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: docs/skycot/evals/scheduled/2026-05-23/eval-in-core-tool-inventory-vs-mission.md`
- `NEW: crates/argo-tool/tests/in_core_tool_inventory_pin_test.rs`
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — flip `argo-tool` row 🟢 with summary.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-tool/src/**` — strict read-only. This is an inventory audit, NOT a refactor.
- Any `crates/argo-mcp-connector-*` — out of scope.
- MISSION.md — read-only; surface drift, don't edit.
- CLAUDE.md — read-only.

## Acceptance criteria

1. Report file exists at the specified path with all 15 tools classified across all 6 cells.
2. Pin test added (asserts each in-core tool has a declared capability + at least one test).
3. Gap-manifest `argo-tool` row flipped 🟢 with cite line.
4. MISSION.md cross-check stated explicitly with citation.
5. At least one paragraph per SHOULD-MIGRATE-TO-MCP finding.
6. Pre-merge gate green: `bash scripts/conductor-pre-merge-gate.sh`.

## Why this is codex-eligible

- Bounded scope: 15 tools, well-defined cells, mechanical answers.
- Pure static analysis — `rg` + `git grep` patterns suffice.
- No architectural judgment needed for the inventory; the verdicts follow from the data.
- Output is a small report + small test — under 500 LOC total.

## Bounded effort

- Expected ~200-400 LOC markdown + ~80-150 LOC test.
- 15 tools × ~5 minutes mechanical inspection = ~1.5 hours of focused work.
