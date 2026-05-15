# RESUME — fresh-session entry point

A new agent picking up SKYCOT (Argo) work reads this page first.

## What this repo is doing right now

Multi-agent swarm shipping v1.0 of SKYCOT (the operator-facing
product surface on top of the Argo core). Workers claim briefs from
`docs/build-plan/streams/`, ship PRs, and a single conductor process
reviews + merges + replenishes the queue.

**Codex workers active again** since 2026-05-14 (the earlier "Codex offline" note from 2026-05-09 is stale). Both `claude` and `codex` workers are running.

**v1.0 ship gate**: `docs/skycot/demos/v1-mission-mvp.mp4` exists
AND `./scripts/smoke-skycot.sh` passes including BrowserExecutor +
Slack-approval scenarios.

## Read in this order (5 files, ~20 min)

1. **`docs/build-plan/BUILD_STATUS.md` — first ~90 lines.** The live
   ledger header. Operator amendments (queue thresholds, paused
   streams, strategic priority, active directives) are here and
   override every other doc. Memory and template defaults are NOT
   authoritative; the header is.

2. **`docs/build-plan/SWARM_PROTOCOL.md` — HARD RULES section + the
   iteration sequence.** Source of truth for: claim → PR → merge
   loop, hard rules, verification, stop condition, graceful exit.
   Re-read on every iteration.

3. **`docs/build-plan/PROTOCOL_LOOP_PROMPT.md`** (conductor only).
   The 9-step tick prompt the conductor runs every loop iteration.

4. **`docs/build-plan/PRODUCT_ROADMAP.md`** (frontend / UX work).
   Operative plan for the post-MVP product-surface program (Tracks
   A–G). Read before touching any UI / persona / journey row.

5. **Active + queued operator directives**, named in the
   BUILD_STATUS.md header's "Live directives" block:
   - **ACTIVE**: `conductor-prompt-deployment-path-v3.md` (drives
     swarm to "deployable end-to-end" — overrides PROTOCOL_LOOP_PROMPT
     step-8 replenishment until §5 stop conditions met).
   - **QUEUED (succeed v3)**: `conductor-prompt-post-v3-directives.md`
     carries BLOCK 1 (SKILL-CANVAS-v1, Track L, 17 briefs) +
     BLOCK 2 (ENTERPRISE-PARITY-v1, Track M, 19+1 briefs, parallel
     to remaining L). Operator pastes each at its trigger. L0+M0
     design docs are `streams/L0-skill-canvas-track-overview.md`
     + `streams/M0-enterprise-parity-track-overview.md`.

## Your role — pick one

- **Worker** (most agents). Read `CONTRIBUTING_AGENT.md` + the
  worker prompt for your model (`PROTOCOL_WORKER_PROMPT_CLAUDE.md`).
  Pick a TODO brief, claim it via `scripts/claim-with-retry.sh`,
  implement, push a PR, exit gracefully after 2-4 PRs.

- **Conductor** (singleton — exactly one across all machines).
  Run `PROTOCOL_LOOP_PROMPT.md` on a 5-minute loop. Review PRs,
  merge clean ones, flip ledger rows, replenish when queue ≤25.
  **Never write feature code.** Mutation surface: PR review/merge,
  BUILD_STATUS.md edits, brief authoring in `streams/`.

- **Ad-hoc session**. If you weren't invoked as a swarm worker or
  conductor, you can ignore the build-plan layer. CLAUDE.md root
  rules still apply.

## What's NOT operative

The following are historical / superseded. Read only for context:

- `BUILD_PLAN_2WK.md` — original 14-day v1 sprint, completed
  tick-57 era. Retained for ship-gate definition.
- `BUILD_PLAN_V11.md` — backend phased plan; backend tracks
  substantively shipped. Live work is now mission-* briefs in
  `streams/` driven by BUILD_STATUS.md, not phase-based.
- `docs/plan/argo-build-plan.md` (v0.2) — core-platform architecture
  reference. Pre-Phase-A; consult only for original invariants.
- `conductor-advisory-{E0-E1,E2-E5,F-reconciliation,G-J}.md` — four
  advisories, all ACTIONED. Rows are inserted in BUILD_STATUS.md;
  see ledger for live state.

## Ground truth, by question

- "What should I work on?" → next TODO row in BUILD_STATUS.md whose
  brief file passes `scripts/check-brief-freshness.sh`.
- "What's the queue-depth threshold?" → BUILD_STATUS.md header
  (currently ≤25, raised by operator 2026-05-10).
- "What's the v1.0 ship gate?" → demo .mp4 + extended smoke per
  BUILD_STATUS.md header.
- "Is codex active?" → no, offline 2026-05-09. Skip codex side.
- "Is brief X stale?" → run `bash scripts/check-brief-freshness.sh
  <brief-id>` before claiming.
- "Where do I report what I shipped?" → BUILD_STATUS.md row flip +
  `status: <brief-id> in review (#<num>)` commit message.

## When the ledger and a doc disagree

**The ledger wins.** Hard Rule 1 from SWARM_PROTOCOL.md. The
BUILD_STATUS.md header is updated every tick; static docs drift.
If a static doc says one thing and the header says another, trust
the header and (if you are conductor) flag the drift for cleanup.

---

---

## Right-now snapshot (2026-05-16) — codex-audit-driven plan in flight

**fresh session picking up SKYCOT v1.0: READ THIS SECTION END-TO-END before doing anything.** session hand-off from the 2026-05-16 conductor session at ~97% context.

### Where we are: audit-driven plan in execution

The 2026-05-14 Phase A–D audits + 2026-05-15 codex full-architecture audit converged on a remediation plan. **Read the codex audit report first:** `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. 18 findings F-001 through F-018, severity-rated, with phased execution roadmap. Authored by codex against `abceee8f`; validated and amended by Claude.

Operator paste-ready audit prompt (in case re-run is needed): `docs/skycot/audit/CODEX_AUDIT_PROMPT_2026-05-15.md`.

### Operator decisions LOCKED 2026-05-15 (do not re-debate)

1. **Q1 macOS CU for v1 gate:** Both paths stay. F-001 (P0 macOS run-loop) stays the ship gate; E2B `computer_use` remains operational. Demo runs on macOS.
2. **Q2 Marketplace authz:** POST install requires bearer/JWT. GET browse stays anonymous (normal SaaS pattern for prospects). Already SHIPPED via #1721.
3. **Q3 AuditSink::new cadence:** Rip the band-aid — one brief marks `#[cfg(test)]` AND migrates every production caller. **NOTE:** pre-flight found 106 production callers (not codex's ~25 estimate). Brief deferred until F-004 lands; will likely split into 2-3 sub-briefs.
4. **Q4 Migration filename duplicates:** Rename to canonical prefixes (best practice; industry norm for Diesel/sqlx/Alembic/Rails). F-016 LOC bumped from 80 → ~250 to account for fallout in tests/tools that grep by filename.
5. **F-008 15-vs-8 invariant drift:** Update CLAUDE.md to match reality. Keep all 15 in core. No tool-to-MCP migration. F-008 brief is a doc edit (~40 LOC).
6. **F-009 unwired tool advertising:** Don't hide via registry split — WIRE the tools (Memory, DelegateTask, SessionSearch, SendMessage). Build `builtin_tools_with_runtime(deps)` in argo-tool; agent runner uses that.
7. **F-011 type canonicalization:** Re-scope to truly-public cross-crate `pub struct` duplicates only (NOT `pub(super)`/`pub(crate)`). Expected count ~5–15 (not codex's 51). LOC ~120.

### Audit-driven briefs — current state (11 authored, 5 MERGED, 3 CLAIMED, 3 TODO, 7 unauthored)

**Phase 1 — v1 SHIP-BLOCKER:**
| Brief | F-NNN | State | Notes |
|---|---|---|---|
| codex-cu-platform-neutral-loop-trait | F-001a | MERGED #1719 | CuStepRunner trait extract — precursor |
| codex-cu-macos-run-loop-wiring | F-001b | CLAIMED | 🔴 V1 SHIP-BLOCKER. Wire macOS exec through trait |

**Phase 2 — P1 security/multi-tenancy:**
| Brief | F-NNN | State | Notes |
|---|---|---|---|
| codex-marketplace-install-authz | F-002 | MERGED #1721 | POST behind auth, GET anonymous |
| codex-webinbox-tenant-auth | F-003 | MERGED #1720 | WsAuth::Legacy removed entirely |
| codex-toolcontext-tenant | F-004 | TODO | **3 BLOCKs amended** — see lessons below; additive-constructor pattern; ~350 LOC |
| (codex-audit-tenant-plumb) | F-005 | DEFERRED until F-004 lands | 106 production callers — needs re-sizing |
| codex-rbac-workspace-scope | F-006 | CLAIMED | ~120 LOC |
| codex-send-message-channel-rbac | F-007 | CLAIMED | New `Capability::MessageSend` variant; ~220 LOC |

**Phase 3 — P2 drift (first batch authored):**
| Brief | F-NNN | State | Notes |
|---|---|---|---|
| codex-claude-md-tool-count-invariant-update | F-008 | TODO | ~40 LOC doc edit per operator Q3 |
| codex-tool-runtime-wiring | F-009 | TODO | Wire (not hide) — `builtin_tools_with_runtime`; ~280 LOC |

**Unauthored (still to write):**
- F-010 mission-genesis-jd-synthesis (~180 LOC, operator-independent)
- F-011 type-canonicalization-wave-2 (~120 LOC, rescoped)
- F-012 file-size-budget-split (~120 LOC)
- F-013 fn-impl-size-budget (~500 LOC)
- F-014 public-api-rustdoc-wave (~600 LOC, Phase 4 polish)
- F-015 global-state-audit (~300 LOC)
- F-016 migration-raw-filename-rename (~250 LOC per operator Q4 best-practice)
- F-017 sql-ident-helper (~120 LOC)
- F-018 provider-comment-refresh (~5 LOC, trivial)

### Pre-flight lessons (3 codex BLOCKs on F-004; apply going forward)

Authoring a brief WITHOUT careful production-callsite grep produced 3 amendment cycles on F-004. **Apply these rules before authoring any of the 7 unauthored audit briefs:**

1. **Grep for `^pub struct X` (definition) not `pub use X` (re-export).** ToolContext is in `tool.rs:53`, not `lib.rs` (which only re-exports). My initial may-touch listed lib.rs.
2. **Filter test files when listing production callsites:** `rg 'PATTERN' crates --type rust -l | grep -v '_tests.rs\|/tests/'`. Inner-mod test functions don't need brief boundaries.
3. **Additive constructor beats breaking signature** when a function has 15+ test callers. Pattern: keep `Foo::new` (set new field to sentinel + `#[deprecated]`); add `Foo::new_with_X` as production constructor. Avoids touching test files that are already at file-size ceiling.
4. **Check for hidden production callsites in adjacent modules.** F-004 had `dispatch_one` in `turn_helpers.rs` (which shares spend.tenant_id scope) — easy to miss because the obvious file is `dispatch.rs`. Always also grep crate-wide.

### Outstanding question (operator was deciding when context ran out)

"Want me to do a proactive pre-flight sweep on F-006/F-007/F-008/F-009 with the new rules before codex hits them?" — Operator hadn't answered. Recommend the fresh session ask this before resuming. If yes: ~15min cost saves ~30-60min of amendment cycles downstream.

### Queue state at hand-off (2026-05-16)

- MERGED: 478 (475 from 2026-05-15 carryover + 3 audit-driven: #1719/#1720/#1721)
- CLAIMED: 3 (codex-cu-macos-run-loop-wiring [v1 SHIP-BLOCKER], codex-rbac-workspace-scope, codex-send-message-channel-rbac)
- IN_REVIEW: 0
- TODO: 3 (codex-toolcontext-tenant + codex-claude-md-tool-count-invariant-update + codex-tool-runtime-wiring)
- BLOCKED: 14 (all categorical pauses — operator-deferred or cascade)
- DROPPED: 44

**0 open PRs.** No swarm-protocol crises in flight.

### What MERGED 2026-05-15/16 (audit-driven session)

| PR | Brief | F-NNN |
|---|---|---|
| #1719 | codex-cu-platform-neutral-loop-trait | F-001a precursor |
| #1720 | codex-webinbox-tenant-auth | F-003 P1 |
| #1721 | codex-marketplace-install-authz | F-002 P1 |
| (earlier today: #1704–#1718 from Tick 12/13 backlog batch — see commits before audit) | | |

### Carryover from 2026-05-15 session

Tick 12/13 of the 2026-05-15 conductor session authored 28 post-v1 backlog briefs. **14 of those (50%) turned out to target already-shipped surfaces** (codex's strict per-claim check caught them). This drift rate is what motivated the formal codex audit. The audit's 18 findings replace that stale gap analysis with a fresh ground-truth plan.

**Memory of the lesson:** the codebase has dramatically more shipped surface than any gap analysis from before 2026-05-15 credited. Future replenishment passes should grep-verify scope before authoring (CLAUDE.md drift-prevention rule §3).

### For the next conductor: how to resume

1. **First action: pull main + check queue state.** `git pull --ff-only origin main && python3 -c "..."` (the state script in every prior tick output works).
2. **Read the codex audit report** (`docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`) and this hand-off section. ~15 min total.
3. **Ask operator about the proactive pre-flight sweep** before continuing brief authoring.
4. **In-flight CLAIMED briefs will surface as PRs.** Tier-1 review them per the standard pattern; admin-merge on GHA-billing-blocked checks (operator-authorized pattern).
5. **Author the 7 unauthored Phase 3/4 briefs** when codex_todo drops below threshold AND the pre-flight discipline is applied. Don't author all 7 at once — codex's first-claim attempt is feedback; 2-3 per tick lets you correct quickly.
6. **F-005 needs special sizing decision** — 106 callers across ~30 files. Either: (a) one big brief that's bigger than codex's normal sizing, (b) split into AuditSink-deprecation-only + 2-3 migration batches, (c) escalate to operator. Recommend (b).

### When does the codebase actually ship v1?

Per V1_DEFINITION.md: `docs/skycot/demos/v1-mission-mvp.mp4` exists + `scripts/smoke-skycot.sh` passes (incl. BrowserExecutor + Slack-approval). F-001b (the v1 SHIP-BLOCKER) is currently CLAIMED. Once it merges + a few P1s, the demo recording can be done.

## When this file is stale

The conductor refreshes it during hand-off + major state changes. If you read this file and the snapshot dates look weeks old, treat the BUILD_STATUS.md header as authoritative and ignore the snapshot.

Last refreshed: 2026-05-16 (codex-audit-driven plan in execution; operator approaching 97% context; handing over). Cron loop STOPPED. Manual ticks. Queue at hand-off: MERGED 478, TODO 3, CLAIMED 3, IN_REVIEW 0, BLOCKED 14.
