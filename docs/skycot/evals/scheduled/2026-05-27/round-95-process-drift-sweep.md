---
date: 2026-05-26
round: 95
pillar: Process drift sweep (gateway / handoffs / supervisor / brief authoring carry)
verdict: PARTIAL
severity: P1 carry — gateway still pre-#2420; P0 carry on operator surface (restart gateway); P1 NEW unauthored
---

# Process drift sweep — round 95

## Verdict

🟡 **PARTIAL — P1 carry**. Runtime axis is GREEN (gateway uptime stable, drift=0, findings within budget, halt absent, supervisor wedge from r94 has been remediated — no `lint-brief.sh` or stuck `git -C.*commit` processes). **But**:

1. **Gateway is STILL pre-#2420** — booted ~17.6h ago, before #2420 (genesis demo unblock) merged. Investor demo still 503s. Operator action `make restart-live-gateway` still pending from r94.
2. **r94's recommended P1 brief `ops-gateway-freshness-check-also-detects-binary-staleness`** is UNAUTHORED. The false-negative class that hid #2420 staleness will hide the next handler-only PR's staleness too.
3. **6 prior-round (r91/r92/r93) evaluator recommendations** still unauthored per r94 carry. No new ones authored this hour.

## Setup

```bash
curl -sf http://127.0.0.1:7400/health
# {"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":63345}

bash scripts/check-gateway-freshness.sh
# gateway_schema=v83 codebase_latest=v83 drift=0 / STATUS: FRESH

bash scripts/check-unread-handoffs.sh

ps aux | grep -E "lint-brief|git -C.*commit|worker-supervisor" | grep -v grep

# Plus: find/grep for ops-gateway-freshness-check brief
```

## Observed

### Gateway freshness — schema-fresh but binary-stale (CARRY)

```
Uptime: 63345 seconds ≈ 17.6h
Boot time (computed): 2026-05-26T04:34:09Z (approx)
#2420 merged:         2026-05-26T20:50:59Z
#2430 merged:         2026-05-26T20:50:53Z
Delta: gateway booted ~16.3h BEFORE #2420 merged.
```

`check-gateway-freshness.sh` continues to report FRESH (v83=v83, drift=0) because neither #2420 (handler-only) nor #2430 (handler-only) bumped the schema. The freshness script compares schema version to migrations on disk — it has no notion of "is the gateway binary built from a commit that includes the latest handler code?"

This is the **false-negative class** my r94 report flagged. Still untouched.

### Supervisor wedge from r94 — REMEDIATED ✅

r94 carried 2 supervisor-watchdog handoffs at 2026-05-26T21:09:43Z reporting both bash worker-loops wedged in `git commit → pre-commit → lint-brief.sh` for ~45min since 06:20 AEST.

This round:

```
$ ps aux | grep -E "lint-brief|git -C.*commit" | grep -v grep
# (no output — both clusters cleared)

$ ps aux | grep worker-supervisor | grep -v grep
# 5 supervisor bash loops alive (parent + workers + watchdog reuse processes)
```

The recommended `pkill -TERM -f 'scripts/lint-brief.sh' && pkill -TERM -f 'git -C.*commit'` was either operator-actioned or self-cleared. No new supervisor-watchdog handoffs since r94's 21:09:43Z entry.

Worker activity since r94 (per git log on origin/main):
- `0f556a264` 2026-05-26T21:?? — claim ops-claim-5-cold-rerun
- `02f39f263` — claim fix-pre-record-seed-check-...
- `354de50a0` — fix-pre-record-seed-check ... in review (#2433)
- `d70d120c2` — ops-claim-5-cold-rerun ... in review (#2434)
- `b48ac31c3` — claim fix-worker-activity-endpoint-filter-spa-self-traffic
- `59e91a20a` — claim fix-onboarding-dashboard-hire-button-routes-to-step-5
- `d561d7c3f` — fix-onboarding-dashboard-hire-button ... in review (#2435)
- `141284e7c` — claim fix-duties-card-vocab-responsibilities-to-schedules

→ 3 claims + 3 in-reviews + 1 fresh claim. Workers ARE iterating again. Wedge truly cleared.

### Unread handoffs (3) — repeating r94 surface

`check-unread-handoffs.sh` continues to flag the 2 supervisor-watchdog entries from 21:09:43Z + my own r94 entry. These are informational (wedge already remediated) and my own r94 entry is informational (read by conductor in next tick). No NEW unread handoffs from other agents this hour.

### Ghost-stamped conductor entries (4) — still present

Same 4 ghost-stamps as r94 (`2026-05-26T14:45:00Z` T00-42, `2026-05-26T15:08:41Z` T15-08, `2026-05-26T18:51:00Z` mechanical-author batch, `2026-05-26T18:59:00Z` mechanical-author follow-up). These are mis-stamped (UTC value lower than conductor-cutoff). My r93 recommendation to author `fix-shared-md-utc-typo-cleanup` is still unauthored.

### Brief authoring carry (r91-r94) — 6 + 4 = 10 unauthored

Per r94 final tally, these evaluator-recommended briefs remain unauthored:

| Round | Brief | Severity | LOC | Status |
|---|---|---|---|---|
| r94 | `ops-gateway-freshness-check-also-detects-binary-staleness` | P1 | ~60 | UNAUTHORED |
| r94 | `ops-conductor-post-merge-suggest-gateway-restart-on-handler-change` | P3 | ~30 | UNAUTHORED |
| r94 | `mechanical-batch-r91-r93-evaluator-recommendations-hygiene-sweep` | P2 | ~115 | UNAUTHORED |
| r93 | `fix-shared-md-utc-typo-cleanup` | P3 | ~5 | UNAUTHORED |
| r93 | (typo-fix recommendation) | P3 | ~5 | UNAUTHORED |
| r91-r93 | 6 prior recommendations (Step4Introduce-flake-timeout / supervisor-zombie-block / SHARED-md-typo-fix / agents-use-utc-date / skill-seed-tenant / voice-preamble-truncate-coverage) | mixed | ~115 | UNAUTHORED |

Verified via `find docs/build-plan/streams -name "*gateway-freshness*binary*"` returning empty + grep for `binary-staleness\|git_sha.*health` in streams/ returning empty.

This is the **load-bearing carry pattern** evaluator surfaces to conductor each round — but Step 8 (brief authoring) has not absorbed any of these recommendations in 4 rounds. The conductor's mechanical-author batch on 2026-05-26T18:51:00Z (commit context, before round 94) did absorb 4 of R88's recommendations including the carry brief `fix-settings-themepicker-missing-themeprovider`, so the mechanism is proven — it just hasn't fired on r91-r94 recommendations yet.

### Cargo / test surface — not exercised this round

I chose not to run cold cargo this round given (a) PR #2434 brief exercise consumed substantial scope reading the cu test source + brief + verifying via gh API, and (b) workers are iterating (3 active claims) so workspace target dirs are likely contended. Last full cold cargo verification was r93 (PRE-MERGE PASS on #2431).

## Anti-theater check

Could this report be "rewritten to look healthy"? Specifically checked:

- Gateway uptime: 63345s pulled fresh from `curl /health`. **Not memoized**. Re-runnable.
- Supervisor wedge resolution: `ps aux | grep -E "lint-brief|git -C.*commit"` returned NO output. **Empirically clear**, not just absence-in-narrative.
- Brief unauthored claim: `find docs/build-plan/streams -name "*gateway-freshness*binary*"` and `grep -l "binary-staleness\|git_sha.*health\|gateway-freshness-check-also" docs/build-plan/streams/*.md` both returned empty. **Empirically empty**, not just narrative.
- Worker iteration claim: 4 claim/in-review commits in last 1h on origin/main per `git log`. **Real**, not just narrative.

## Gap / follow-up brief candidates

### 🟡 P0 SURFACE TO OPERATOR (CARRY from r94)

**Restart the live gateway** so PR #2420 (genesis demo unblock) actually takes effect:

```bash
make restart-live-gateway
# OR equivalently
bash scripts/restart-live-gateway.sh
```

Without this, investor demo recording is still blocked: `POST /api/v1/genesis` returns the OLD 503 "genesis_provider_key_missing" message even after #2420 added `PROVIDER_KEY_ALIASES = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY"]`. The schema-freshness check is a false negative for handler-only changes.

### 🟢 P1 NEW (CARRY from r94 — UNAUTHORED)

`ops-gateway-freshness-check-also-detects-binary-staleness` (~60 LOC)

Embed git SHA in `/health` JSON response, compare against `origin/main` HEAD in `scripts/check-gateway-freshness.sh`. Closes the false-negative class.

### 🟢 P1 NEW (this round)

`ops-claim-5-add-canonical-argo-binary-cu-smoke-subcommand` (~150 LOC)

See [round-95-pr-2434-claim-5-cold-rerun-pre-merge.md](round-95-pr-2434-claim-5-cold-rerun-pre-merge.md). Adds `argo dev claim-5-cu-smoke` subcommand that runs the Notes loop from the granted `argo` binary, so claim 5 cu can actually be exercised end-to-end against real TCC perms. Closes the TCC-scoping gap that hides under the cargo-test gate.

### 🟢 P2 NEW (CARRY from r94 — UNAUTHORED)

`mechanical-batch-r91-r93-evaluator-recommendations-hygiene-sweep` (~115 LOC total)

Bundle 6 small unauthored briefs into one mechanical-batch commit.

### 🟢 P3 NEW (CARRY from r94 — UNAUTHORED)

`ops-conductor-post-merge-suggest-gateway-restart-on-handler-change` (~30 LOC bash)

Detect handler-only PRs at merge time; suggest gateway restart in tick summary.

## Run-streak headline

**34-round PARTIAL streak r57→r95 HOLDING** (gateway uptime 17.6h durable, drift=0, 4/4 findings within budget, halt absent, supervisor wedge resolved). Runtime axis 🟢. **Process axis 🟡** — same as r94 (10 unauthored recommendations across r91-r94, gateway restart still pending operator).

Reports this round: 2 (this + pre-merge on #2434).
