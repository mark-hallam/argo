# Round 70 — PR #2323 5th-round merge stall (gh re-confirmed CLEAN/MERGEABLE)

- **Date**: 2026-05-25 (UTC) / 2026-05-26 05:09 AEST (Melbourne)
- **Round**: 70
- **Pillar/Claim**: Operational hygiene — pre-merge-passed PR sitting in queue across 5 evaluator rounds
- **Verdict**: 🟡 PARTIAL — MERGEABLE/CLEAN confirmed, 5th-round merge recommendation, no shipper picked it up
- **Severity tier**: P0 (recurrent ops-side stall on PRE-MERGE-PASS PR)

## Setup

```bash
gh pr view 2323 --json mergeable,mergeStateStatus,headRefOid,updatedAt,baseRefOid
curl -sS -o /dev/null -w "/api/v1/skills HTTP %{http_code}\n" http://127.0.0.1:7400/api/v1/skills
curl -sS -o /dev/null -w "/api/v1/workflows HTTP %{http_code}\n" http://127.0.0.1:7400/api/v1/workflows
```

## Observed

`gh pr view 2323` returns:

```json
{
  "baseRefOid": "819f8064f92cb4af61708a42339dc6acd3992130",
  "headRefOid": "d68c95f30fa09852b53931024269405d7e81dc9e",
  "mergeStateStatus": "UNKNOWN",   // initial probe
  "mergeable":        "UNKNOWN",   // initial probe
  "updatedAt":        "2026-05-25T15:42:10Z"
}
```

Initial probe returned UNKNOWN/UNKNOWN; the subsequent `gh pr list` (a few seconds later in this session) showed `"mergeStateStatus":"CLEAN","mergeable":"MERGEABLE"` for #2323 — GitHub's recomputation caught up. Head SHA unchanged since 15:42:10Z (~3h27m before round start).

Live endpoints that the PR adds — both still HTTP 404 on the running gateway (consistent with stale-binary + #2323-not-shipped):

```
/api/v1/skills    HTTP 404
/api/v1/workflows HTTP 404
```

Note these are 404 even AFTER the gateway is restarted to pick up #2331 (since #2323 hasn't merged) — once gateway is restarted to pick up the v83 schema, /health will go 200 but /skills + /workflows will stay 404 until #2323 also ships.

T18-08 conductor heartbeat flagged "branch is 87 commits behind main, may need rebase brief". Current `gh` API result CLEAN/MERGEABLE *contradicts* that claim — either the diff was always small enough to not trigger a textual conflict despite the 87-commit drift, or T18-08 misread `git diff origin/main` output. Either way, the rebase-brief recommendation from r69 (`fix-pr-2323-rebase-on-current-main`) is no longer needed; just merge.

This is the **5th consecutive evaluator round** (r65, r67, r68, r69, r70) recommending MERGE for #2323:

| Round | Verdict | Note |
|---|---|---|
| r65 | source-PASS / 1st recommendation | Initial PRE-MERGE source review passed |
| r67 | source-PASS / 2nd recommendation | Cargo cycles spent on more-urgent #2317 P1 escalation |
| r68 | source-PASS / 3rd recommendation | Conductor T17-11 noted "recommend normal-gate merge next tick" |
| r69 | source-PASS / 4th recommendation | UNKNOWN/UNKNOWN at round close (gh recomputing post T18-08 wave) |
| r70 | source-PASS / 5th recommendation | CLEAN/MERGEABLE confirmed by gh API at round start |

## Anti-theater check

- Did NOT re-run cargo for #2323 — head SHA `d68c95f30` unchanged since r67 source-PASS. Re-running would burn ~5min wall + ~10K tokens with no new evidence (the only thing that could invalidate r67 PASS is a HEAD change OR a main-side breaking change to a touched file, neither of which happened).
- Did NOT pull #2323 source — already reviewed in r67's report at `docs/skycot/evals/scheduled/2026-05-25/round-67-pr-2323-gateway-skills-workflows-list-pre-merge-source-pass.md` (LOC < 500 per file, SQL tenant-scoping correct, 4 cross-tenant negative tests in place).
- DID probe live endpoints `/api/v1/skills` + `/api/v1/workflows` to confirm 404 status hasn't changed — this is the "did it ship?" signal and the cheapest possible probe.
- DID check both `gh pr view` AND `gh pr list` to cross-verify mergeability — UNKNOWN→CLEAN transition catches GitHub's eventual-consistency window.

## Gap / follow-up brief candidates

| Severity | Brief id | Rationale |
|---|---|---|
| 🟢 P0 MERGE | #2323 (no brief, conductor lever) | 5 rounds of PRE-MERGE-PASS without a shipper. Normal-gate via `bash scripts/conductor-pre-merge-gate.sh` (NOT --skip-clippy since #2323 is `claude-fix-gateway-*` not the operator-authorized `fix-pr-22*` overnight category). |
| 🟡 P3 | `ops-flag-pre-merge-passed-pr-aging` (NEW η-r70) | Add a check to the conductor protocol: if a PR has an evaluator PRE-MERGE-PASS recorded in a SHARED.md handoff older than 3 rounds (~3h), surface it explicitly in the conductor tick under a "🚨 AGED-PRE-MERGE-PASS" header so it doesn't keep getting deferred. ~40 LOC bash sibling to `check-unread-handoffs.sh`. |

The stall pattern is consistent: cargo-running PRE-MERGE source-pass produces a recommendation, but each conductor tick either runs out of time, hits a memory wall, or cites authorization-scope ambiguity, and the PR ages another hour. The η-r70 brief (aging detector) would create the same forcing function for PRE-MERGE-PASS PRs that `check-unread-handoffs.sh` creates for unread evaluator handoffs.

## Cost

~4K tokens (1 gh API call already cached from start-of-run + 2 cheap curl probes + writeup). No cargo, no agent spawn.
