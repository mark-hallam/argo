# fix-claim-regex-empty-col4-starves-queue

**Owner**: claude or codex — **P1 RELIABILITY + OBSERVABILITY (swarm-critical).**
**Depends on**: none.
**Authored against**: origin/main `ed6e8e1a3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-claim-regex-empty-col4-starves-queue

## Problem (root-caused 2026-06-12 — the swarm was starved for HOURS)

`scripts/claim-with-retry.sh` `apply_edit()` matches a TODO row with:
```
^(\| {brief_id} \| [^|]+ \|) TODO \| [^|]+ \|(.*)\|$
```
The 4th column (PR/commit) uses `[^|]+` — it requires **at least one
non-pipe char**. A TODO row whose PR column is **empty** (`| TODO | |`) — a
perfectly legal "no PR yet" state — does NOT match, so the claim fails with
`could not match TODO row`, rc=2.

Because workers claim the **first eligible TODO by doc order**, a single
malformed first-TODO row makes EVERY worker fast-fail on it and exit before
reaching any other TODO. The whole queue starves. The supervisor's
`/tmp/supervisor.err.log` showed `5 fast-fails in a row; cooling down 15min`
across all slots going back hours; `loops` decayed to 0 while 14 claimable
TODOs sat waiting. The conductor only caught it by reading a worker log
(`could not match TODO row for launch-wire-spendgate...`).

This is a **silent, total-swarm-outage** failure mode triggered by a
one-character ledger-formatting slip (`| — |` vs `| |`). Severity is
disproportionate to the cause — exactly the kind of brittle coupling to
root out (priorities #1 reliability, #2 observability).

## Fix (two layers — defense in depth)

### 1. Root: make the claim regex tolerant
Change the 4th-column match from `[^|]+` to `[^|]*` in `apply_edit()` so an
empty PR column claims fine (the column is genuinely empty for an unstarted
TODO). Verify the `repl()` still rewrites col-4 to `—` on claim. Add a unit
test (a row with `| TODO | |` claims successfully; a normal `| TODO | — |`
still claims).

### 2. Guard: lint + loud supervisor signal
- Add a check to `scripts/lint-brief.sh` (or a new ledger lint run in the
  conductor tick + the brief pre-commit hook) that REJECTS `| TODO | |`
  (empty col-4) with a clear message pointing at this failure mode.
- In the supervisor / worker: when a claim fails with `could not match TODO
  row` **and the row exists as TODO** (awk found it but the claim regex
  rejected), append a LOUD line to `docs/skycot/agents/SHARED.md` (or a
  dedicated alert) so a malformed-row starvation surfaces in the next
  conductor tick within minutes, not hours. A claim-regex reject on an
  existing row is a ledger-format bug, not a normal race — it must not look
  like routine backoff noise.

## Acceptance
- A `| TODO | |` row is claimable (regex test) AND is rejected by the lint
  before it can be committed (belt + suspenders).
- A simulated claim-regex-reject-on-existing-row emits a SHARED.md alert line
  (not just a silent rc=2 + backoff).

~60-120 LOC (shell + a lint rule + tests). **🔴 P1 — a 1-char ledger slip
must never again silently take down the entire swarm.**
