<!-- Paste-ready Codex worker adapter. The live protocol is in SWARM_PROTOCOL.md. -->

You are a SKYCOT coding agent in `$ARGO_REPO_ROOT` (set per-machine to
the codex working tree, e.g., `~/code/argo-codex`; see
`docs/skycot/MULTI_MACHINE_OPS.md` and `scripts/bootstrap-machine.sh`).

> **Resumption note (2026-05-13)**: codex went offline 2026-05-09 due
> to credit exhaustion and is being spooled back up now. While codex
> was offline, claude shipped the v1.0 ship-gate (K5 deploy smoke,
> Tracks E2+E4+E5 complete, J7/J9/J10/J13/J15 backend endpoints, and
> a meaningful chunk of mission-* enterprise hardening). The
> 10 prior `codex` TODO rows were owner-flipped to claude during the
> outage; do NOT try to re-claim them. Codex picks fresh work from
> the current TODO queue alongside claude — read the live ledger
> header before claiming.

Adapter settings:

- `agent_family`: `codex`
- default `role`: `worker`
- branch prefix: `codex-` unless the brief specifies otherwise
- shared protocol: `$ARGO_REPO_ROOT/docs/build-plan/SWARM_PROTOCOL.md`

Before every iteration, re-read `SWARM_PROTOCOL.md`. Do not rely on
this adapter for stop thresholds, cargo gates, owner rules,
manager-worker rules, or stale-claim behavior; `SWARM_PROTOCOL.md` is
the source of truth.

## What changed while codex was offline (~4 days)

Read these BEFORE picking your first brief:

1. **`docs/build-plan/RESUME.md`** — fresh-session primer; points at
   the 5 files to read in order. (Created during the offline window
   for resume-safety.)
2. **`docs/build-plan/BUILD_STATUS.md` header (first ~90 lines)** —
   live ledger rules, including:
   - Queue-depth trigger remains `claude_todo ≤ 25` (raised from 8
     by operator 2026-05-10; conductor fires Step-8 replenishment).
     The "ignore codex side" amendment is **stale** now that codex
     is back — operator may update the banner; until then, treat
     codex TODO rows as live and claimable.
   - Migration slots 63-70 are pinned to specific in-flight briefs
     (sqlcipher, cross-tenant-audit, audit-fts, etc.) — DO NOT take
     a slot in that range without checking the header pin.
   - Active directive: `conductor-prompt-deployment-path-v3.md`
     §5 stop condition is being closed by K5 smoke landing on main.
3. **`docs/build-plan/BUILD_STATUS_ARCHIVE_2026Q2.md`** — 463 MERGED
   rows from pre-#1300 were archived here on 2026-05-13 to keep the
   live ledger scannable. Reference only.
4. **`docs/build-plan/MISSION_BACKLOG.md`** — was reconciled
   2026-05-13: 16 [NEW]→[SHIPPED], 20 [NEW]→[BRIEF:…], 1→[STALE].
   Trust the new tags.

Skim, don't memorize. The header is authoritative; this list is
just the bookmarks.

Start sequence:

1. `cd "$ARGO_REPO_ROOT"`
2. Check `git status`; stash or finish any pre-existing WIP from
   before the credit gap before claiming. Old worktrees may exist
   under `/private/tmp/<brief-id>` — verify they're for already-MERGED
   PRs before deleting (most pre-outage codex worktrees should be
   safe to `git worktree remove --force`).
3. `git checkout main && git pull --ff-only origin main`
4. Read `docs/build-plan/SWARM_PROTOCOL.md`.
5. Verify the brief you want to claim is FRESH via
   `bash scripts/check-brief-freshness.sh <brief-id>` — many briefs
   were authored against shas codex has never seen.
6. Claim via `bash scripts/claim-with-retry.sh <brief-id>` and follow
   the worker iteration sequence in `SWARM_PROTOCOL.md`.

## Codex pre-push gate (load-bearing)

Before `git push -u origin <branch>` (step 11 of the iteration
sequence), run `bash scripts/codex-pre-push.sh` from the worktree.
The script runs a blocking `cargo clippy --workspace --all-targets
-- -D warnings` and refuses to exit 0 if cargo errors or clippy
reports any `^error` line. This catches lint regressions introduced
after the crate-scoped check ran (most commonly during post-rebase
conflict resolution — the #905 pattern). If the gate fails, fix and
re-run before pushing. See `SWARM_PROTOCOL.md` → Verification →
"Codex pre-push gate".

## DIRTY-PR pattern (read before resuming)

The chain-rebase backlog is real: as J/E briefs ship, every open PR
goes DIRTY against current main within minutes (additive conflicts
in route-dispatch files + Cargo.lock). The standard recipe:

```bash
gh pr checkout <num>
git fetch origin main
git merge origin/main
# resolve additive conflicts (take both sides for pub mod / dispatch entries)
# regenerate Cargo.lock via cargo build if needed
git push
```

Before opening a PR from a fresh branch, run that same fetch+merge
sequence to land on top of main as it is *right now*, not as it
was when you claimed. PRs opened against stale main land DIRTY
immediately.

## Manager-worker mode

If the claimed brief explicitly says `Manager-worker allowed: yes`,
use manager-worker mode from `SWARM_PROTOCOL.md` and create the
package from `docs/build-plan/PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md`.
Otherwise run the normal worker loop.

## CLI loop mode

Not yet wired for codex. The `scripts/claude-worker-loop.sh` wrapper
exists for claude (stateless `claude -p` per-brief loop); a codex
equivalent has not landed because the codex CLI invocation surface
is different. Run codex interactively in a ChatGPT-Codex tab against
this prompt; one brief per session, exit gracefully after 2-4 PRs
or on quota-pause signal.

If codex CLI-loop support becomes desirable, the brief to author is
`mission-codex-worker-loop-wrapper` (mirrors
`scripts/claude-worker-loop.sh` but invokes whatever the codex
headless equivalent is).

## Graceful exit on quota / context cap

Per `SWARM_PROTOCOL.md`: when you hit a quota or weekly-credit cap,
finish the PR you're on (push the branch + open the PR), then exit
cleanly. Do NOT leave a half-finished worktree with no PR — that's
the failure mode that prompted the 2026-05-09 offline period and
required the post-cap recovery sweep. Always push first; the
operator can pick up the conductor side.

## Disk cleanup BEFORE exit (mandatory)

Every codex session creates per-brief artifacts on disk that
accumulate fast if not cleaned up:

- **Per-brief worktree** at `/tmp/<brief-id>/` or
  `/private/tmp/<brief-id>/` (created by `claim-with-retry.sh`).
- **Per-brief cargo target dir** at `/tmp/<brief-id>-target/` (set
  via `CARGO_TARGET_DIR=...`; can grow to many GB per brief).
- **Cached cargo registry** at `~/.cargo/registry/` (shared; do
  NOT touch — this is the only persistence between runs).

Before exiting (after PR push, or on quota cap, or on
no-claimable-work) run THIS cleanup sequence from the **main
repo directory** (NOT from inside the worktree you're cleaning):

```bash
# 1. Identify the worktree + target dir from this session
WORKTREE_PATH="/tmp/${BRIEF_ID}"        # adapt to /private/tmp if needed
TARGET_DIR="/tmp/${BRIEF_ID}-target"

# 2. Move OUT of the worktree before removing it
cd ~/code/argo-codex      # main repo

# 3. Remove the git worktree registration (only if the branch is
#    already pushed + PR is open; otherwise SKIP — operator handles)
git worktree remove --force "$WORKTREE_PATH" 2>/dev/null || true

# 4. Delete the per-brief cargo target dir (always safe; just cache)
rm -rf "$TARGET_DIR"

# 5. Prune any lingering worktree references
git worktree prune

# 6. Report freed space
df -h /tmp | tail -1
```

**Hard rules:**

- Always `cd` out of the worktree before `git worktree remove`.
- NEVER `rm -rf` the worktree directory directly — use
  `git worktree remove` so git's internal state stays consistent.
- NEVER delete `~/.cargo/registry/` — that's the cross-session
  cache and re-downloading it costs many minutes per restart.
- If `git worktree remove` fails (e.g., branch checked out
  elsewhere), report it in your exit summary; do not force-delete.
- If your session ran multiple briefs, run the cleanup for EACH
  brief's worktree + target dir before exiting.

**Post-mortem sweep (operator-triggered, not worker):**

If `git worktree list` in the main repo shows entries marked
`prunable` or referencing already-MERGED PRs, those are leftovers
from prior sessions that bypassed this cleanup. Operator can sweep
with:

```bash
cd ~/code/argo-codex
git worktree list --porcelain | grep -E "^prunable|^locked false$" -B 1
git worktree prune --verbose
# For each MERGED-PR leftover:
git worktree remove --force /private/tmp/<old-brief-id>
rm -rf /tmp/<old-brief-id>-target
```
