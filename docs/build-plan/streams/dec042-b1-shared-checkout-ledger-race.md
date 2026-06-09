# dec042-b1-shared-checkout-ledger-race

**Owner**: claude.
**Authored against**: origin/main `517955308` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh dec042-b1-shared-checkout-ledger-race`.

> DEC-042 Stage 1, upgrade **B-1** (the first build-loop hardening step). See
> [`docs/skycot/decisions/DEC-042-continuous-self-improvement-loop.md`](../../skycot/decisions/DEC-042-continuous-self-improvement-loop.md)
> §3 B-1 + §5 Stage 1. This is the highest-priority Stage-1 row — claim first.

## Why (VERIFIED 2026-06-09, read-only)
Workers isolate brief *code* in `/tmp/$BRIEF_ID` git worktrees
(`scripts/claude-worker-loop.sh:362`), but they all `cd` back into the **single
shared `$REPO` main checkout** to edit `docs/build-plan/BUILD_STATUS.md`
(`claude-worker-loop.sh:376-383`: `git pull --rebase` → `git add BUILD_STATUS.md`
→ commit → `git push origin main`). The **conductor** also runs its own cargo
builds in that same `$REPO` checkout. Two distinct contention surfaces share one
working tree:

1. **Build collision.** A conductor (or gateway) `cargo build` in `$REPO` while
   N workers also compile there shares target dirs + the OS file cache and pegs
   RAM. This bit us **2026-06-09** (DEC-042 §1.3): a gateway rebuild + the
   workers collided and the swarm had to be *paused* (`touch /tmp/argo-stop
   ~/.argo/stop`) to rebuild safely. Verified live this tick: with 3 workers
   building, `vm_stat` free+speculative was **~64 MB** — far under the
   supervisor's own 1 GB spawn floor — so the conductor cannot safely build at
   all. "Pause the swarm to build" is a manual dance, not a fix.
2. **Ledger write race.** Concurrent `pull --rebase` + commit + push of the SAME
   `BUILD_STATUS.md` from multiple workers + the conductor. Line-disjoint edits
   usually rebase cleanly, but **same-row** edits collide, and the watchdog's
   mid-edit stash can orphan a brief mid-write (auto-memory
   `feedback_conductor_recent_commit_check` + the ledger-guard false-positive
   note). The result is divergence flakes (`feedback_worker_git_divergence_flake`)
   and the occasional clobbered status row.

DEC-042 B-1 verifier (the bar this brief must hit): *"run N workers + a conductor
build concurrently; zero divergence flakes over a fixed window."*

## Grep-before-write
- `rg -n 'BUILD_STATUS|git pull --rebase|git push|worktree add|cd .*REPO|flock|lock' scripts/claude-worker-loop.sh` — the worker ledger-edit + worktree path to wire into (lines ~147-170 claim-edit, ~376-383 status-edit).
- `rg -n 'CARGO_TARGET_DIR|argo-shared-target|/tmp/.*target|cargo build|cargo test' scripts/worker-supervisor.sh scripts/*.sh` — where build target dirs are set today (supervisor children already use `$HOME/.cache/argo-shared-target` per CLAUDE.md; the gateway plist still uses `/tmp/argo-shared-target`). The conductor has **no** dedicated build tree yet — that is the gap.
- `ls scripts/ | rg -i 'gate|restart-live-gateway|build'` — `conductor-pre-merge-gate.sh` + `restart-live-gateway.sh` are the conductor builds that currently run in `$REPO`; they are the consumers of the new build worktree.
- `rg -n 'with-ledger-lock|ledger-lock|conductor-build-worktree' scripts/` — confirm neither helper exists yet (they don't as of `517955308`; this brief creates them — do NOT declare a parallel one elsewhere).
- macOS note: `flock(1)` (util-linux) is **not** present on macOS by default. Use a portable mutex (a `mkdir`-based spinlock with a stale-lock timeout, or `/usr/bin/python3 -c 'import fcntl…'`). Verify with `command -v flock` before assuming it.

## Fix (two additive scripts + one wire-in; ship the 80% first)
First-principles: the worktree isolation that already works for brief *code*
should extend to (a) the conductor's *builds* and (b) the *ledger* read-modify-
write. Both are additive + fall back to today's behavior if the helper is absent
— **no behavior change for a worker that does not opt in** (DEC-042 "default-OFF
where a flag is involved"; here the safety is additive-with-fallback).

1. **`scripts/conductor-build-worktree.sh`** (NEW, < 200 LOC). Creates/reuses a
   dedicated build worktree at `$HOME/.cache/argo-conductor-build` checked out
   from `origin/main`, with its own `CARGO_TARGET_DIR`
   (`$HOME/.cache/argo-conductor-build-target`), so a conductor `cargo` run never
   touches the shared `$REPO` tree or the workers' target dir. Idempotent:
   `git -C <wt> fetch origin main && git -C <wt> reset --hard origin/main` to
   refresh before a build (this is the conductor's OWN tree — `reset --hard`
   here is SAFE, it is never the shared `$REPO`). Print the worktree path + the
   `CARGO_TARGET_DIR` to stdout so callers can `eval` them. Wire
   `conductor-pre-merge-gate.sh` (and document for `restart-live-gateway.sh`) to
   build through it. This alone removes the "pause the swarm to build" dance —
   the 2026-06-09 failure.
2. **`scripts/with-ledger-lock.sh`** (NEW, < 120 LOC). A portable mutex wrapper:
   `with-ledger-lock.sh -- <command…>` acquires a lock (mkdir spinlock on
   `$REPO/.git/skycot-ledger.lock` with a configurable timeout, default 60s, and
   a stale-lock reaper that removes a lock whose holder PID is dead), runs the
   command, releases on exit (trap). If the lock dir is uncreatable for any
   reason, **fall through and run the command anyway** (additive safety, never a
   hard block — a worker must not deadlock waiting on a lock).
3. **Wire it into `claude-worker-loop.sh`.** Wrap the two ledger read-modify-
   write blocks (the claim edit ~147-170 and the status edit ~376-383) in
   `with-ledger-lock.sh` *iff* the helper exists (`[ -x scripts/with-ledger-lock.sh ] && …`),
   so the `pull --rebase`/edit/commit/push of `BUILD_STATUS.md` is serialized
   per-host. Keep the existing `pull --rebase` fallback inside the lock (defence
   in depth for cross-host pushes). Do NOT change which row a worker claims or
   the row format.

Defer to a follow-up brief `dec042-b1b-ledger-fragments` (name it in this brief's
"On ship" so it is not lost): the larger redesign of moving status writes to
per-worker fragment files the conductor merges. The lock is the 80% fix; the
fragment model is the 100% fix and is a separate, bigger change.

## Acceptance (truth-bounded — must end in a CHECKABLE verifier, DEC-042 safety spine)
1. **Race harness `scripts/test-ledger-race.sh`** (NEW, < 150 LOC): spawns
   `K=8` concurrent processes that each, under `with-ledger-lock.sh`, append a
   uniquely-tagged line to a throwaway copy of a ledger file and `git`-style
   read-modify-write it `M=10` times; asserts **zero lost updates** (all
   `K*M` tags present) at the end. Run it 3× → 3/3 zero-loss. This is the B-1
   "zero divergence flakes over a fixed window" verifier, made self-contained
   (no live swarm needed to prove the lock serializes).
2. **Build-isolation check**: `scripts/conductor-build-worktree.sh` prints a path
   under `$HOME/.cache/` (NOT `$REPO`, NOT `/tmp` under pressure) and a separate
   `CARGO_TARGET_DIR`; a `cargo metadata` (cheap, no full compile) run through it
   succeeds and touches no file in `$REPO` (`git -C $REPO status --short` is
   unchanged before/after). Full builds are proven by the existing gate, not by
   this brief (RAM-bound).
3. Both new scripts pass `bash -n` (syntax) + `shellcheck` if available; each
   < 500 LOC; `claude-worker-loop.sh` stays < 500 LOC after the wire-in (it is
   ~640 today — if the wire-in would push a sub-block over, extract the
   ledger-edit into a sourced helper rather than inlining).
4. No change to a worker's claimed-row selection, the row schema, or the push
   protocol when the helpers are absent (prove: run the loop's ledger-edit path
   with the helper renamed away → identical behavior).

## On ship (same commit)
- Add a `FEATURE_MATRIX.md` row (or extend the build-loop/observability area) for
  "B-1 shared-checkout race hardening (conductor build worktree + ledger lock)"
  with the commit SHA + the two script paths; regenerate
  `docs/skycot/feature-matrix.html` via `python3 scripts/render-feature-matrix.py`
  in the SAME commit.
- Name the deferred follow-up: `dec042-b1b-ledger-fragments` (per-worker status
  fragments merged by the conductor — the 100% fix).
- This unblocks Stage-1 B-2 (`Stop`-gate): once the conductor builds in its own
  worktree, the `Stop` hook can run the gate without contending with workers.

## Effort
M (two small bash scripts + a wire-in + a self-contained race harness; no Rust,
no migration, no runtime flag — additive ops tooling with a fallback path).
