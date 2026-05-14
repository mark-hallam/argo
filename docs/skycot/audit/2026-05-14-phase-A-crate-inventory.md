# SKYCOT Phase A Codebase Audit — Crate Inventory + Mission-Claim Mapping

**Authored**: 2026-05-14 by conductor via Explore agent (a5fe216a06015f3d2).
**Scope**: Full enumeration of `crates/` mapped against the 6 load-bearing mission claims in `docs/skycot/V1_DEFINITION.md` §2.
**HEAD**: `29b798be`.

## Executive summary

- **88 crates total**, ~520,814 LOC of Rust. 81 active, 7 stubs (0 LOC).
- **Critical risks**: 2 single-points-of-failure (Claim 1, Claim 3), 1 drift cluster (Claim 4), 11+ duplicate type definitions.
- **Strong coverage**: Claim 2 (5 crates), Claim 6 (3 crates + 8 transports), Claim 5 (40+ crates — possibly over-covered).
- **Immediate actions**: delete 7 stub connectors (apollo, mailgun, resend, signal, telegram, zoom, zoominfo), resolve top-2 duplicate types (Decision, AuditRow).

## Section 1 — Crate census + LOC distribution

**Total**: 88 crates, ~520,814 LOC. Avg 5,918, median 1,050.

### Top 10 by LOC

| Crate | LOC | Category | Claim |
|---|---|---|---|
| argo-cli | 199,382 | CLI binary | TOOLING |
| argo-store | 34,642 | SQLite store | SUBSTRATE |
| argo-skill | 14,484 | Skill library | 4 |
| argo-provider | 10,198 | Manifest layer | SUBSTRATE |
| argo-tool | 10,007 | Tool registry | 5 |
| argo-channels | 9,317 | Approval transport | 6 |
| argo-genesis | 7,939 | Vertical generator | 3 |
| argo-identity | 7,828 | Identity primitives | 1, 2 |
| argo-orchestrator | 7,685 | Multi-worker orchestration | 2 |
| argo-tui | 7,620 | Interactive TUI | TOOLING |

### Bottom 10 (7 stub connectors)

| Crate | LOC | Status |
|---|---|---|
| argo-mcp-connector-slack | 445 | active |
| argo-mcp-connector-notion | 384 | active |
| argo-bluebubbles-client | 383 | active |
| argo-mcp-connector-apollo | 0 | STUB |
| argo-mcp-connector-mailgun | 0 | STUB |
| argo-mcp-connector-resend | 0 | STUB |
| argo-mcp-connector-signal | 0 | STUB |
| argo-mcp-connector-telegram | 0 | STUB |
| argo-mcp-connector-zoom | 0 | STUB |
| argo-mcp-connector-zoominfo | 0 | STUB |

## Section 2 — Mission-claim mapping

### Claim 1 — Teammate, not tool
**Serving**: argo-identity (7,828), argo-capability (500), argo-memory (3,453).
**Status**: ⚠️ SPOF — only argo-identity directly serves. Hardening: dual-code-review + 95% coverage.

### Claim 2 — Delegation is the unit
**Serving**: argo-identity, argo-orchestrator (7,685), argo-workflow (1,937), argo-runtime (6,147), argo-cron (1,197).
**Status**: ✓ Well-covered (5 crates). `argo-delegate` was removed by `claude-argo-delegate-triage` because the live delegation path is `argo-tool` → `argo-orchestrator`, and the crate had zero workspace consumers.

### Claim 3 — Platform builds workers
**Serving**: argo-genesis (7,939), argo-vertical (5,736), argo-judge (1,543).
**Status**: ⚠️ SPOF — Genesis + vertical tightly coupled.

### Claim 4 — Skills shared and learned
**Serving**: argo-skill (14,484), argo-forge (1,952), argo-marketplace (2,388), argo-eval (1,595), argo-vertical-regression (618).
**Status**: ⚠️ DRIFT — learning loop fragmented across 5 crates, no clear orchestrator. Per-worker skill learning queued.

### Claim 5 — Real-world execution
**Serving**: 40+ crates.
- Executors: computer-use (macOS 1,945, v1 primary; E2B 6,793, DEFERRED-v1.x cloud sandbox path; Linux 1,636, DEFERRED-v1.x server deploy path; Windows 2,428, DEFERRED-v1.x enterprise endpoint path), document (1,648), filesystem (1,076), sandbox (581).
- MCP core: argo-mcp (2,059), argo-tool (10,007).
- 30 live connectors + 7 stubs.

**Status**: ✓ Over-covered. Operator decision 2026-05-14 resolves v1 ownership: macOS is primary, while E2B/Linux/Windows remain shipped but deferred to v1.x.

### Claim 6 — Multi-channel approval gates
**Serving**: argo-channels (9,317, hosts 8 transports), argo-voice-tui (1,493), argo-bluebubbles-client (383).
**Status**: ✓ Well-covered (8 transport diversity proven).

## Section 3 — Orphan crates (zero claims)

7 zero-LOC stub connectors. Recommendations:

| Crate | Action |
|---|---|
| argo-mcp-connector-apollo | DELETE |
| argo-mcp-connector-mailgun | DELETE (overlaps SendGrid) |
| argo-mcp-connector-resend | DELETE |
| argo-mcp-connector-signal | DELETE (iMessage + SMS cover) |
| argo-mcp-connector-telegram | DEFER |
| argo-mcp-connector-zoom | DEFER |
| argo-mcp-connector-zoominfo | DELETE |

## Section 4 — Claim coverage summary

**SPOFs**: Claim 1 (argo-identity alone), Claim 3 (argo-genesis alone).
**Drift candidates**: Claim 4 (4+ overlapping crates), Claim 5 (40+ — over-coverage).
**Well-covered**: Claims 2 and 6.

## Section 5 — Duplicate type definitions

11+ duplicates found:

| Type | Crates | Severity |
|---|---|---|
| Decision | argo-cli, argo-identity | HIGH |
| ApprovalDecision | argo-channels, argo-store | HIGH |
| AuditRow | argo-cli, argo-executor-computer-use, argo-skill, argo-store | HIGH (4x) |
| JudgeVerdict | argo-genesis, argo-types, argo-vertical | MEDIUM |
| Role | argo-identity, argo-types, argo-vertical | MEDIUM |
| Outcome | 2+ crates | MEDIUM |
| RestartDecision | 2+ crates | LOW |

**Resolved since audit-3 (2026-05-13)**: SessionId, ExpectedOutcome.
**Root cause**: argo-types under-enforced.

## Section 6 — Top 5 surprises

1. **argo-cli is 199K LOC** (38% of codebase). TOOLING by mission ("v0 placeholder") but 3.4× larger than next crate. Recommend decompose: argo-gateway + argo-cli-repl.
2. **4 computer-use execution paths** (macOS primary for v1; E2B/Linux/Windows DEFERRED-v1.x). Ownership clarity is now explicit for launch.
3. **MCP connector explosion** (37 connectors, 22% of all crates). Maintenance scales O(n). Consider macro-driven template.
4. **Skill optimization loop fragmented** across skill + forge + marketplace + eval (~20K LOC) with no clear orchestrator.
5. **Type fragmentation** — argo-types exists but is under-enforced; 11+ duplicate definitions surfaced.

## Phase A → Phase B handoff

Phase A complete. Phase B candidates (per-crate deep dive on top 20 by LOC) ready to dispatch. Phase E will author briefs for:

- `claude-delete-stub-mcp-connectors` (delete 4-5 orphans)
- `claude-canonicalize-types-to-argo-types` (resolve 11+ duplicates)
- `claude-skill-optimizer-orchestrator` (close Claim 4 drift)
- `claude-argo-cli-decompose-gateway` (cut the 199K monolith) — operator-decisional given scope
- `claude-computer-use-os-native-status-decision` — operator-decisional (deprecate or invest)
