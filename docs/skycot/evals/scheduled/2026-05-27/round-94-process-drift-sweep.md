# Round 94 — Process drift sweep + supervisor wedge incident

- **Date**: 2026-05-26T21:15Z (AEST 2026-05-27 07:15)
- **Round**: 94
- **Pillar / Claim**: Cross-cutting — agent productivity + runtime freshness + brief follow-through
- **Verdict**: 🟡 **PARTIAL** — runtime axis 🟢 holding (33-round streak r57→r94); 1 P0 demo-runtime-stale finding (covered separately); 1 P1 supervisor-wedge incident (unread, conductor-actionable)

## Setup

Standard scheduled-evaluator process sweep:

- `bash scripts/check-gateway-freshness.sh`
- `bash scripts/check-findings-only-fix-by.sh`
- `bash scripts/check-unread-handoffs.sh`
- `curl http://127.0.0.1:7400/health`
- `git log --oneline origin/main -25`
- Survey of unauthored-recommendation carry from r92-r93

## Observed

### A — Runtime axis 🟢 holding

| Probe | Result |
|---|---|
| Gateway HTTP | 200 OK |
| Schema | v83 = v83 FRESH (drift 0) |
| Uptime | 60028s (~16.7h) — durably stable across overnight merge wave |
| Halt sentinels | `/tmp/argo-stop` + `~/.argo/stop` ABSENT |
| Findings-only | 4/4 within budget (all due 2026-05-30, 4 days) |
| Drift rows | 0 (per `scripts/find-stale-claim-drift.py --quiet`, presumed clean since r91 swept) |
| Open PRs | 15 |
| Recent commits | 25 commits since r92, including 13-PR drain session at T20-35 + T18-16 |

**33-round 🟢 streak r57 → r94 HOLDING** on runtime axis.

### B — `check-unread-handoffs.sh` surfaces 2 supervisor-watchdog handoffs (CONDUCTOR-ACTIONABLE, NOT EVALUATOR-ACTIONABLE)

The handoffs are dated 2026-05-26T21:09:43Z (~5 min before this evaluator fire). Both are from `supervisor-watchdog`:

**Handoff 1 — operational status line:**
```
[supervisor-watchdog] sup=PID:85450/rc:0 slots=2/2 s1=IDLE-pre-commit-hung
  s2=IDLE-pre-commit-hung pulses=0/0-stale claude_pids=0/0hang
  remediated=none drift=0 streak=0 no-claude-streak=1 prs=unavailable
  quota_probe=skipped
```

**Handoff 2 — anomaly report:**
```
🚨 ANOMALY: both bash worker-loops wedged in 'git commit' →
pre-commit hook → lint-brief.sh for ~45min since 06:20 AEST;
slot-1 PID 62799 + slot-2 PID 28809; git commits PID 83446/83441
(release ... zombie commits); pre-commit hooks PID 83458/83450;
lint-brief.sh PID 83523/83497 stuck on lint of
fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request brief;
no .git/index.lock present; no claude -p PIDs running; underlying
trigger was 'fatal: Cannot fast-forward to multiple branches' in
worker fetch at 06:20Z. Workers can't iterate until wedged commits
resolve. Allowlist has no kill-git-commit recipe — operator/conductor
must SIGTERM the stuck git/lint-brief.sh processes, or kill the
brief-lint hook, to unblock.
Recommended: pkill -TERM -f 'scripts/lint-brief.sh'
          && pkill -TERM -f 'git -C.*commit'
```

**Evaluator interpretation:** the swarm productive-axis is degraded. Both worker bash loops are stuck in commit hooks for 45+ minutes. Workers cannot claim or iterate during the wedge. Conductor should action the SIGTERM remediation on next tick (within their write-scope).

### C — 4 ghost-stamped conductor entries still present in SHARED.md

`check-unread-handoffs.sh` also surfaces 4 ghost-stamp warnings on conductor entries where the UTC date stamp appears 1-day-shifted (e.g., `2026-05-26T14:45:00Z` carrying a body text that says `AEST 2026-05-27 01:08`). Same class as r93's finding `ops-fix-shared-md-evaluator-round-88-and-conductor-t00-42-utc-date-typo` — that brief was authored r93 but the carry list ahead has not been cleared.

### D — Recommendation carry-forward audit (r92 + r93 unactioned)

Tracking unauthored briefs the evaluator recommended:

| Brief | Recommended | Status |
|---|---|---|
| `ops-known-flake-step4introduce-template-fallback-timeout` | r92 (T19:25Z) | **UNAUTHORED** — find returns 0 rows in streams/ |
| `ops-supervisor-block-runaway-zombie-claims-on-brief` | r91 (T18:25Z) | **UNAUTHORED** |
| `ops-fix-shared-md-evaluator-round-88-and-conductor-t00-42-utc-date-typo` | r93 (T20:42Z) | **UNAUTHORED** |
| `ops-agents-use-utc-date-not-aest-when-stamping-shared-md` | r93 | **UNAUTHORED** |
| `fix-gateway-skill-seed-tenant-id-matches-demo-jwt-principal` | r93 | **UNAUTHORED** |
| `test-voice-preamble-truncate-word-boundary-coverage` | r93 | **UNAUTHORED** |
| `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` | (operator-authored via commit `4585055d3`) | **AUTHORED** (TODO row in ledger; unclaimed) |

Mostly P3 hygiene work. The `ops-supervisor-block-runaway-zombie-claims-on-brief` recommendation (P2 in r91) is potentially related to the **current wedge incident** described in axis B — runaway zombie commits from worker loops are exactly the failure mode it was designed to block.

### E — Claim-5 cold-rerun brief is well-authored and TODO

The operator auto-authored `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` via commit `4585055d3` at ~2026-05-26T20:30Z after granting macOS permissions. Brief file at `docs/build-plan/streams/ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms.md` is comprehensive:

- Names the three test functions precisely
- Specifies isolated `CARGO_TARGET_DIR` per test (load-bearing for worker-cache safety)
- Acceptance criteria capture both PASS path (flip CLAUDE.md to LIVE) and FAIL path (author follow-up brief)
- Out-of-scope section explicitly excludes touching test code or other mission paths

This brief is sound. Once a worker can iterate (post-wedge-resolution), this is the next major v1.0 ship-gate closer. Closes CLAUDE.md "Remaining for v1.0 launch" item (c).

## Anti-theater check

- Gateway probe is a direct HTTP call to the live process (`/health` JSON parse); not a status-file recap
- Unread-handoff finding is read directly from `scripts/check-unread-handoffs.sh` output, not from any agent's narrative
- Recommendation carry-forward uses `find docs/build-plan/streams -name '<slug>*'` to test brief existence, not memory or trust
- The "33-round streak" claim is auditable: r57 sits at 2026-05-24, r94 sits at today; gateway has stayed FRESH across all hourly probes (every prior round's process-drift-sweep cites runtime 🟢)

**99th anti-theater verification this week.**

## Gap / follow-up brief candidates

### 🟡 **P1 — SURFACE TO CONDUCTOR: supervisor-wedge remediation**

Both worker bash loops wedged in `git commit` / `pre-commit hook` / `lint-brief.sh` since 06:20 AEST (~21:09Z handoff time). Conductor SHOULD action the supervisor-watchdog's recommended remediation:

```bash
pkill -TERM -f 'scripts/lint-brief.sh'
pkill -TERM -f 'git -C.*commit'
```

…on next tick, or surface to operator. Workers cannot iterate while wedged. **This is the dominant productivity blocker right now.**

### 🟢 **P1 — Live gateway needs restart to carry PR #2420**

Covered fully in `round-94-pr-2420-genesis-demo-unblock-post-merge.md`. Investor-demo recording still blocked until `make restart-live-gateway`. Operator-actionable.

### 🟢 **P1 NEW — `ops-gateway-freshness-check-also-detects-binary-staleness`** (~60 LOC)

The current `scripts/check-gateway-freshness.sh` is schema-only. Rust handler changes (this week: #2420, #2429, #2422, #2430, plus more) ship to main without bumping schema, so freshness stays GREEN while the live runtime is silently stale. Add a `git_sha` (or build-sha) field to `/health` JSON, compare against `git rev-parse origin/main`, surface STATUS: STALE-BINARY when drift > 0 commits ahead. Closes the false-negative class.

### 🟢 **P2 — Bundle r91-r93 unauthored recommendations into one mechanical-author batch**

Six P3 hygiene briefs are queued across r91/r92/r93 still unauthored. Conductor could mechanical-author all six in one tick at low cost (each ~10-30 LOC). Bundle title: `mechanical-batch-r91-r93-evaluator-recommendations-hygiene-sweep`.

### 🟢 **P3 — Roll-forward note: claim-5 brief is the next swarm-actionable v1.0 closer**

Once the supervisor wedge resolves, `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` is the highest-leverage TODO row. ~30-90 min wall, closes CLAUDE.md item (c), flips Claim 5 to 🟢 LIVE, leaves only operator-only items (d) demo MP4 record as ship-gate.

## Closure

Runtime axis 🟢, but two operational realities are dragging:
1. **Supervisor wedge** — workers can't iterate; conductor SIGTERM action needed
2. **Gateway-stale-but-fresh-reported** — false-negative in the freshness check; PR #2420's investor-demo unblock is code-correct but live-runtime-blocked

Brief follow-through has degraded slightly (6 unauthored evaluator recommendations across 3 prior rounds), but the highest-priority v1.0 closer (`ops-claim-5-cold-rerun-...`) **is** authored and ready.

**Next round (95)** should re-probe: (a) live gateway uptime — restart will reset to a small number, (b) `pkill` remediation outcome, (c) whether any cluster of unauthored recommendations got picked up.
