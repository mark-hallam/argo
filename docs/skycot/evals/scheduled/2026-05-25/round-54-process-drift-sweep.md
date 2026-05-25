# Round 54 — Process drift sweep + 7th-consecutive-round merge-deferral

- **Date**: 2026-05-25 (UTC) · Melbourne 2026-05-25 ~10:30 AEST
- **Round**: 54 (scheduled-task hourly fire ~1h20min after r53 ship at 23:10Z)
- **Pillar**: Process / conductor loop drift (axis 4 in evaluator skill)
- **Verdict**: 🟡 **PARTIAL — 7TH-CONSECUTIVE-ROUND merge-deferral with new failure mode (CLAIM CYCLING)**

## Setup

- `bash scripts/check-gateway-freshness.sh` → 🟢 schema=v80=v80, drift=0, FRESH
- `bash scripts/check-findings-only-fix-by.sh` → 4/4 within budget (eval-cron-duty / eval-workflow-execution / mission-hire-audit / mission-cu-macos-cross-tenant — all 5 days remaining)
- `bash scripts/check-unread-handoffs.sh` → 6 unread (5 informational + the r53 evaluator handoff itself)
- `gh pr list --state open --limit 100 --json number,createdAt,mergeable` → **65 open** (net +1 since r53's 64); ALL `MERGEABLE` or `UNKNOWN`; oldest is #2200 from `2026-05-24T10:47Z` = **~14h old**
- `grep -nE "^\| .* \| (codex|claude) \| TODO " docs/build-plan/BUILD_STATUS.md | wc -l` → **6 TODO rows** (queue near-exhausted again after T23-24 replenishment +6; +1 new claim in last 13 min by `fix-web-inherited-red-test-cluster-2241-baseline`)
- `wc -l crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` → **526 LOC** (still > 500 ceiling; LOC-gate still red)

## Observed

### A. The 526-LOC blocker has NOT been fixed and is now CLAIM-CYCLING

Brief `codex-cli-routes-approvals-json-split-helpers` claim history in last 7 hours (from BUILD_STATUS.md ledger row + commits):

| Time (UTC) | Event | Commit / actor |
|---|---|---|
| 2026-05-24T17:13Z | First claim (codex pool, mac-host) | `claim-with-retry` |
| 2026-05-24T17:13Z → 23:35Z | **6h15m of "Waiting for clippy/tests..." in worker log without opening a PR** | worker silent-fail |
| 2026-05-24T23:35Z | DEMOTED CLAIMED→TODO by T23-24 conductor (`b886bd9e2`) | operator-directed via amendment |
| 2026-05-24T23:51Z | Re-claimed (zombie release in between) (`b57b77e6b`) | worker auto-claim-retry |
| 2026-05-25T00:01Z | Released as zombie ("no PR opened") (`54e129f1a`) | worker-loop zombie-detection |
| 2026-05-25T00:10Z | **THIRD claim** (current) (`2706c0fa2`) | worker auto-claim-retry |

**Net: 3 claim cycles, 0 PRs, ~7h elapsed.** This is a distinct pathology from the "merge-deferral" pattern — it's a **claim-cycle livelock**: every fresh worker claims the brief, fails to ship, releases via zombie-detection, and a sibling worker re-claims within minutes. The work never gets done, but the row also never sits in TODO long enough for an operator or conductor to escalate it.

### B. 7TH-CONSECUTIVE-ROUND merge-deferral (now with corrected root cause confirmed)

Last 6 evaluator rounds (r48 through r53) and now r54 have all observed `0 substrate merges since prior round`. Today's confirmation:

```
git log --oneline 8b454f505..origin/main  # r53 baseline → now
2706c0fa2 status: claim codex-cli-routes-approvals-json-split-helpers
54e129f1a status: release codex-cli-routes-approvals-json-split-helpers (zombie; no PR opened)
aeae40bdd status: claim fix-web-verticals-route-cors-preflight-symmetric-fix
be9206ef2 status: T23-24 queue replenishment — 6 new TODO briefs (3 claude + 3 codex)
234ffd9f4 ops: SHARED.md — corrected queue-depth diagnostic (T23-24)
b57b77e6b status: claim codex-cli-routes-approvals-json-split-helpers
b886bd9e2 status: demote codex-cli-routes-approvals-json-split-helpers CLAIMED→TODO (zombie release)
dc36cb726 status: mark fix-routing-decision-tenant-scoping in review
272625c72 evaluator: round 53 — ...
```

**Zero substrate-code commits.** Only status flips + queue ops + the r53 evaluator commit. The root cause is the same 526-LOC file the prior 6 rounds flagged — but **escalation has stalled in the claim-cycle pathology above** rather than reaching operator-interactive resolution.

### C. Open-PR ageing escalation

- **65 PRs open** (+1 since r53)
- **Oldest: #2200** from 14h ago
- **24+ PRs** in the "evaluator PRE-MERGE-PASS'd, waiting for gate" cluster (r48-r53 verifications)
- **Net +35 in 18h** — unsustainable: average ship rate dropped from ~14/tick (T17-33 wave) to **0/tick × 7 ticks**

### D. SPA gateway probes (sanity)

Live gateway at `:7400`, schema v80, uptime 22h:
- `/health` → 200 ✅
- `/api/v1/team` → 200 ✅ (canonical workers-list path; SPA uses this)
- `/api/v1/audit?limit=5` → 200 ✅
- `/verticals` (bare) → 200 ✅ but `/api/v1/verticals` → 404 (already-known r49 finding `gateway-vertical-discovery-sibling-gap-confirm` — CLAIMED at `fix-web-verticals-route-cors-preflight-symmetric-fix` 00:04Z; this is its 5th-consecutive carry-forward round)
- `/api/v1/workers` (GET) → 404 (**expected**: only POST is registered at `routes_onboarding.rs:225`; the GET path uses `/api/v1/team` per `web/lib/api/team.ts:65`; ruled out as a new gap)

### E. CLAIMED-row staleness (potential zombie carriage)

```
CLAIMED rows by claim date (2026-05-20..25):
  10 2026-05-20
  56 2026-05-21
  37 2026-05-22
  55 2026-05-23
  81 2026-05-24   ← 81 CLAIMED rows from yesterday still in ledger
   2 2026-05-25   ← only 2 fresh today (the LOC-split + the red-test cluster, both within last 25 min)
```

**81 carry-forward CLAIMED rows from 2026-05-24** is a flag. Many may be MERGED-stale or zombie-claimed (per the 23+ `RELEASED by worker-loop zombie-detection` markers in the `eval-claim-2-live-duty-cron-fires-runner-test` row alone). A `scripts/find-stale-claim-drift.py` sweep is overdue but is conductor-domain (not evaluator), so flagging-only.

### F. 18TH-consecutive-round AEST→UTC stamp drift (carry-forward)

Conductor heartbeat rows `T22-10`, `T23-24`, `T19-08` (today's T19-08 stamps to 2026-05-25T19:08Z which is in the FUTURE relative to current 00:30Z UTC — same Melbourne 19:08 AEST = 09:08Z drift). The MERGEABLE brief `ops-heartbeat-utc-stamp-lint` (#2249) is sitting in the gated queue.

## Anti-theater check

- All findings cross-checked with two independent measures: ledger grep + `gh pr list` direct API call.
- Worker-log evidence (`~/.argo/worker-logs/20260524T171334Z-codex-cli-routes-approvals-json-split-helpers.ndjson`) was independently surfaced by the T23-24 conductor; my role is corroborate-by-symptom-pattern (3 claim cycles, zero PRs).
- Did NOT touch any code, briefs, or ledger rows — read-only sweep per evaluator charter.

## Gap / follow-up brief candidates

### NEW this round

**(m) 🚨 conductor-claim-cycle-detector** — P0 conductor-process — when a brief has been claimed → released → re-claimed ≥2 times within 6 hours without opening a PR, BLOCK the brief automatically and surface to operator. The current claim-cycle pathology is invisible to the merge-deferral and gateway-freshness checks; needs a dedicated detector. Source script can sit alongside `scripts/find-stale-claim-drift.py`. ~50-100 LOC bash + a SHARED.md handoff template.

### CARRIED FORWARD from r52/r53 (unchanged; 12 → 13 brief candidates total with new (m))

(a) gateway-vertical-discovery-sibling-gap-confirm — **now CLAIMED at fix-web-verticals-route-cors-preflight-symmetric-fix** (00:04Z); carry-forward closed when PR ships
(b) eval-claim-3-live-genesis-to-run cold cargo rerun — **being verified this round** (build still cooking; standalone report coming)
(c) through (l) — unchanged; refer to r52 / r53 reports for full text

## URGENT escalation (unchanged from r53, now compounded)

1. 🔴 **Operator interactive 5-min split of `routes_approvals_json.rs`** — the claim-cycle livelock means worker pool cannot ship this; recipe is identical to #2144 sibling-helpers convention; **this is the gate**.
2. After (1) lands as a PR → admin-merge wave of 24+ PRs blocked behind it.
3. **If LOC split not shipped by r55** → P0 operator escalation for the claim-cycle pathology itself (the brief-claiming infrastructure is masking the problem, not solving it).

## Reports + ledger update

- This file: `docs/skycot/evals/scheduled/2026-05-25/round-54-process-drift-sweep.md`
- Sibling reports: `round-54-pr-2256-delegate-task-fallback-chain-pre-merge-anti-theater-verify.md` (🟢 PASS), `round-54-claim-3-cold-cargo-verify.md` (pending — build in progress)
- LATEST.md: row added
