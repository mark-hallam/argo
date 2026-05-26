# Round 90 · S3 · Typecheck-regression + conductor-follow-through sweep

- **Date**: 2026-05-26 (UTC) / 2026-05-27 AEST
- **Round**: 90
- **Claim/Pillar**: process drift — repeat-pattern detection (rounds 85→90)
- **Verdict**: 🟡 **PARTIAL** — runtime axis 🟢 (32-round streak r57→r90 HOLDING), brief-carry axis 🔴 (carry briefs now 6/5 rounds unclaimed + 1 brief NEVER AUTHORED from r88 recommendation)

## Setup

```bash
curl -sf http://127.0.0.1:7400/health
bash scripts/check-gateway-freshness.sh
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
grep -n "fix-pr-2404-use-provision\|fix-pr-2366-provenance-block-hastext\|ops-conductor-pre-merge-gate-web-typecheck\|fix-settings-themepicker" docs/build-plan/BUILD_STATUS.md
find docs/build-plan/streams -name "fix-settings-themepicker*"
cd /tmp/r90-2429-wt/web && pnpm tsc --noEmit
```

## Observed — runtime axis (🟢 HOLDING)

| Probe | Result |
|---|---|
| Gateway health | `{"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":49030}` (~13.6h uptime, durably stable through entire overnight Genesis+Track P merge wave) |
| `check-gateway-freshness.sh` | `gateway_schema=v83 codebase_latest=v83 drift=0` → `STATUS: FRESH` |
| Halt sentinels | `/tmp/argo-stop`: absent. `~/.argo/stop`: absent. |
| Findings-only budget | 4 within-budget · 0 due-today · 0 overdue (all 4 due 2026-05-30, 4 days remaining) |
| Mid-rebase stuck | None (`.git/rebase-merge` + `.git/rebase-apply` both absent — the r89-T15-17Z supervisor-watchdog `REBASE-STUCK` heartbeat was resolved within the ~3h since) |
| Open PRs | 30+ in_review (drain queue active; #2422 just merged at 18:09Z; #2429 verified this round) |

**32-round 🟢 streak r57→r90 HOLDING on runtime axis** (gateway uptime durably crossing 14h, 0 drift rows, halt sentinels absent, 4/4 findings within budget through 4 hourly evaluator runs + ~5 conductor ticks since r89).

## Observed — typecheck regression carry (🔴 ESCALATING)

`pnpm tsc --noEmit` on PR #2429 head (= origin/main + 6 LOC, so substantively == main) returns exit 2 with exactly the same 2 errors seen since r85:

```
components/skycot/audit/ProvenanceBlock.tsx(36,56): error TS2345: Argument of type 'string | undefined' is not assignable to parameter of type 'string'.
lib/onboarding/use-provision.ts(58,3): error TS2741: Property 'created' is missing in type '{ tenantId: string; workspaceId: string; }' but required in type 'ProvisionResult'.
```

Brief status per `BUILD_STATUS.md` lines 351-353:

| Brief | First flagged | Status | Rounds unclaimed |
|---|---|---|---:|
| `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` | r85 | TODO | **6** (r85→r90) |
| `fix-pr-2366-provenance-block-hastext-accepts-optional` | r86 | TODO | **5** (r86→r90) |
| `ops-conductor-pre-merge-gate-web-typecheck-pin` | r86 | TODO | **5** (r86→r90) |

All 3 are claude-pool briefs sized 1-3 LOC (use-provision) / 3 LOC (provenance-block) / ~30 LOC bash (typecheck-pin). None have been claimed since first-flag despite 5+ rounds.

## Observed — r88 recommendation NEVER AUTHORED

R88 recommended P1 brief `fix-settings-themepicker-missing-themeprovider-in-tests` (~10 LOC) to close 4 vitest failures in `__tests__/settings/page.test.tsx` caused by PR #2368 not wiring `ThemeProvider` in test renderers.

```bash
find docs/build-plan/streams -name "fix-settings-themepicker*"
# (no output)
```

The brief was **never authored by the conductor**. The vitest regression in `pnpm vitest run` (6 FAILED / 1641 across 3 test files) has now persisted ≥3 rounds with no remediation tracking.

## Anti-theater check

Process-drift sweeps are inherently observation-not-test. Anti-theater check on this report itself:

- The 2 typecheck errors were confirmed by running `pnpm tsc --noEmit` against an independent worktree (not against my main repo, which could have local edits).
- The "NEVER AUTHORED" claim was verified with `find docs/build-plan/streams -name "fix-settings-themepicker*"` returning zero rows — not relying on negative memory.
- The brief-carry round counts are anchored to `BUILD_STATUS.md` ledger line numbers (351-353), not narrative recall.

## Gap / follow-up brief candidates

- 🔴 **P1 SURFACE TO OPERATOR** — `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` is **6 rounds unclaimed**. Conductor has had 5 ticks to mechanical-author this 1-LOC fix and hasn't. Operator-decision moment: (i) bump freshness via `scripts/check-brief-freshness.sh` and force-claim, (ii) conductor mechanical-authors the PR personally, or (iii) STALE-WONTFIX if the regression isn't actually blocking the demo MP4.
- 🔴 **P1 SURFACE TO OPERATOR** — `fix-pr-2366-provenance-block-hastext-accepts-optional` is **5 rounds unclaimed**. Same options as above for the 3-LOC fix. Risk profile higher than #2404: conditional Scene 6 audit drill crash if all preceding rows have empty `id` + `full` AND chain-anchor present.
- 🔴 **P1 SURFACE TO OPERATOR** — r88-recommended `fix-settings-themepicker-missing-themeprovider-in-tests` was **NEVER AUTHORED**. Evaluator→conductor handoff broke on this one. Recommend operator (a) ping conductor in next tick prompt to author it, or (b) author it manually.
- 🟡 **P1 ESCALATE** — `ops-conductor-pre-merge-gate-web-typecheck-pin` **5 rounds unclaimed**. Each subsequent round confirms the bet: 4+ same-root-cause TS regressions in <13h means pinning `pnpm tsc --noEmit` to the pre-merge gate when `web/**` is touched would already be strictly cheaper than the per-PR leaf cost. Approach the operator: P3→P1.
- 🟡 **P2 NEW** — `ops-supervisor-block-runaway-zombie-claims-on-brief` (see Scenario 2 report) — 50-cycle ghost-tick loop on `fix-spa-client-use-same-origin-gateway-rewrite` between 03:00→04:04 AEST burned ~64min of worker budget before #2429 finally shipped.

## Streak note

The **32-round 🟢 streak r57→r90** measures runtime-axis state (gateway up, schema fresh, halt absent, findings within budget). It does NOT measure typecheck, vitest, or conductor follow-through on evaluator recommendations — all three of those are degrading in parallel. This gap-between-axes is itself process drift worth flagging.
