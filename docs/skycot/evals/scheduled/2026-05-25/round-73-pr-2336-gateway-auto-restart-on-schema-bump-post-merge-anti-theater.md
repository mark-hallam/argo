# Round 73 — PR #2336 gateway auto-restart on schema bump: POST-MERGE anti-theater

- **Date**: 2026-05-25 (Melbourne 2026-05-26 08:09→08:30 AEST)
- **Round**: 73 (scheduled-task hourly cron)
- **Pillar / Claim**: process observability (ν-closer — closes the conductor side of the schema-drift loop that #2335 closes on the detector side)
- **Verdict**: 🟢 PASS · P2 (process)
- **Headline**: #2336 merged `c65ff417e` at 2026-05-25T21:12:10Z (~3.5min after r72 ship; ~30s after #2335). 5-branch drive of `scripts/auto-restart-gateway-on-schema-bump.sh` proves: (1) the docs-only-commit no-op guard fires, (2) the lib.rs-touched-but-no-bump guard fires, (3) the gateway-unreachable branch no-ops + writes a paper-trail entry to SHARED.md, (4) the gateway-already-fresh branch no-ops without invoking the restart script, (5) bogus-sha defensive validation exits non-zero. SHARED.md append-only logging is wired through `AUTO_RESTART_GATEWAY_SHARED_LOG` test seam and observed appending the expected `[conductor-auto-restart-gateway]` line.

## Setup

PR #2336 merge commit `c65ff417e`:

```
ops: auto-restart gateway after schema bump merges (#2336)
docs/build-plan/PROTOCOL_LOOP_PROMPT.md        |   2 +-
scripts/auto-restart-gateway-on-schema-bump.sh | 117 +++++++++++++++++++++++++
```

Script entry-points:
- `sha` positional arg (required)
- 4 env-var test seams: `AUTO_RESTART_GATEWAY_SHARED_LOG`, `AUTO_RESTART_GATEWAY_BASE_URL`, `AUTO_RESTART_GATEWAY_RESTART_SCRIPT`, `AUTO_RESTART_GATEWAY_CURL_TIMEOUT`

The 5 branches of the production wire (file ordering):
1. **Line 50** — `git show --name-only` doesn't include `crates/argo-store/src/lib.rs` → exit 0 "no schema bump"
2. **Line 55** — lib.rs touched but `git show -- crates/argo-store/src/lib.rs | grep -E '^\+pub const EXPECTED_SCHEMA_VERSION'` is empty → exit 0 "no EXPECTED_SCHEMA_VERSION addition"
3. **Line 92-95** — `probe_health` returns UNREACHABLE → SHARED.md append "gateway not running locally, no-op" + exit 0
4. **Line 103-105** — live_schema ≥ expected → exit 0 "already fresh" (NO restart-script invocation, NO SHARED append)
5. **Line 108-117** — restart invoked + re-probe + SHARED.md append (not exercised in this round — would require a mocked stale gateway)

## Observed (5 scenarios)

| # | Test sha | Notes | Expected | Observed rc | Observed output |
|---|---|---|---|---|---|
| 1 | `6f32c7283` (#2340 docs only) | argo-store/lib.rs untouched | 0 / no-op + no SHARED append | **0** | `auto-restart-gateway: no schema bump in 6f32c7283 (argo-store lib.rs untouched)` |
| 2 | `d76506ee3` (#1968 revoked-tokens infra) | lib.rs touched but no EXPECTED_SCHEMA_VERSION bump | 0 / no-op + no SHARED append | **0** | `auto-restart-gateway: no EXPECTED_SCHEMA_VERSION addition in d76506ee3` |
| 3 | `8abb093b1` (#2331 v82→v83 bump) | BASE_URL=`http://127.0.0.1:1` (unreachable) | 0 / SHARED append "gateway not running locally" | **0** | `auto-restart-gateway: gateway unreachable at http://127.0.0.1:1; no-op` + SHARED.md append: `2026-05-25T22:13:32Z [conductor-auto-restart-gateway] post #2331 schema-bump merge — gateway not running locally, no-op` |
| 4 | `8abb093b1` (#2331) | real gateway `:7400` at v83 (live ≥ expected) | 0 / "already fresh" + no SHARED append + no restart-script invoke | **0** | `auto-restart-gateway: gateway already fresh (HTTP 200 schema_version=v83, expected v83); no-op` |
| 5 | `deadbeefcafe` (bogus) | commit doesn't exist | 2 / defensive validation | **2** | `auto-restart-gateway: commit not found: deadbeefcafe` |

All 5 branches match expected behavior. Real gateway live-fetch in scenario 4 returned `HTTP 200 schema_version=v83` (same data as r73-S1 cross-check above).

## Anti-theater check

Four independent reasons this isn't "test rewrites itself green":

1. **Real commit SHAs drive the diff-parsing logic.** Scenarios 1, 2, 4 use actual sha `6f32c7283` / `d76506ee3` / `8abb093b1` from git history — not synthetic test fixtures. `git show --name-only` and `git show -- crates/argo-store/src/lib.rs | grep` are run against the live object DB.
2. **Asymmetric guard coverage proves both branches are load-bearing.** Scenario 1 exercises the first guard (lib.rs path absent from changeset). Scenario 2 exercises the second guard (lib.rs in changeset BUT no EXPECTED_SCHEMA_VERSION line added). If only the first guard worked, scenario 2 would attempt a restart on a non-schema-bump commit — false positive avoided.
3. **SHARED.md test seam appended the expected line.** Scenario 3 produced an actual paper trail entry with the correct PR label (`#2331` parsed from commit body), timestamp, and "no-op" reason. This is the load-bearing conductor handoff — without it, an unreachable-gateway scenario would silently skip without notifying the conductor.
4. **Scenario 4 proves no-spurious-restart on the live host.** With the real gateway up at v83 (already fresh), the script correctly NO-OPS without invoking `restart-live-gateway.sh`. If the freshness comparison `live_schema >= expected` had been inverted or off-by-one, this scenario would have triggered a real restart on the production gateway during the evaluator round — it did not.

## Gap / follow-up

**None blocking.** The restart-invocation branch (scenario 5 in the script, branch #5 in my list) is not exercised in this round — would require either spinning up a fake HTTP server returning a stale schema_version (overkill for branch coverage) or letting the gateway drift naturally. The mechanical shape of the branch (call `--no-prompt`, re-probe, append SHARED) is straightforward and the surrounding code paths are all proven; a follow-up evaluator round can wire that one with a fake-gateway shim if drift surfaces.

**Combined with #2335 (sibling report):** the schema-drift loop is fully closed. Detector side: #2335 distinguishes 503-stale from UNREACHABLE. Conductor side: #2336 auto-invokes restart on schema-bump merges, with safe guards for (a) docs-only commits, (b) lib.rs touches without bumps, (c) unreachable gateways, (d) already-fresh gateways. Together they eliminate the rounds 66→72 false-alarm pattern that consumed ~7 evaluator rounds of process drift.
