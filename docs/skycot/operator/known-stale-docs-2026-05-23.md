# Known-stale docs audit — 2026-05-23

**Authored by**: conductor during operator-directed staleness sweep, post-quota-halt.
**Context**: operator concerned that new agents reading old plans may make changes based on incorrect information. This is a triage list — what to trust, what to cross-check, what to ignore.

## Severity legend

- 🔴 **CRITICAL** — would mislead a new agent into wrong action. **FIXED in this commit** (banner added or content updated).
- 🟡 **STALE BUT FLAGGED** — explicit in-doc note now warns about staleness; safe to read if you respect the note.
- 🟢 **FRESH** — recently updated, trust as-is.
- ⚪ **HISTORICAL** — kept for context only; not the operative source.

## Entry-point doc audit (top-level)

| Doc | Last modified | Severity | Status |
|---|---|---|---|
| `CLAUDE.md` (repo root) | 2026-05-24 | 🟡 → 🟢 | **FIXED**: Status block updated to 2026-05-24 reality (6 mission claims with current LIVE state + tick-91 chain-merge wave mention + handoff doc pointer at top). |
| `docs/build-plan/RESUME.md` | 2026-05-23 | 🟡 → 🟢 | **FIXED**: last-sweep date bumped to 2026-05-23; quota-halt warning added at top. |
| `docs/build-plan/BUILD_STATUS.md` | 2026-05-24 09:00 | 🟢 | Live ledger — auto-maintained by conductor every tick. |
| `docs/build-plan/PROTOCOL.md` | 2026-05-23 | 🟢 | Updated 2 days ago, current. |
| `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` | 2026-05-24 | 🟢 | Updated today, current. |
| `docs/build-plan/SWARM_PROTOCOL.md` | 2026-05-15 | 🟡 | 9 days old. Hard rules + iteration sequence still valid per spot-check; minor drift in line numbers possible. |
| `docs/build-plan/CONTRIBUTING_AGENT.md` | 2026-05-09 | 🟡 | 15 days old. Short doc (67 lines). Worker-side rules; spot-check before claiming if you're a worker. |
| `docs/build-plan/MISSION_BACKLOG.md` | 2026-05-19 | 🟡 | 5 days old. 787 lines, 280 backlog items. Items tagged `[NEW]` may have shipped; **always cross-check `git log` before authoring a brief from this list**. |
| `docs/build-plan/PRODUCT_ROADMAP.md` | 2026-05-15 | 🟡 | 9 days old. Tracks A-G. Track A DONE; B QUEUED; C-G operator-decisional. Likely still accurate but not actively edited. |
| `docs/build-plan/BUILD_PLAN_V11.md` | 2026-05-12 | 🟡 | 12 days old. Operative for backend tracks only (product-surface moved to PRODUCT_ROADMAP). |
| `docs/plan/argo-build-plan.md` | 2026-05-14 | ⚪ | **FIXED**: HISTORICAL banner added at top warning agents not to use as operative plan. |
| `docs/skycot/MISSION.md` | 2026-05-24 00:11 | 🟢 | Updated today — operator-locked LIVE criteria + 6-claim definitions. **Source of truth for "what does done look like."** |
| `docs/skycot/DRIFT_PREVENTION.md` | 2026-05-15 | 🟡 | 9 days old. Rules still valid. |
| `docs/skycot/operator/post-reboot-swarm-startup.md` | 2026-05-23 | 🟢 | Updated 2 days ago. Now supplemented by `quota-conservation-handoff-2026-05-23.md` for the halt-scenario. |
| `docs/skycot/operator/quota-conservation-handoff-2026-05-23.md` | 2026-05-23 | 🟢 | NEW — the canonical resume doc post-halt. |

## Other potentially-stale areas (NOT audited in detail — defer to next opportunity)

| Area | Estimated staleness | Audit recommendation |
|---|---|---|
| **`docs/build-plan/streams/*.md`** (1,015 brief files) | High variance | Many are MERGED but file not deleted. **Always cross-check the brief's ledger row in BUILD_STATUS.md before claiming.** The 425 truly stale briefs were bulk-archived to `streams/archive/` in tick-69. |
| `docs/skycot/audit/scheduled/*.md` | Auto-managed by auditor | LATEST.md is the pointer; older dated files are historical reports. Safe to trust per their dates. |
| `docs/skycot/evals/scheduled/*.md` | Auto-managed by evaluator | Same pattern — LATEST.md is current; dated subdirs are round-by-round history. |
| `docs/skycot/ux-research/` | 2026-05-19 era | Persona + journey maps. Likely still valid; design language locked per the design-explorations dir. |
| Memory files at `~/.claude/projects/-Users-operator-code-argo/memory/` | Stable | Auto-surfaced in every Claude Code session; operator-curated; cross-check against current state when in doubt. |
| `crates/argo-*/src/**` | LIVE | This IS the code. Always trust git over docs. |

## What an agent should do when in doubt

1. **Trust git over docs.** `git log` and current `crates/` source is the only true source of behavior.
2. **Trust the heartbeat over older state.** `docs/skycot/agents/heartbeats/conductor.json::last_run_utc` is the truthful "when did we last reason about this." If a doc claims something but the heartbeat is newer than the doc, the doc is suspect.
3. **Cross-check briefs against the ledger.** A brief file existing in `streams/` does NOT mean the work is needed; check `grep "^| <brief-slug>" docs/build-plan/BUILD_STATUS.md` for current status.
4. **Read the handoff doc first if you're a fresh session.** It always reflects current state at the moment of authoring.
5. **Ask the operator** if a doc + git disagree and it matters.

## Followup recommended (not done this commit)

These would benefit from a deeper edit but the operator was at 98% quota when this was authored — defer to a token-rich session:

- **`docs/build-plan/BUILD_PLAN_V11.md`** — full rewrite to reflect tick-91 chain-merge wave + claims 1-6 LIVE status.
- **`docs/build-plan/MISSION_BACKLOG.md`** — sweep all `[NEW]` items and re-tag based on current main (likely 30%+ have shipped).
- **`docs/build-plan/CONTRIBUTING_AGENT.md`** — refresh to reference the new fix-by + unread-handoffs + gateway-freshness scripts that didn't exist 15 days ago.

These are not blockers — the entry-point docs (CLAUDE.md + RESUME.md + handoff) now correctly route a new agent to the authoritative state.

---

**Audit complete**. The 4 critical fixes (CLAUDE.md status, CLAUDE.md staleness-gotchas section, RESUME.md sweep date, argo-build-plan.md historical banner) shipped in the same commit as this doc.
