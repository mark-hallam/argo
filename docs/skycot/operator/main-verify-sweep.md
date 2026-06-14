# main-verify sweep — operator runbook

**What it is (plain English):** a background watchdog that re-checks the `main`
code branch *after* each change lands and shouts (and blocks further merges) if
`main` is broken. We run it because our automatic test robot (GitHub CI) is
turned off on purpose — so without this, a good fix can be silently undone by a
later change and nobody notices (that's [DEC-055](../decisions/DEC-055-ci-off-local-post-merge-main-verify-sweep.md);
it actually happened to fix #2936 on 2026-06-14).

It is **safe-by-design**: it only ever *reads* `main`, builds in its own private
copy, and never touches the workers' working folder. The only thing it changes is
a small flag file under `~/.argo/` that tells the merge process "stop, main is
red."

## Install / update (one command)

```bash
bash scripts/main-verify-install.sh
```

This copies the daemon to a **stable** runtime dir (`~/.argo/main-verify/bin`) and
the plist to `~/Library/LaunchAgents`, pulling both from `origin/main` (so it works
even if your working tree is behind), then loads it. It starts immediately and
re-runs on every login. **First run does a cold compile (~15 min)** in its own
build folder; every run after is incremental and fast.

Why a stable copy rather than running from the repo: the daemon guards `main`, and
the workers' shared checkout is volatile (reset/dirty), so the daemon must not
depend on it for its own scripts. It verifies from its own freshly-synced worktree.
Re-run the install command after pulling a new version of the daemon to update it.

## Check status anytime

```bash
bash scripts/main-verify-status.sh
```

Prints e.g. `main-verify: GREEN @ origin/main 20d18de3d (checked ... UTC) freeze=off`.
`state` is GREEN (main is healthy) or RED (main is broken). `freeze=ON` means the
merge process is blocked until main goes green again.

## When it goes RED (what to do)

1. You'll see it in the conductor's next tick (a `[main-verify-RED]` line) and in
   `bash scripts/main-verify-status.sh` (it lists the failing test(s)).
2. **The conductor automatically stops merging** — `conductor-pre-merge-gate.sh`
   refuses while frozen. You don't have to do anything to stop the bleeding.
3. Fix forward: the failing test names point at what regressed. Restore/repair on
   `main` (usually a clobbered fix — re-apply it, like the #2936 restore).
4. The sweep re-verifies the next time `main` advances; on GREEN it **clears the
   freeze automatically** and merging resumes. (To force an immediate re-check
   without waiting: `launchctl kickstart -k gui/$(id -u)/com.skycot.main-verify`.)

## Pause / stop / restart

```bash
touch ~/.argo/main-verify-stop          # graceful pause (the loop survives, just idles)
rm ~/.argo/main-verify-stop             # resume
launchctl kickstart -k gui/$(id -u)/com.skycot.main-verify   # restart after editing the script
launchctl unload ~/Library/LaunchAgents/com.skycot.main-verify.plist   # hard stop
```

It also respects the swarm's own stop sentinels (`/tmp/argo-stop`, `~/.argo/stop`)
— pausing the swarm pauses the sweep too.

## What it actually runs

On each `origin/main` advance, in a private worktree (`~/.cache/argo-main-verify-wt`,
dedicated target dir, shared `sccache`):

1. **Compile guard** — `cargo check --workspace --tests` (catches a merge that
   doesn't build).
2. **Critical verifiers** — `scripts/main-verify-critical-tests.txt`: the #2936
   convergence canary, the mission-claim LIVE regressions, the 17 public-API
   `surface_contract` pins. **A verifier that has gone missing counts as RED**
   (a deleted canary must not pass).

To add a guard: add a `cargo test` line to `main-verify-critical-tests.txt`. To
tolerate a known flaky test: add its name to `main-verify-flake-allowlist.txt`
(keep that list short — every entry is a hole in the guard).

## Files

| Path | Role |
|---|---|
| `scripts/main-verify-sweep.sh` | the daemon (loop) |
| `scripts/main-verify-suite.sh` | the critical-path check (compile + verifiers) |
| `scripts/main-verify-critical-tests.txt` | the curated verifier list |
| `scripts/main-verify-flake-allowlist.txt` | tolerated known flakes |
| `scripts/main-verify-status.sh` | read the current state |
| `scripts/main-verify-freeze-check.sh` | the conductor's pre-merge gate calls this |
| `deploy/launchd/com.skycot.main-verify.plist` | the launchd job (copy to `~/Library/LaunchAgents/`) |
| `~/.argo/main-verify/merge-freeze` | the freeze flag (present = main RED) |
| `~/.argo/main-verify/status.json` | full state |
| `~/.argo/logs/main-verify.launchd.log` | the daemon's log |
