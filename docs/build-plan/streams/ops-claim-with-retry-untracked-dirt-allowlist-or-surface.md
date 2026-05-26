---
brief-id: ops-claim-with-retry-untracked-dirt-allowlist-or-surface
authored-against: origin/main 6c50fcfaf on 2026-05-26
validate-before-claim: bash scripts/check-brief-freshness.sh ops-claim-with-retry-untracked-dirt-allowlist-or-surface
parallel-surfaces-checked: rg 'VOLATILE_PATHS' scripts/ → 1 hit (scripts/claim-with-retry.sh) — extends, not duplicates
priority: P3
estimated-loc: ~30-50 (bash)
estimated-files: 1-2 (scripts/claim-with-retry.sh + optional notice)
---

# ops-claim-with-retry-untracked-dirt-allowlist-or-surface

## Problem

`scripts/claim-with-retry.sh` refuses to claim a brief when the working
tree is dirty. Patch `bcb7a2cd5` (2026-05-26) added `VOLATILE_PATHS` to
stash protocol-owned dirt (heartbeats, SHARED.md, BUILD_STATUS.md,
streams/) before the reset so workers don't get kicked out by their own
sibling agents' write-windows.

But the allowlist does **not** cover operator work products that live
outside the protocol path tree. Supervisor-watchdog 2026-05-26T00:06:33Z
flagged both worker slots in TRAP state because of these 4 untracked
paths:

1. `docs/skycot/demos/pre-record-ux-audit-2026-05-25.md` — operator-REJECTED 2026-05-25T07:50Z per CLAUDE.md memory; should be deleted, but currently sits untracked
2. `docs/skycot/marketing/x-profile-2026-05-24/` — X profile avatar/banner mockup HTML+PNG bundle, 29 files
3. `web/public/brand/social/` — per-platform social-share brand assets (discord/facebook/favicons/instagram/linkedin subfolders + .DS_Store)
4. `docs/skycot/evals/scheduled/2026-05-26/round-74-*.md` — evaluator's own r74 report file (since closed; this entry may have aged out by claim time)

Workers hit the dirty check → enter same-brief-2×-fail spin-loop → get
masked by supervisor's RAM-pressure backoff (5/15/30 min cap). The
result is a silent throttling of throughput: workers respawn, dirty
check fails again, no useful work happens.

## Goal

Extend `claim-with-retry.sh` so workers no longer get stuck on operator
work products outside the protocol path tree. Two acceptable shapes —
implementer picks the cleaner one for review:

### Option A — extend VOLATILE_PATHS allowlist for operator-asset roots

Add these path roots to `VOLATILE_PATHS` in `scripts/claim-with-retry.sh:44`:

```bash
VOLATILE_PATHS=(
    'docs/skycot/agents/heartbeats'
    'docs/skycot/audit/scheduled'
    'docs/skycot/evals/scheduled'
    'docs/skycot/agents/SHARED.md'
    'docs/build-plan/BUILD_STATUS.md'
    'docs/build-plan/streams'
    # NEW: operator work-product roots that frequently sit untracked
    # for hours-to-days between operator sessions. Without these,
    # workers spin-loop on dirty-check failures.
    'docs/skycot/demos'
    'docs/skycot/marketing'
    'web/public/brand'
)
```

Pro: same stash/restore semantics; zero behavior change for operator;
operator dirt survives across worker iterations because of the
post-reset stash pop.

Con: any operator work product under these roots is silently stashed
mid-session if a worker happens to fire. Operator can lose minutes of
unsaved edits if stash pop conflicts (unlikely but possible).

### Option B — surface-to-operator hook (warn loudly, don't spin)

Add a pre-reset check that detects "non-protocol dirt sustained across
N consecutive claim attempts," and instead of looping silently, append
a single SHARED.md entry tagged `[claim-with-retry]` like:

```
2026-05-26T01:00:00Z [claim-with-retry] worker SLOT-N blocked on
untracked operator paths for 3 consecutive iterations; either commit
or .gitignore: <path1> <path2> <path3>. RAM-pressure backoff masks
this trap until cleared.
```

Then exit with the same `FAIL: working tree dirty` so supervisor still
treats it as a fail, but the operator gets one notification per worker
per trap-window rather than zero.

Pro: zero silent stashing of operator dirt; trap stays visible.
Con: doesn't unblock workers — operator action still required.

### Recommendation

Implementer picks; both are acceptable. **Option A is the faster
unblock** but bleeds operator dirt into stash on long-running sessions.
**Option B is the more honest "we're trapped" signal** but needs
operator follow-through. Either shape ships in a single PR; both
shapes pair well with a docs note in `CONTRIBUTING_AGENT.md` explaining
which path roots get the protocol-stash treatment.

## Tests

Add to `scripts/claim-with-retry-tests.sh`:

- **A1 (Option A)** — Create dirt under `docs/skycot/marketing/foo.md`;
  call `claim-with-retry.sh some-test-brief-id`; assert the script does
  NOT exit with `FAIL: working tree dirty`; assert the dirt survives
  post-claim (stash-pop'd back) when the brief was rejected.
- **A2 (Option A)** — Create dirt under `web/public/brand/social/x.png`;
  same assertions as A1.
- **B1 (Option B)** — Create dirt under `docs/skycot/demos/foo.md`;
  call `claim-with-retry.sh` 3 times; assert the 3rd run appends a
  single SHARED.md entry tagged `[claim-with-retry]`.

Test harness already creates a temp repo under `/tmp/argo-claim-test-*`
— extend it; don't write to the real repo.

## Acceptance

- `bash scripts/claim-with-retry-tests.sh` passes all existing + new tests.
- Manual smoke: with the 3 operator-work-product directories present and
  uncommitted, claim a fresh TODO brief and confirm the claim succeeds
  (Option A) or that SHARED.md gets a single `[claim-with-retry]` entry
  (Option B).
- PR adds a one-line note in `docs/build-plan/CONTRIBUTING_AGENT.md`
  pointing at the extended allowlist or the new SHARED.md surface.

## Non-goals

- Do NOT change the protocol VOLATILE_PATHS contract (heartbeats,
  SHARED.md, BUILD_STATUS.md, streams/ — those keep their existing
  stash/restore behavior).
- Do NOT add path roots that the swarm itself writes to (would cause
  worker-vs-worker stash conflicts).
- Do NOT auto-commit operator work products; that's an operator
  decision.

## Why now

Workers are silently throttled by the trap right now (per
supervisor-watchdog 00:06:33Z flag + evaluator R75 P3 advisory). The
overnight Genesis+Track P merge-aggressive window expires
2026-05-26T22:00Z AEST and we need workers picking up the queue at full
throughput.

## References

- `scripts/claim-with-retry.sh:44` (`VOLATILE_PATHS` array)
- `scripts/claim-with-retry-tests.sh` (test harness)
- Evaluator R75 P3 recommendation
- Conductor T00-11 heartbeat (worker-trap diagnosis)
- Patch `bcb7a2cd5` (original VOLATILE_PATHS introduction)
