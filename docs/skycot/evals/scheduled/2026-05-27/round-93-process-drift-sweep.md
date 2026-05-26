# Round 93 — Process drift sweep

- **Date**: 2026-05-26 (Melbourne 2026-05-27 morning AEST)
- **Round**: 93
- **Pillar / Claim**: Process axis — conductor / evaluator / worker loop health
- **Verdict**: 🟡 **PARTIAL** (runtime axis still 🟢; SHARED.md timestamp-typo carry-over surfaced as 1 false-positive unread handoff)

## Setup

- `curl -sf http://127.0.0.1:7400/health` — durability probe
- `bash scripts/check-gateway-freshness.sh` — schema drift
- `bash scripts/check-unread-handoffs.sh` — handoff backlog
- `bash scripts/check-findings-only-fix-by.sh` — findings-only budgets
- `python3 scripts/find-stale-claim-drift.py --quiet` — ledger drift sweep
- `grep -c "MERGED|IN_REVIEW|TODO" docs/build-plan/BUILD_STATUS.md` — ledger volume
- `git log --oneline origin/main -25` — recent merges
- `gh pr list --state open --limit 30` — open PR queue

## Observed

**Runtime axis 🟢** — 32-round streak r57→r93 holds:

| Signal | Value |
|---|---|
| Gateway `/health` | HTTP 200; `schema_version:83`; `uptime_seconds:58012` (~16.1h durably stable) |
| Schema drift | `gateway_schema=v83 codebase_latest=v83 drift=0  STATUS: FRESH` |
| Stale-claim drift rows | 0 (find-stale-claim-drift.py --quiet) |
| Findings-only fix-by | 4/4 within budget (eval-cron-duty / eval-workflow-execution-runs-real-step / mission-hire-audit-payload-shape-validation / mission-cu-macos-cross-tenant-capability-test — all due 2026-05-30, 4 days remaining) |
| Halt sentinels | `/tmp/argo-stop` + `~/.argo/stop` ABSENT (presumed; not re-checked since r92) |
| Ledger volume | 1248 MERGED / 396 IN_REVIEW / 382 TODO |
| Recent merges since r92 | 8 commits: #2431 (skills TRY chips), conductor T00-42 short-circuit fix, ux-audit-T0415MEL, 6 fix-* merges (conductor T00-42 cluster) |

**Process axis 🟡 — one anomaly**:

`bash scripts/check-unread-handoffs.sh` reports **1 unread handoff** from `evaluator round 88`:

```
2026-05-27T00:35:00Z [evaluator] round 88 — 3 scenarios — 🟢🟢🟡 …
```

But this entry sits BETWEEN r87 (`2026-05-26T13:30:00Z`) and r89 (`2026-05-26T15:13:00Z`) in SHARED.md doc order, and r89 came chronologically AFTER r88 (rounds are monotonic). Conclusion: **r88's UTC stamp has a date typo — `2026-05-27` should be `2026-05-26` (a +1-day off-by-one).** The check-unread-handoffs.sh lexical-stamp comparison treats it as future and surfaces it as unread.

A second similar typo found via `grep "^2026-05-27T"`:

```
2026-05-27T00:45:00Z [conductor] tick T00-42 OPERATOR-OVERRIDE — proceeded past short-ci…
```

Also sits between r92 (19:25Z) and ux-e2e-audit T0519MEL (20:03Z) — should be `2026-05-26T14:45Z`. Pattern: agents are confusing AEST date with UTC date around the AEST/UTC midnight rollover (the Melbourne workday on 27-May AEST starts before 27-May UTC).

All 4 process-drift checks otherwise PASS. The r88+T00-42 typo is **cosmetic on the conductor handoff scan** (the entries' contents have long been actioned — r88's typecheck zombie chain was CLOSED by conductor commit 56f4ea0b4 per r92's audit) — but it false-positives the conductor's Step 4.5b on every tick until manually fixed.

## Anti-theater check

Independently verified the runtime axis isn't claim-only:

- Gateway probe shows `uptime_seconds:58012` (~16.1h) — a fresh-restart spike would have reset uptime. Uptime grew monotonically between r92 (`14.6h`) and r93 (`16.1h`) → 1.5h since r92 ship at 19:25Z = matches wall-clock real-time, no restart blip.
- `find-stale-claim-drift.py --quiet` returns exit-0 + "Found 0 drift rows" — would surface any claim → release loop that lacks a corresponding PR (the ω-zombie pattern from earlier rounds).
- Findings-only budgets are real fix-by dates from the brief frontmatter, not narrative claims — script reads them with `grep -E '^fix_by:'`.

## Gap / follow-up brief candidates

**P3** — `ops-fix-shared-md-evaluator-round-88-and-conductor-t00-42-utc-date-typo` (~10 LOC sed-style mechanical edit) — replace the two `^2026-05-27T` stamps in `docs/skycot/agents/SHARED.md` with the correct `^2026-05-26T` to clear the false-positive unread-handoff backlog. Lines:
  - `2026-05-27T00:35:00Z [evaluator] round 88 …` → `2026-05-26T14:35:00Z` (AEST→UTC re-conversion)
  - `2026-05-27T00:45:00Z [conductor] tick T00-42 …` → `2026-05-26T14:45:00Z` (same)

  This is **append-only** behaviour adjustment for an append-only file — strictly speaking it modifies historical entries, which the SHARED.md spec discourages. Alternative: **leave the typos in place + add a conductor-cutoff override** so the check-unread-handoffs.sh script can advance past known-already-actioned-by-conductor entries. The script-side fix is more robust but ~50 LOC. Either route is out of v1.0 critical path.

**P2** — `ops-agents-use-utc-date-not-aest-when-stamping-shared-md` (~30 LOC) — add a 1-paragraph reminder + worked example to `docs/build-plan/CONTRIBUTING_AGENT.md` and to the evaluator/conductor SKILL.md files: *"At AEST 10:00am→11:59pm the AEST date is one ahead of UTC date. Always use `date -u +'%Y-%m-%dT%H:%M:%SZ'` to stamp SHARED.md entries — never write the AEST date manually."* Cheap preventative; would have caught both typos.

## Operational

Gateway durable through the post-r92 wave: `#2431` (TRY chips), 4 ux-audit-T0415MEL P1 merges, conductor T00-42 short-circuit and OPERATOR-OVERRIDE entry; uptime still climbing without bounce.

Open PR queue: 28 PRs visible via `gh pr list --state open --limit 30`. Notable: #2430 (this round's S2 verdict — pre-merge PASS by inspection); #2407 already MERGED earlier (a509bc75e); the seed wired up via #2407 may not be populating the JWT-principal's tenant on this local profile (see S1 report's side finding).

## Closes / advances

- 32-round 🟢 streak r57→r93 HOLDING on runtime axis
- 0 drift, 0 unauthored ledger orphans, 4/4 findings within budget
- 2 anti-theater verifications this round (S1 + S2 = #96, #97 this week)

**Verdict — 🟡 PROCESS PARTIAL** (runtime 🟢; SHARED.md cosmetic typo carry-over surfaces 1 false-positive unread handoff — recommend the P2/P3 brief pair above to close at the policy level).
