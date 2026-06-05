# ops — worker shared cargo target dir + compile serialization

**Owner**: unclaimed (ops / swarm-infra)
**Depends on**: none (harness-only change; no crate code touched)
**Authored against**: origin/main `250be9e25` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-worker-shared-cargo-target-dir-and-concurrency-cap`.

## Problem (measured, not assumed)

Every worker currently compiles in its **own** per-brief target dir
(`/tmp/$BRIEF_ID-target`, ~7 GB each), set by the worker prompt convention,
not by a hardcoded env in `claude-worker-loop.sh`. Three measured consequences
(captured 2026-06-05 on the live machine):

1. **Zero artifact reuse across workers / iterations.** Each brief recompiles
   the whole dependency graph from cold. `sccache` is wired
   (`RUSTC_WRAPPER=/opt/homebrew/bin/sccache`, 20 GB cap, `CARGO_INCREMENTAL=0`)
   but `sccache --show-stats` reports **Rust cache-hit rate 0.90 %** (22 hits /
   2431 misses). sccache is doing effectively nothing for Rust — the per-brief
   target dirs defeat reuse, and sccache can't cache workspace path-deps well.
2. **Memory thrash when two compiles overlap.** Two independent `cargo build`
   invocations against two separate target dirs run two full parallel rustc
   fleets → OOM under macOS jetsam (swap hit 9.3 GB earlier today; the operator
   dropped the swarm to 1 slot as a stopgap so only one compile runs at a time).
3. **Unbounded disk bloat.** `~/.cache/argo-shared-target` (the supervisor's
   gate/clippy dir) has grown to **136 GB**; `/tmp/argo-shared-target` is 15 GB;
   `~/Library/Caches/Mozilla.sccache` is at its 20 GB cap. Nothing prunes the
   shared gate dir.

## The fix (one shared, locked, bounded target dir)

Switch all worker cargo to a **single shared `CARGO_TARGET_DIR`** on real disk
(NOT `/tmp` — survives reboot). This yields three wins at once:

- **Artifact reuse.** Worker B reuses the `serde`/`tokio`/workspace crates
  worker A already compiled — cargo's own fingerprinting, which is far more
  effective for a workspace than sccache's 0.9 %.
- **Free compile serialization → no OOM.** Cargo takes an **exclusive file
  lock** on the target dir (`<target>/debug/.cargo-lock`). A second concurrent
  `cargo build` against the same dir **blocks** until the first releases it,
  instead of running a second parallel rustc fleet. This is exactly the
  behaviour we want: it makes a 2nd worker slot safe to re-enable later (it does
  web/docs/non-cargo work, or waits on the lock) without re-introducing the
  2-concurrent-compile OOM. **No custom semaphore needed** — cargo provides it.
- **One dir to bound** instead of N unbounded ones.

### Required changes

1. **Worker target-dir convention → shared.** Change the per-brief
   `export CARGO_TARGET_DIR=/tmp/$BRIEF_ID-target` convention to a single shared
   dir, e.g. `export CARGO_TARGET_DIR="$HOME/.cache/argo-worker-shared-target"`.
   This convention lives in the **worker prompt text** inside
   `scripts/claude-worker-loop.sh` (the cleanup/disk section, ~lines 379-397 and
   the cleanup block near ~line 800) and may also be referenced in
   `docs/build-plan/SWARM_PROTOCOL.md`. Grep both for `BRIEF_ID-target` and
   `CARGO_TARGET_DIR` and update every worker-facing instance consistently.
   - Decide whether to **reuse the existing `~/.cache/argo-shared-target`** (max
     reuse with the supervisor gate, but inherits the 136 GB; must prune first)
     or a **fresh `~/.cache/argo-worker-shared-target`** (clean start, less reuse
     with the gate). Recommend: fresh dir for workers + a one-time prune of the
     136 GB legacy dir, documented in the PR body. Pick one and justify it.

2. **🔴 CRITICAL — cleanup must NOT delete the shared dir.** The current worker
   exit cleanup does `rm -rf "/tmp/$BRIEF_ID-target"`. With a shared dir, exit
   cleanup MUST remove **only the worktree** (`git worktree remove`), never the
   shared `CARGO_TARGET_DIR`. If this is gotten wrong, every worker exit wipes
   the shared cache and defeats the entire change (and can race a live compile).
   Update both the cleanup *instructions* in the prompt and the actual cleanup
   `rm -rf` near line 800. The FORBIDDEN-paths list must add the shared dir.

3. **Bound the shared dir size.** Add a size guard so it can't grow unbounded
   like the 136 GB legacy dir. Simplest safe approach: a supervisor pre-iteration
   check — if `du -s "$CARGO_TARGET_DIR"` exceeds a threshold
   (`SHARED_TARGET_MAX_GB`, default e.g. 60) **and no cargo lock is held**, prune
   stale artifacts. Prefer `cargo-sweep --time 0` / `--installed` if available, or
   delete the dir wholesale only when idle (no `.cargo-lock` holder). NEVER prune
   while a compile holds the lock. If `cargo-sweep` isn't installed, a
   delete-when-idle-over-threshold is acceptable; document the choice.

4. **Leave sccache in place (do not remove in this brief).** It's near-useless
   for Rust today but harmless; removing `RUSTC_WRAPPER` is a separate
   subtract-decision once shared-dir reuse is proven. Note it in the PR body as a
   follow-up candidate. (Subtract carefully — one change at a time.)

## Out of scope / explicitly NOT in this brief

- Re-enabling a 2nd worker slot. That is an **operator/conductor** decision +
  `plist` edit + `launchctl` reload, done as a separate step **after** this lands
  and is observed working. This brief only makes 2 slots *safe*; it does not turn
  them on.
- Removing sccache.
- Any crate / `src/` code change. This is harness + docs only.

## Rollout is a separate step (truth-bounded — DEC-029)

Landing this PR changes the script on disk. **It does not change the running
swarm** — the supervisor and in-flight workers keep their already-loaded
behaviour until the supervisor is reloaded (`launchctl unload`/`load` of
`com.skycot.worker-supervisor.plist`), which is a **conductor/operator** action.
The PR body MUST state plainly: "code landed; NOT yet live in the running swarm —
requires a supervisor reload to take effect." Do not claim the shared dir is in
use until a post-reload worker is observed exporting it.

## May-touch

- `scripts/claude-worker-loop.sh` (target-dir convention in worker prompt +
  cleanup block)
- `scripts/worker-supervisor.sh` (optional: the size-guard pre-iteration check)
- `docs/build-plan/SWARM_PROTOCOL.md` (if it documents the per-brief target dir)
- this brief's BUILD_STATUS.md row only

## Verification (NOT a cargo build — this is a bash/doc change)

- `bash -n scripts/claude-worker-loop.sh` and `bash -n scripts/worker-supervisor.sh`
  (syntax) + `shellcheck` if available.
- Grep proof: **zero** remaining worker-facing `\$BRIEF_ID-target` references;
  the shared `CARGO_TARGET_DIR` appears in both the export instruction and is
  **absent** from any `rm -rf` cleanup path. Paste the greps in the PR body.
- Dry-run trace (no live workers touched): show the cleanup block removes the
  worktree but not `$CARGO_TARGET_DIR`; show the size-guard only prunes when no
  `.cargo-lock` is held.
- Do NOT reload the supervisor or kill workers as part of this brief.

## Drift / safe-git rules (inherited)

- Grep before adding any new env-var name; reuse `CARGO_TARGET_DIR`.
- `git add` explicit files only (never `-A`); rebase-before-push; never
  `git reset` the shared main checkout.
- Update this brief's BUILD_STATUS.md row to IN_REVIEW with the PR number in the
  same push that opens the PR. No FEATURE_MATRIX change (no capability ships).
