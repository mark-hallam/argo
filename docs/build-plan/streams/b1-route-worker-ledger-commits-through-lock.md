# b1-route-worker-ledger-commits-through-lock

Authored against: origin/main 75cc69d086eaf81301ce6909f6acd74dcd413646 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh b1-route-worker-ledger-commits-through-lock

## Goal

Stop worker ledger/status commits from STRANDING on the shared checkout's local
main when their push fails — the root cause of the 2026-06-10 SWARM STALL. Route
worker ledger commits (status flips, zombie-releases) through B-1's
`scripts/with-ledger-lock.sh` AND ensure a commit that cannot push does not
leave local main diverged (which fails every subsequent worker with
`git pull --ff-only` rc=128).

⚠️ **OPERATOR-REVIEW-BEFORE-MERGE**: this changes core worker-loop behavior
(claude-worker-loop.sh). Like B-3/B-4, the conductor HOLDS the merge for operator
sign-off after gate + review. Build it, gate it, review it — do NOT auto-merge.

## Why now (2026-06-10 swarm-stall incident — DEC-042 B-1 refinement)

B-1 (#2753) shipped `with-ledger-lock.sh` and wired the WORKER ledger commits to
serialize — but a worker zombie-release that COMMITS to local main and then FAILS
TO PUSH leaves local main 1-ahead. The next worker's `git pull --ff-only` then
fails rc=128, the supervisor backs the slot off (up to 900s), and the swarm
collapses (observed: loops 9→1, ~25 min stall; recovered manually by the
conductor realigning local main + re-releasing 2 stranded-CLAIMED briefs incl. a
P1). The race no longer just delays a conductor commit — it HALTS THE SWARM.

## Fix (in scripts/claude-worker-loop.sh, minimal + reversible)

1. Wrap every worker ledger/status commit+push in `with-ledger-lock.sh` (if not
   already), AND make the sequence atomic-or-clean: `git add <files> && git commit
   && git fetch origin main && git rebase origin/main && git push origin main` —
   if the PUSH fails, RESET local main back to origin/main (so it never strands
   ahead). I.e. a failed push must NOT leave local main diverged.
2. Add a self-heal preflight at worker start: if `git pull --ff-only` fails (local
   main diverged), `git fetch && git reset --hard origin/main` to recover (this is
   the documented manual recovery — automate it so a single stranded commit can't
   cascade into a swarm stall).
3. Keep it behind no flag (this is a reliability fix, default-on) OR a default-ON
   `ARGO_WORKER_LEDGER_SELFHEAL=1` if the operator wants a kill switch.

## Acceptance / verifier

1. A test/sim: a worker commit whose push is forced to fail leaves local main
   == origin/main afterward (NOT 1-ahead).
2. A worker started on a deliberately-diverged local main self-heals to
   origin/main and proceeds (no rc=128 exit).
3. `bash -n` + shellcheck clean on claude-worker-loop.sh.
4. The normal happy path (push succeeds) is byte-identical.

## Size / owner

~20-40 LOC bash. Owner: claude. 🔴 P1 RELIABILITY — prevents the swarm-stall
recurrence (DEC-042 B-1 refinement). OPERATOR-REVIEW-BEFORE-MERGE.
