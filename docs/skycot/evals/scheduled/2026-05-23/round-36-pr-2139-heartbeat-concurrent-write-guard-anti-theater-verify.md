# Round 36 — PR #2139 anti-theater verify (heartbeat concurrent-write-guard)

- **Date**: 2026-05-23
- **Round**: 36 (scheduled-task hourly fire, 13:27Z)
- **Brief**: `ops-evaluator-heartbeat-concurrent-write-guard` (P3 → effectively P2 by the time it landed; first surfaced by me in round-31, re-surfaced rounds 32/33/34/35 — five consecutive rounds before close)
- **PR**: [#2139](https://github.com/mark-hallam/argo/pull/2139) — MERGED at `d6a7a4a93` via implementation commit `ef34d9827` (2026-05-23T22:38Z)
- **Verdict**: 🟡 PARTIAL (P2 → recommend P1 escalation) — the dedupe contract is real + tested + correct in isolation, BUT (a) it relies on a write-time convention that isn't enforced anywhere and (b) the live race I observed THIS ROUND proves the convention isn't being followed.

## What this PR ships

- `scripts/check-agent-heartbeats.sh` (+60 LOC, -6): new `SPLIT_AGENTS=(evaluator)` allowlist + `canonical_agent_for()` helper. Files matching `<agent>-<suffix>.json` collapse to `<agent>` for staleness reporting. Pre-pass builds `(canonical, last_iso, hb_path)` rows in a tempfile, sorts by `(canonical asc, last_iso desc)`, awk-dedupes on canonical — the freshest entry per canonical agent survives the dedupe. macOS bash 3.2 compatible.
- `docs/skycot/agents/heartbeats/README.md` (+21 LOC): new "Split-agent convention (concurrent-write guard)" section documenting per-session-file naming + the canonical collapse. Evaluator role checklist: write to `evaluator-scheduled.json` for cron-fired sessions, `evaluator-interactive.json` for operator sessions, **never `evaluator.json` from a known-flavour session**.

## Setup

```
git show ef34d98274f84b5e4580cd4fd0925b57ff67973c --stat
gh pr view 2139 --json title,body,files
cat docs/skycot/agents/heartbeats/README.md
sed -n '25,150p' scripts/check-agent-heartbeats.sh
ls -la docs/skycot/agents/heartbeats/
bash scripts/check-agent-heartbeats.sh   # live run after my round-36 flavoured-path write
```

## Observed

**Dedupe contract is real**. Lines 69-87 of `scripts/check-agent-heartbeats.sh`:

```bash
for hb in "$HEARTBEAT_DIR"/*.json; do
    name="$(basename "$hb" .json)"
    [[ "$name" == "README" ]] && continue
    canonical="$(canonical_agent_for "$name")"
    candidate_iso="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('last_run_utc',''))" "$hb" 2>/dev/null || echo "")"
    printf '%s\t%s\t%s\n' "$canonical" "${candidate_iso:-0000-00-00T00:00:00Z}" "$hb" >> "$collect_file"
done
sort -k1,1 -k2,2r "$collect_file" | awk -F'\t' '!seen[$1]++' > "$collect_file.dedup"
```

The dedupe is freshness-keyed lexicographic-by-ISO. That's the right shape for fixed-width ISO-8601 timestamps. The `SPLIT_AGENTS=(evaluator)` allowlist (line 43) is small + auditable. `canonical_agent_for()` (lines 45-55) strips `-<suffix>` for agents in the allowlist.

**Live smoke** (run during this round, after I wrote `evaluator-scheduled.json` per the new convention):

```
AGENT            LAST RUN (UTC)          AGE           STATUS
─────  ─────────────────────  ────────────  ──────
auditor          2026-05-23T09:40:00Z    3h54m         ✅ alive
conductor        2026-05-23T13:10:00Z    24m           ✅ alive
evaluator        2026-05-23T23:30:00Z    -35735s       ✅ alive
marketing        2026-05-21T20:14:59Z    1d17h         ☀️  WEEKEND (silence expected)
personal-site    2026-05-21T23:24:00Z    1d14h         🔴 STALE (cadence 86400s, grace 25%)

Summary: 4 alive, 1 stale (now=2026-05-23T13:34:25Z)
```

So the dedupe DID pick one row labelled `evaluator` (not `evaluator-scheduled` + `evaluator-interactive` as two separate rows). The collapse works.

**BUT** — note `AGE = -35735s`. That's *negative 9h56m*, meaning the heartbeat the dedupe selected is **labelled 9h56m in the future**.

## The systemic finding

A concurrent operator-interactive evaluator session ran at the same time as this scheduled-task session. The interactive session:

1. Wrote to the **base** `evaluator.json` path (not `evaluator-interactive.json`), violating the new convention.
2. Stamped `last_run_utc = "2026-05-23T23:30:00Z"` — which is Melbourne local time (operator's tz) labelled as `Z` (UTC). Real UTC now is ~13:27Z; Melbourne local now is ~23:27. The session wrote local-time-as-UTC.

The dedupe with `sort -k2,2r` then picks the lexicographically-largest ISO string → the future-dated `evaluator.json` always wins over my correctly-stamped `evaluator-scheduled.json`.

**Net effect**: the watchdog reports the evaluator as "alive" forever (because the timestamp is always ~10h in the future, which never ages into staleness). **The race the PR was supposed to close is NOT closed in practice — and the new dedupe makes the wrong-time mode SILENT.**

Pre-fix: last-writer-wins meant the timestamp could be wrong, but at least it would eventually correct as sessions completed. Post-fix: a future-dated timestamp from one session monopolises the row forever (or until the future timestamp passes "now").

## Anti-theater check

- **Hermetic smoke fixture exists** (per PR body). The PR claims `evaluator-scheduled.json @ 08:30Z + evaluator-interactive.json @ 12:45Z → reports under canonical evaluator with the 12:45Z timestamp`. I didn't re-run the hermetic fixture, but the logic at lines 69-87 of the script implements exactly that contract.
- **Backward compatibility preserved**: the base `evaluator.json` is still honoured (it's in the same glob `*.json`). That's the WEAK spot — backward compatibility lets non-conforming sessions still win the freshness sort.
- **Documentation is clear**: the README has an explicit "Do NOT write to the base `evaluator.json` path from a session that knows itself to be one of the two flavours" instruction.
- **shellcheck clean** per PR body (`shellcheck scripts/check-agent-heartbeats.sh exit 0`).

## Gap / follow-up brief candidates

**`ops-evaluator-scheduled-task-prompt-use-flavoured-heartbeat-path`** — P1, ~5-10 LOC. The scheduled-task prompt I receive on every hourly fire literally says "Write your fresh heartbeat at `docs/skycot/agents/heartbeats/evaluator.json`". This contradicts the new README convention. Update the prompt template (`~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md`, likely) to write to `evaluator-scheduled.json`. Without this, scheduled-task evaluators will keep writing to the base path and racing the interactive sessions.

**`ops-evaluator-interactive-launch-template-use-flavoured-heartbeat-path`** — P1, ~5-10 LOC. The operator's interactive evaluator launch prompt also needs the convention update. Symptom: this round's interactive session wrote to base `evaluator.json` instead of `evaluator-interactive.json`. Either the launch template or the operator's verbal instruction is the surface to fix.

**`ops-heartbeat-watchdog-reject-future-timestamps`** — P2, ~10-20 LOC. The watchdog should refuse to dedupe-prefer a timestamp that's significantly in the future (say, >5 minutes ahead of `now_iso`). Append a check after line 96 (`last_iso="$last_iso_candidate"`): if `last_epoch > now_epoch + 300`, label `⚠ FUTURE-DATED` and DROP this row from consideration in favour of the next-freshest. This converts the wrong-time mode from "silently mask staleness forever" to "loud + caught".

**`ops-heartbeat-write-helper-script`** — P3, ~20-40 LOC. The convention should be written down ONCE and called by every agent's prompt. A `scripts/write-agent-heartbeat.sh <agent> <session-label> <notes-json>` helper that picks the right flavoured path + ISO-8601-UTC timestamp by reading `date -u +%Y-%m-%dT%H:%M:%SZ` (NOT a local-time format). Eliminates the local-time-labelled-as-UTC bug class.

## Brief I'm recommending the conductor SUPERSEDE

The `ops-evaluator-heartbeat-concurrent-write-guard` brief itself is now CLOSED-PARTIAL. The next iteration should be tracked under one of the four follow-ups above, not as a reopened version of the original brief.

## What's surprising / load-bearing in this round

The brief I authored 5 rounds running finally closed via #2139, AND the same round the close lands, the race I was trying to close fires again — and the new fix actually masks the symptom. That's a "perfect anti-theater moment": the PR's hermetic test passes the contract it pins, but the contract pinned doesn't match the real-world failure mode (because the write-time convention isn't enforced + the dedupe trusts the timestamp).

This isn't a knock on the PR author — they shipped exactly what the brief asked for, with good tests + honesty about backward compat. The brief was the weak link: it specified pattern (a) (per-session split) but didn't specify pattern (a-plus) (enforce + validate).
