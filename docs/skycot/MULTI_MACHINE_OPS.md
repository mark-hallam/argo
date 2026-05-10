# SKYCOT — Multi-machine swarm operations

**Audience:** the operator running Skycot's worker swarm across
multiple machines (Mac, PC laptop, Linux box, etc.) with separate
Anthropic accounts to ride out 5h / weekly quota windows.

**Last updated:** 2026-05-10 (b21078ad).

---

## The model in 30 seconds

- **Conductor**: singleton. Exactly ONE conductor running at any time
  across ALL machines. Conductor merges PRs, flips ledger rows,
  authors briefs, demotes zombies. Two conductors WILL corrupt the
  ledger via race-on-flip.
- **Workers**: N parallel, machine-agnostic. Workers claim TODO
  briefs, implement, push PRs. They coordinate ONLY via:
  1. GitHub `origin/main` (claim push, PR push, race-aware via
     `scripts/claim-with-retry.sh`)
  2. The ledger `docs/build-plan/BUILD_STATUS.md` (TODO → CLAIMED
     → IN_REVIEW → MERGED state)
- **Quota windows**: each Anthropic account has its own 5h + weekly
  bucket. Run different accounts on different machines to keep
  total throughput steady when one account hits its cap.

## Machine layout (recommended)

```
Machine A (Mac, account 1)        Machine B (Mac, account 2)        Machine C (PC, account 3)
─────────────────────────         ─────────────────────────         ─────────────────────────
1× Conductor (singleton)          0× Conductor                       0× Conductor
2-3× Worker                       2-3× Worker                        2-3× Worker
```

Conductor location is your choice — pick the machine you'll babysit
most actively. **Never run two conductors.** If you migrate the
conductor between machines: stop the old one cleanly, ensure
`git status` is clean, then start on the new machine.

---

## Per-machine setup checklist

Run on each machine before starting any worker / conductor.

### Machine prerequisites

| Requirement | macOS install | Linux install (incl. WSL2) | Windows native |
|---|---|---|---|
| Rust 1.75+ | `brew install rustup-init && rustup-init` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | use WSL2 (Windows native is not supported) |
| `gh` CLI | `brew install gh` | `sudo apt install gh` or [docs.github.com/gh](https://cli.github.com/) | use WSL2 |
| `git` | preinstalled | `sudo apt install git` | use WSL2 |
| `claude` CLI | [download from Anthropic](https://docs.anthropic.com/en/docs/claude-code) | same | same |
| `python3` | preinstalled | `sudo apt install python3` | same |
| `sccache` (optional) | `brew install sccache` | `cargo install sccache` | same |

### Per-machine env (one-time)

**Where to put these vars matters.** Claude Code's Bash tool runs commands
in non-interactive subshells, which do **not** source `~/.zshrc` or
`~/.bashrc`. If you persist swarm env vars there, swarm tooling (worker
loop, conductor tick, helper scripts invoked from a Claude Code session)
will see them as unset. Use the file each shell sources for
non-interactive sessions:

- **macOS (zsh, the default):** put non-secret vars in `~/.zshenv`. zsh
  sources `~/.zshenv` for every invocation, including non-interactive
  ones; `~/.zshrc` is interactive-only.
- **Linux / WSL2 (bash):** put non-secret vars in `~/.profile` (sourced
  by login shells and most launchers), or set `BASH_ENV=~/.bash_env`
  globally and put exports in `~/.bash_env`. `~/.bashrc` is
  interactive-only and won't be seen by Bash tool subshells.
- **`ANTHROPIC_API_KEY` is a secret** — prefer `direnv` (with a
  gitignored `.envrc` in `$ARGO_REPO_ROOT`) or a sourced gitignored
  `~/.argo-env` file. Avoid persisting raw secrets in `~/.zshenv` /
  `~/.profile` if you can; those files often get backed up, synced, or
  accidentally screen-shared.

Add to `~/.zshenv` (macOS) or `~/.profile` (Linux/WSL2):

```bash
# Skycot/Argo repo location — derived at runtime, NOT hardcoded
export ARGO_REPO_ROOT="$HOME/code/argo"   # adjust to wherever you cloned

# Cargo target dir — shared across worktrees on THIS machine; speeds up incremental builds 5-10×
export CARGO_TARGET_DIR=/tmp/argo-shared-target

# Optional: sccache for cross-tab build cache sharing
export RUSTC_WRAPPER=sccache
```

For the Anthropic API key, recommended pattern is direnv:

```bash
# In $ARGO_REPO_ROOT/.envrc  (gitignored — verify it's in .gitignore)
export ANTHROPIC_API_KEY='sk-ant-...'
```

Then `direnv allow` once. Or, if you don't use direnv, source a
gitignored `~/.argo-env` from a wrapper script before launching workers.

Then reload: open a new terminal, or `source ~/.zshenv` / `source
~/.profile`. Verify with `env | grep ARGO_REPO_ROOT` from inside Claude
Code's Bash tool, not just your interactive shell.

#### Common pitfalls

- **Set vars in `~/.zshrc`, swarm scripts can't see them.** Symptom:
  `bootstrap-machine.sh` reports `ANTHROPIC_API_KEY not set` even though
  `echo $ANTHROPIC_API_KEY` works in your terminal. Fix: move the export
  to `~/.zshenv`.
- **`source ~/.zshenv` in your current shell ≠ persisted.** A new Claude
  Code Bash tool subshell starts fresh; if the export isn't in a file
  zsh re-reads on every spawn (i.e. `~/.zshenv`), it won't be there.
- **Linux+systemd launchers** may not source `~/.profile` either; use a
  user-level systemd `Environment=` directive or wrap your launcher.

### Repo bootstrap

```bash
# Clone if not already present
mkdir -p "$(dirname "$ARGO_REPO_ROOT")"
cd "$(dirname "$ARGO_REPO_ROOT")"
git clone https://github.com/mark-hallam/Argo-Agent.git "$(basename "$ARGO_REPO_ROOT")"
cd "$ARGO_REPO_ROOT"

# Authenticate gh CLI (browser flow)
gh auth login

# Verify
git pull --ff-only origin main
gh pr list --limit 3
cargo --version
claude --version
```

---

## Bootstrap script — `scripts/bootstrap-machine.sh`

A small script that validates the env on each machine and prints a
ready-to-run launch command. Lives in the repo so all machines pick
it up after `git pull`.

```bash
bash $ARGO_REPO_ROOT/scripts/bootstrap-machine.sh
```

The script checks: rust toolchain, gh auth, repo cleanliness,
ANTHROPIC_API_KEY set, `claude` reachable, disk free. Exits non-zero
if anything's missing.

---

## Things to watch for in multi-machine ops

### 1. Pulse files don't cross machines (already mitigated)

`/tmp/argo-worker-pulse/<brief-id>` is local per machine. Conductor's
zombie-claim sweep uses **ledger row age** (>2h since `claimed_at`)
as the primary signal, not pulse presence. Pulse-presence is now
informational only. See `docs/build-plan/BUILD_STATUS.md` header
"CONDUCTOR HARD RULE — ZOMBIE-CLAIM SWEEP".

### 2. Conductor singleton enforcement

There is no script-level enforcement (yet). It's an operator
discipline rule. If you accidentally start two conductors:
- Both will pull origin, both will see same TODO list
- They'll race on `git push` for ledger flips
- One will hit `git push --force-with-lease` failures and recover
- But during the race, rows may transit through inconsistent states

**To migrate the conductor between machines safely:**

```bash
# On old conductor machine:
cd "$ARGO_REPO_ROOT"
git status                          # MUST be clean (no uncommitted edits)
git log --oneline origin/main..HEAD # MUST show no unpushed commits
# stop the conductor terminal (Ctrl-C the cron tab or kill the loop)

# On new conductor machine:
cd "$ARGO_REPO_ROOT"
git pull --ff-only origin main
# start your conductor cron / loop on this machine
```

### 3. Claim races between machines

`scripts/claim-with-retry.sh` already handles this via git's
fast-forward race semantics. When two workers on different machines
try to claim the same brief simultaneously:
- Both edit BUILD_STATUS.md locally
- Both `git push`; first wins, second gets non-fast-forward
- claim-with-retry retries up to 3× with jitter
- If still racing, worker switches to a different TODO

This works correctly across machines. No coordination needed beyond
the existing script.

### 4. Branch / PR creation race

If both machines somehow finish a brief in parallel:
- First push of `<branch>` to remote succeeds
- Second push fails (branch exists)
- Worker should detect, release claim, move on

**Verify in `scripts/claim-with-retry.sh`** that branch-already-exists
is treated as race-loss, not a hard error. If it isn't, add a check.

### 5. Disk per machine

Each machine fills its own `/tmp/argo-shared-target` (~20-30GB after
cold builds) plus `/tmp/<brief-id>` worktrees (~10-20GB each). Run the
worktree-cleanup pattern periodically per-machine:

```bash
# Show all worktrees
git worktree list

# Find MERGED worktrees that are safe to remove
git worktree list | awk '$1 ~ /^\/tmp\// {print $1}' | while read wt; do
  brief=$(basename "$wt")
  status=$(grep "^| $brief " "$ARGO_REPO_ROOT/docs/build-plan/BUILD_STATUS.md" | awk -F'|' '{print $4}' | tr -d ' ' | head -1)
  if [ "$status" = "MERGED" ]; then
    echo "git worktree remove --force '$wt'"
  fi
done
# Review output, then pipe to bash if it looks right
```

### 6. Cross-platform clippy parity

Different OSes / rustc versions occasionally surface different
clippy lints. Worker self-verification on PC may pass while the
conductor's local re-check on Mac fails (or vice versa). Since CI
is currently OFF (GitHub Actions billing), we're flying blind on
cross-platform parity.

**Mitigation:** the conductor merges based on worker self-verification
+ Tier 1 diff-shape. If a merge introduces a regression on a
different platform, the next worker to do `cargo clippy` on that
platform will hit it and either fix in their PR or escalate.

### 7. Rust version drift

Different machines may have different `rustup` channels. Pin to a
single version per workspace via `rust-toolchain.toml` if drift
becomes an issue. Currently the workspace uses Rust 1.75
compatibility (per CLAUDE.md invariants).

### 8. Quota strategy

- **Mac A account hits 5h limit** → workers idle for 5h. Worktree state
  + claims survive. When quota resets, workers auto-resume (per the
  worker-loop's quota-detect logic).

### 9. Path portability

The repo doesn't hardcode `~/code/argo` paths in
scripts (it uses `git rev-parse --show-toplevel` and relative paths
from there). All paths in worker briefs are repo-relative
(`crates/argo-cli/...`). Worktrees use `/tmp/<brief-id>` which is
portable across Unix.

**The one fragile spot:** the auto-launch / cron setup on each
machine needs to know `ARGO_REPO_ROOT`. Set it as env var in your
shell rc on each machine; never hardcode.

### 10. Branch-name length

GitHub allows ≤256 chars per branch. Some Skycot brief IDs get long
(e.g. `claude-cli-tests-vertical-gtm-engineer-assets-split` →
branch becomes `claude-claude-cli-tests-vertical-gtm-engineer-assets-split`
due to double-prefix). Check branch length before push if you see
errors.

---

## Operating modes

### Single-machine, single-account (current default)

Mac A only. Conductor + 4-6 workers. When quota hits, swarm pauses
for 5h. This is what's been running since Phase A.



---

## Common errors & fixes

### "Your local changes to the following files would be overwritten by merge"

Usually `docs/build-plan/BUILD_STATUS.md` got dirtied by a previous
worker's incomplete iteration. Fix:

```bash
git stash push -u -m "stale-wip-$(date +%s)"  # preserve in case it's salvageable
git pull --ff-only origin main
# inspect stash if needed: git stash show -p stash@{0}
git stash drop stash@{0}                       # if not salvageable
```

### Worktree path collision: "fatal: '/tmp/<brief>' is already used by worktree at..."

```bash
git worktree remove --force /tmp/<brief>
git worktree prune
git worktree add /tmp/<brief> origin/main
```

### "Cannot fast-forward to multiple branches"

Cosmetic git warning, usually safe to ignore. Confirm with:

```bash
git status -sb       # should show "## main...origin/main" (no ahead/behind)
git pull --ff-only origin main
```

### Cargo build cache poisoned across machines

If you copy `/tmp/argo-shared-target` between machines (don't do
this), incremental builds may produce wrong artifacts. Each machine
should have its own target dir.

---

## Glossary

- **brief**: a markdown file in `docs/build-plan/streams/<brief-id>.md`
  describing one unit of work. Worker reads, implements, opens PR.
- **ledger**: `docs/build-plan/BUILD_STATUS.md` — one-row-per-brief
  state machine.
- **worktree**: `git worktree add` checkout in `/tmp/<brief-id>` so a
  worker can build/test in isolation without polluting the main
  checkout.
- **claim**: worker flips ledger row TODO → CLAIMED via
  `claim-with-retry.sh`. Race-aware.
- **pulse**: heartbeat file at `/tmp/argo-worker-pulse/<brief-id>`,
  touched every 30s by an alive worker. Local-only signal.
- **quota event**: Anthropic 5h limit hit. Worker-loop catches this
  and gracefully releases claim back to TODO.
