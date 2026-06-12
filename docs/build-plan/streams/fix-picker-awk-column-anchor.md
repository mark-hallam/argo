# fix-picker-awk-column-anchor

**Owner**: claude or codex — **P1 RELIABILITY (swarm-critical).**
**Depends on**: none (complements #2864, which fixed the CLAIM side; this fixes the PICK side).
**Authored against**: origin/main `5301f76ac` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-picker-awk-column-anchor

## Problem (root-caused 2026-06-12 — second starvation in one day, different vector)

The brief-picker in `scripts/claude-worker-loop.sh` (~line 322) selects the
first claimable TODO with:

```
awk -F'|' '/^\| .* \| .* \| TODO \|/ { ... print $2 }'
```

The `.*` atoms are **greedy and match pipes**, so the regex is NOT anchored to
the status column. Any row whose **description text** contains a literal
pipe-space-TODO-space-pipe sequence matches, regardless of its real status.

That is exactly what happened at ~19:54 local 2026-06-12: the ledger row for
`fix-claim-regex-empty-col4-starves-queue` (status IN_REVIEW, PR #2864)
**quoted the malformed row shape it had fixed, pipes included, in its
description column**. The picker matched it as "first TODO", `claim-with-retry.sh`
correctly refused (the row is not TODO), the worker fast-failed 2× and exited,
the supervisor respawned a fresh worker into the same wall every ~2.5 min.
The swarm starved for ~40 min until the conductor reworded the row pipe-free
(commit on main, 2026-06-12). The irony is the point: the fix for starvation
#1 caused starvation #2, because picker and claimer parse the ledger with
different rules.

## Required change

1. **Column-anchor the picker.** In the awk, stop regex-matching the whole
   line; test field 4 (status column) directly, e.g.:

   ```awk
   awk -F'|' 'NF >= 5 {
     s = $4; gsub(/^[ \t]+|[ \t]+$/, "", s)
     if (s == "TODO") { id = $2; gsub(/^[ \t]+|[ \t]+$/, "", id); print id }
   }'
   ```

   Keep the existing owner-column permissiveness (any owner may claim — see
   the 2026-05-27 comment block already in the file; preserve that comment).

2. **One parser, two callers (subtract before adding).** If practical, hoist
   the row-parse into a small shared helper (e.g. `scripts/ledger-first-todo.sh`
   or a function sourced by both `claude-worker-loop.sh` and
   `claim-with-retry.sh`) so pick and claim can never disagree about what a
   TODO row is again. If the hoist balloons, the field-4 anchor alone is an
   acceptable 80% ship — note the residual in the PR body.

3. **Regression test.** Bash assertions alongside the existing claim-regex
   tests (see the #2864 test home): a fixture ledger with (a) an IN_REVIEW row
   whose description contains a literal `| TODO |` string, (b) a real TODO row
   below it — assert the picker returns (b), not (a). Also assert a TODO row
   with empty col-4 is still picked (parity with #2864's relaxation).

4. **Note the detection gap is closed by #2864's alert.** Once #2864 merges,
   `alert_claim_regex_reject` writes a LOUD SHARED.md line whenever the claim
   regex rejects an existing row — which is the symptom this bug produces. No
   extra alerting needed here; just confirm the alert fires in the test if
   cheap to do.

## Verify

- `bash <the new assertion script>` green; existing claim-regex assertions stay green.
- `shellcheck scripts/claude-worker-loop.sh` clean (or no new warnings).
- Manual: run the picker awk against the live `docs/build-plan/BUILD_STATUS.md`
  and confirm the first row printed has real status TODO (grep the row).

## May touch

- `scripts/claude-worker-loop.sh`
- `scripts/claim-with-retry.sh` (only if hoisting the shared parser)
- new test/assertion script + (optional) `scripts/ledger-first-todo.sh`
