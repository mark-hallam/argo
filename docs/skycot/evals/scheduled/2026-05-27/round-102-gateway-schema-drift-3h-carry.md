# Round 102 — Gateway schema-drift 3h carry

**Date**: 2026-05-27 (UTC tick 13:29).
**Round**: 102.
**Pillar/Claim**: Process drift — operational hygiene (gateway freshness vs codebase migration cadence).
**Verdict**: 🟡 PARTIAL — within FRESH budget but a 3h+ standing recommendation has not been actioned.

## Setup

```bash
curl -s http://127.0.0.1:7400/health
bash scripts/check-gateway-freshness.sh
grep -E "EXPECTED_SCHEMA_VERSION" crates/argo-store/src/lib.rs
```

## Observed

```text
$ curl -s http://127.0.0.1:7400/health
{"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":32690}

$ grep -E "EXPECTED_SCHEMA_VERSION" crates/argo-store/src/lib.rs
pub const EXPECTED_SCHEMA_VERSION: u32 = 84;

$ bash scripts/check-gateway-freshness.sh | tail -3
gateway_schema=v83 codebase_latest=v84 drift=1
STATUS: FRESH  (drift <= 2)
```

- Gateway uptime: **32690 seconds ≈ 9h 5min** — no restart since well before the v84 merge.
- v84 merged at `88bab8ae9` on 2026-05-27T00:11+10 (per R101 report `pr-2495-v84-substrate-carry-closure.md`).
- R100 (2026-05-27T09:32Z) wrote: `Post-merge make restart-live-gateway required (v83→v84 drift expected ~1 tick)`.
- R101 (2026-05-27T10:45Z) wrote: `Runtime: gateway v83 vs codebase v84 = drift 1 = FRESH; recommend make restart-live-gateway next conductor tick to flip runtime to v84.`
- This round (T13:29Z): drift still 1, gateway still v83 — **2h 44min after R101's recommendation**.

## Anti-theater check

This is a 🟡 PARTIAL finding (not a 🟢 PASS or 🔴 FAIL) — anti-theater is about confirming the partial state is real:

1. Gateway uptime `32690s` (~9h) is independently observable via the `/health` endpoint, not a value the swarm can fabricate.
2. `EXPECTED_SCHEMA_VERSION = 84` in `crates/argo-store/src/lib.rs:364` is the canonical source of truth for the codebase schema — confirmed in R101 (PR #2495 merge POST-MERGE close-out).
3. `check-gateway-freshness.sh` reads both numbers and emits `STATUS: FRESH (drift <= 2)`. The drift budget is 2; current drift is 1; so the script correctly reports FRESH. No false alarm.
4. The script's `FRESH` verdict is the correct technical state — the storm of activity in the SHARED.md handoff thread between R101 and this round is not gateway-restart work (it's the supervisor-watchdog informational ticks + the conductor's web-only batch merges per memory `feedback_web_only_merge_authorization_2026_05_26`). The conductor has not been negligent — just hasn't prioritised the restart, presumably because the `FRESH` verdict doesn't surface as an action item.

## Why this is 🟡 not 🟢

The numbers say FRESH so by the gate's own definition, no action is required. The 🟡 verdict is a **process** concern, not a substrate concern:

1. R100 and R101 both surfaced the restart-recommendation. If two consecutive evaluator rounds make the same recommendation and the next 3 hours pass with no action, that's a small but real signal that the handoff loop has a leak. The recommendation has no enforcement bite — it sits in the handoff line but doesn't appear in `check-unread-handoffs.sh` output once the round is marked read.
2. The longer the gateway runs an older schema, the larger the eventual blast radius if the gateway is asked to serve a write that requires v84 semantics (e.g. the per-tenant `UNIQUE(name, version, tenant_id)` skill-install path that R101 verified). The current drift=1 is within budget, but the budget is `<=2`, and the next migration that lands (v85 — not in flight at time of writing, but inevitable on the swarm's typical 1-2 day migration cadence) would push the drift to 2, then any further drift would breach.
3. The gateway restart is cheap (~30 seconds on warm `~/.cache/argo-shared-target` per the supervisor memory) and reversible — there is no good reason to defer it.

## Gap / follow-up brief candidates

**None authored this round.** This is operational hygiene, not a substrate defect that needs a brief.

Recommendations to the next conductor tick (handoff line in SHARED.md):

1. Run `make restart-live-gateway` (or its operator-facing equivalent) once. Verify with `curl -s http://127.0.0.1:7400/health | grep schema_version` returns `"schema_version":84`. Note the new uptime is small (<60s).
2. If `make restart-live-gateway` is wedged or the binary won't rebuild, surface as a real defect in the next conductor handoff. The current "informational" framing has carried 2.5 rounds; one more round of carry without action and I'll promote to P2 with an explicit brief.

If the restart does not happen by R103, I will author `ops-conductor-tick-action-stale-gateway-restart-recommendation` as a P2 ~30 LOC bash extension to `scripts/check-gateway-freshness.sh` that distinguishes `FRESH but recommendation pending` from `FRESH and no action needed`, surfacing a third status `FRESH (ACTION-PENDING)` once the drift has been ≥1 for ≥1h. That keeps the recommendation visible to the conductor's `check-unread-handoffs.sh` sweep instead of silently sitting in the prior round's handoff line.

## Verdict

🟡 PARTIAL — within FRESH budget (drift=1, ceiling 2) but standing 2.5h restart recommendation from R100/R101 has not been actioned. No brief authored — operator/conductor manual action (or one more round of carry before I promote to a brief). 110th anti-theater verification this week (counting #108 + #109 from prior reports and this carry-check as #110).
