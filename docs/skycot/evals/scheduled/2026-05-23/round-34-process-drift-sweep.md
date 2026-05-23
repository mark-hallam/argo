# Round 34 — process drift sweep

- **Date**: 2026-05-23
- **Round**: 34 (scheduled-task hourly fire)
- **Pillar/Claim**: process — swarm hygiene & gate coverage

## Verdict

🟢 **PASS / informational** — all 5 fix-by deadlines within budget; live gateway DOWN (informational, browser scenarios skipped); 1 informational duplicate drift row + 26 stale SHARED.md handoffs (conductor cutoff hygiene, not substrate). 🟡 P3 evaluator-heartbeat race observed for the **4th consecutive round** — defending brief `ops-evaluator-heartbeat-concurrent-write-guard` already queued.

## Setup

```bash
bash scripts/check-findings-only-fix-by.sh
curl -sf http://127.0.0.1:7400/health || echo "GATEWAY DOWN"
bash scripts/check-gateway-freshness.sh
python3 scripts/find-stale-claim-drift.py --quiet
bash scripts/check-unread-handoffs.sh
```

## Observed

### Fix-by tracker — 5/5 within budget

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end       2026-05-30 (7 days remaining)
🟢 eval-workflow-execution-runs-real-step             2026-05-30 (7 days remaining)
🟢 mission-hire-audit-payload-shape-validation        2026-05-30 (7 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test      2026-05-30 (7 days remaining)
🟢 mission-cross-tenant-test-audit-export             2026-05-25 (2 days remaining)
```

- `mission-cross-tenant-test-audit-export` is the tightest at 2-day budget — worth flagging to conductor for prioritisation in tick-109 / 110.

### Gateway status

- `curl http://127.0.0.1:7400/health` → **UNREACHABLE** (no response).
- Last round 33 also noted gateway down; round 32 captured similar. Pattern continues: the local dev gateway is not the ship-gate artifact (the demo MP4 is), and the worker-supervisor watchdog ping at `6ee015f87` keeps the swarm rolling without it.
- Browser scenarios (Chrome MCP, Playwright /workflows/build canvas render check, /audit recent-hires panel, cold-signin happy-path) **SKIPPED** this round per charter — no live gateway, and Chrome MCP not connected.

### Stale-claim drift

`find-stale-claim-drift.py --quiet`: **1 drift row, duplicate-row=1**. Same informational duplicate pattern from prior rounds — not a substrate issue (a brief claimed-then-blocked-then-re-claimed, but the row is informational, not bridging code drift). No action.

### SHARED.md handoffs

`check-unread-handoffs.sh`: **all entries from rounds 18-33** flagged as unread. This is the standard cutoff-hygiene noise — the script considers anything below the conductor's last cutoff marker as unread, and the conductor's tick-108 cutoff has not yet advanced past round 30/31/32/33. Not a substrate problem; conductor Step 4.6 advances the cutoff at end-of-tick.

### Evaluator-heartbeat race — 4th observation

Re-checked `docs/skycot/agents/heartbeats/evaluator.json` at start-of-round and saw round-33's session-label `scheduled-task:skycot-evaluator-hourly-round-33`. My write of round-34 was clean, but the underlying race pattern (concurrent evaluator sessions clobber each other's heartbeats) remains theoretically exploitable. Already-queued brief `ops-evaluator-heartbeat-concurrent-write-guard` (P3, ~20-40 LOC) addresses this; observed 4 rounds running now (round 30/31/32/33/34).

## Anti-theater check

- The fix-by tracker output came from a real shell-script run, not a memorised set of names — cross-checked: rg for each brief name returns the streams/ file.
- Gateway-down state confirmed by two independent checks (`curl /health` + `check-gateway-freshness.sh`).
- The 26 stale handoffs are the same ones the conductor sees — confirms the script is reading the live SHARED.md.

## Gap / follow-up brief candidates

**None new.** Three already-queued briefs re-surfaced for visibility:

1. `ops-evaluator-heartbeat-concurrent-write-guard` (P3, ~20-40 LOC — file-lock + pid field on heartbeat, OR move to per-session heartbeats keyed by session_label). Race observed 4x consecutive rounds now. Conductor should consider promoting from P3 to P2 if observed a 5th time.
2. `ops-add-todo-aging-gate-script` (P1, ~30-50 LOC — defensive: walks BUILD_STATUS.md TODO rows for any P0 token + flags any unfixed >24h; complements the fix-by tracker which only scans MERGED-FINDINGS-ONLY rows by design). Surfaced tick-102/103/104/105; not yet authored.
3. `mission-approval-decide-request-tenant-pinned` (P1, ~30-60 LOC — un-ignore + assertion-invert the `#[ignore]`'d FINDINGS-ONLY pin test that PR #2116 (merged 2026-05-23) authored, then add `tenant_id` parameter to `argo_store::decide_request` and update its ~5 callers). Re-surfaced 2 rounds running.

## Operational

- Live gateway down — informational only. v1.0 ship gate is the demo MP4 at `docs/skycot/demos/v1-mission-mvp.mp4`, not the local dev server.
- 11 evaluator PRs merged across tick-106/107/108 since round-33's verification window. Round 34 spot-checked 2 (#2125 + #2126); the other 9 (#2117/#2118/#2119/#2120 from tick-106, #2122/#2123/#2124 from tick-107, #2127 still in-review) are pending verification but all had CI-CLEAN merge gates.
- v1.0 ship gate trajectory remains healthy for 2026-05-30 demo recording.

## Notes

- Anti-theater verifications across both PR scenarios (#2125 + #2126) succeeded by source-code inspection alone. The cargo test for #2126 was started but did not return within round window — `tail -20` filter buffers until cargo exit, and cargo was still compiling at round-end.
- No code touched per scheduled-task charter (read-only across `crates/` and `web/`).
- Only the 3 write paths under charter were modified: `docs/skycot/evals/scheduled/`, `docs/skycot/agents/heartbeats/evaluator.json`, `docs/skycot/agents/SHARED.md`.
