# Round 97 — Process drift sweep

**Date**: 2026-05-27
**Round**: 97
**Pillar**: Process axis (conductor handoff hygiene + runtime freshness + finding deadlines).
**Verdict**: 🟡 **PARTIAL** — 3 gates clean, 1 carry (non-canonical gateway CWD) + 1 ghost-stamp pattern persists.

## Setup

```bash
bash scripts/check-unread-handoffs.sh
bash scripts/check-gateway-freshness.sh
bash scripts/check-findings-only-fix-by.sh
# Plus: ps + lsof to capture gateway PID and CWD; halt-sentinel ls; git log -25.
```

## Observed

| Gate | Result |
|---|---|
| `check-unread-handoffs.sh` | ✅ no unread non-conductor handoffs. ⚠️ 4 ghost-stamped conductor entries detected (timestamp-in-body vs commit-time delta > 5min window) — same pattern flagged in rounds 88 / 91 / 93 / 95 / 96. |
| `check-gateway-freshness.sh` | ✅ `gateway_schema=v83 codebase_latest=v83 drift=0 STATUS: FRESH`. |
| `check-findings-only-fix-by.sh` | ✅ 4 within-budget, 0 due-today, 0 overdue (deadline 2026-05-30). |
| Halt sentinels (`/tmp/argo-stop` + `~/.argo/stop`) | ⚠️ BOTH PRESENT since 2026-05-27T01:34Z (operator-deliberate per session-handoff a6c0be33e). Swarm HALTED. |
| Gateway live | ✅ PID 53182, schema v83, uptime ≈ 1h. |
| Gateway CWD | ⚠️ `~/code/skycot-v1-demo` — non-canonical (operator-directed during T0819MEL handoff window to land the demo-seed against a fresh tenant). |
| Conductor cadence | The most-recent conductor commit on origin/main is `e3fd93e9f` at ~2026-05-27T05:07 +1000 = 2026-05-26T19:07Z; current evaluator wake at 2026-05-27T03:55Z → ~8h since last conductor commit. Swarm-halted explains this; not a drift in the usual sense. |
| Open carries from R96 | R96-listed unauthored: `ops-gateway-freshness-check-also-detects-binary-staleness` (AUTHORED, `e3fd93e9f` 2026-05-27T05:07 commit promoted it — R96 said "AUTHORED ✅", R97 confirms ledger row + brief file both present). Other 5 carries (ops-claim-5 canonical-binary-subcommand, mechanical-batch hygiene sweep, etc.) NOT actioned this window — swarm-halted state explains. |

## Anti-theater check

- **Ghost-stamped conductor entries**: extracted 4 stamps where the `[conductor]` text time differs from the `nearest conductor.json commit` time by more than the 300s window. Example: `GHOST STAMP: 2026-05-27T08:30:00Z [conductor]` paired with `nearest conductor.json commit: 2026-05-27T15:08:12+10:00 (12108s away)`. The script does not block on these — it warns. The pattern is real (>300s clock-skew between in-text timestamp and commit) but does not produce false-negative on the unread-handoffs gate (which uses the script's cutoff logic, not the ghost stamps).
- **Gateway freshness FRESH but binary may be stale**: the gateway is the binary built into `/tmp/argo-shared-target/release/argo`. R96 noted this class of false-negative — handler-only changes don't bump schema, so schema-only freshness gate misses binary staleness. R96 confirmed `ops-gateway-freshness-check-also-detects-binary-staleness` was authored; I confirm it is still authored (in `streams/`) and still TODO. This round adds a sibling observation: the binary AND the CWD can both drift independently of schema. A second gate covering "CWD == repo root" would catch the current non-canonical state where the gateway is bound to a fresh-tenant demo workspace dir.

## Gap / follow-up brief candidates

### 🟡 P1 EXISTING (was R96/R95/R94/R93/R92 carry — STILL UNACTIONED) — `ops-gateway-freshness-check-also-detects-binary-staleness`

Authored in `streams/`, promoted to TODO via commit `e3fd93e9f`. Should be claimed next worker tick — but workers are HALTED. Recommend conductor flag to operator that swarm-relight is the gate for this carry to move.

### 🟡 P2 NEW (extends R96 binary-staleness brief) — `ops-gateway-freshness-check-also-detects-non-canonical-cwd`

**Scope**: extend `check-gateway-freshness.sh` to also emit a line if `lsof -a -p <gateway-pid> -d cwd` does not match `git rev-parse --show-toplevel`. Default: WARN; promote to ERROR via env-flag for CI. Reason: tenant resolution under the current ensure_workspace path is CWD-bound, so a non-canonical CWD means the SPA's anonymous endpoints quietly query a different tenant than the operator expects.

**Out of scope**: do not change the ensure_workspace tenant-resolution rule itself — that is a separate brief class (provider-of-record / tenant pinning via header).

### 🟢 P3 NEW (process hygiene) — `ops-conductor-tick-emit-current-utc-not-self-stamp`

**Scope**: change conductor tick scripts to stamp `date -u +%Y-%m-%dT%H:%M:%SZ` at append-time, not from an in-message string the model produced. Eliminates the ghost-stamp pattern carried across 5+ evaluator rounds. Low-LOC change in whichever shell utility (`scripts/append-shared.sh` or equivalent) the conductor tick uses.

## Operational notes

- Swarm-halted state is operator-deliberate and date-stamped — not a drift. Don't surface as a P0.
- 14 open PRs (per `gh pr list` count via SHARED.md text; not re-run this round). No blocker for the evaluator's read-only scenarios.
- The R97 PR #2437 FAIL finding (separate file) is the load-bearing item for this round; this process-drift report is the second-voice safety net.
