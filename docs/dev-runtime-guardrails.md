# Dev runtime guardrails

This is the safety system for long autonomous coding sessions
against the Argo repo. Autonomous build loops fill disk via
`target/`, `/private/tmp/argo*`, `/private/tmp/claude-*`, and
`.skycot/outputs/`. Without guardrails, a few hours of agent
activity can brick the dev machine.

The guardrails are 3 shell scripts + this doc + a test script.
Nothing modifies product runtime behaviour. Nothing requires
new Rust state or migrations.

## Commands

| Command                                  | When to run                                        |
|------------------------------------------|----------------------------------------------------|
| `./scripts/dev-safe-check.sh`            | Right now: see disk + artifact sizes.              |
| `./scripts/dev-safe-clean.sh`            | Dry-run: list what would be deleted.               |
| `./scripts/dev-safe-clean.sh --apply`    | Actually delete. Always after a dry-run review.    |
| `./scripts/dev-safe-test.sh`             | Pre-flight before any autonomous coding session.   |
| `./scripts/dev-safe-test.sh --apply-cleanup-on-fail` | Same, but auto-clean if disk gate fails. |
| `./scripts/tests/dev-safe-clean.test.sh` | Verify the cleanup script's safety properties.     |

## Run before every autonomous coding session

```
./scripts/dev-safe-test.sh
```

This runs:

1. **Disk health snapshot** — `dev-safe-check.sh`.
2. **Free-space gate** — fails if free disk < 50 GiB (or your
   `--threshold-gib` / `ARGO_DEV_FREE_GIB` override). Prints
   the cleanup recommendation.
3. **`cargo fmt --all -- --check`** — workspace formatting.
4. **`cargo clippy --workspace --all-targets -- -D warnings`** —
   warnings-as-errors lint.
5. **`cargo test --workspace`** — full workspace test pass.

Exit codes are unique per failing step (1=disk, 2=fmt, 3=clippy,
4=tests, 5=missing prereq) so CI / wrapper scripts can switch
on them.

## Run during a session if disk feels tight

```
./scripts/dev-safe-check.sh                # quick read-only snapshot
./scripts/dev-safe-clean.sh                # dry-run; review the list
./scripts/dev-safe-clean.sh --apply        # actually delete
```

## Recovery procedure if disk filled mid-session

When `/private/tmp` fills the Bash tool starts erroring with
`ENOSPC: no space left on device, open '/private/tmp/...'`. The
Bash tool itself can't run — even `df` fails to allocate its
output file. **Step out of Claude Code** to a regular terminal:

```
# 1. Confirm the situation
df -h /private/tmp
df -h /

# 2. Free /private/tmp first — the agent runner uses it for every
#    command's output file. Mostly safe; these are scratch files.
rm -rf /private/tmp/claude-* /private/tmp/argo*

# 3. If still tight, free target/ in the repo
cd ~/code/argo
rm -rf target/

# 4. Verify
df -h /private/tmp /
ls /private/tmp | head

# 5. Resume the Claude session — Bash should now work.
```

Once Bash works again, run the standard cleanup:

```
./scripts/dev-safe-clean.sh           # dry-run to see what's left
./scripts/dev-safe-clean.sh --apply
```

## Retention policy for `.skycot/outputs/`

Per-job worker output bundles accumulate. The cleanup script
applies a two-axis policy:

* **Age cutoff** — anything older than `--max-age-days` (default
  **14 days**) is eligible for deletion.
* **Recency floor** — the newest `--keep-recent` (default **20**)
  bundles are ALWAYS kept, even if they're older than the age
  cutoff. This protects history that's still being reviewed
  during a bursty week.

A bundle is deleted only when it's both `> max-age-days` old AND
outside the newest-`keep-recent` window. The script never
deletes:

* The current in-progress job's output (it has `mtime ≈ now`,
  comfortably inside the recency floor).
* The memory DB (`~/.argo/argo.db`).
* The workspace's `argo.toml`.
* Source files, `.env`, configs, secrets — anything outside
  the four whitelisted prefixes.

## What the cleanup script touches (the whitelist)

Every path passes through a strict whitelist check
(`is_whitelisted`) before any `rm -rf` runs. The four
allowed prefixes:

| Prefix                          | What it is                                             |
|---------------------------------|--------------------------------------------------------|
| `<repo>/target`                 | Cargo build cache. Wholesale-deletable; rebuilds.      |
| `<repo>/.skycot/outputs/*`      | Per-job artifact bundles. Subject to retention policy. |
| `/private/tmp/argo*`            | Argo-side scratch tempdirs. Wholesale-deletable.       |
| `/private/tmp/claude-*`         | Claude Code runner tempfiles. Wholesale-deletable.     |

Anything outside these prefixes is REFUSED with a clear error.
The whitelist also rejects paths containing `..` traversal
markers and empty / unset variables — defence in depth even
though `realpath` would handle the former.

The whitelist is locked in regression by
`scripts/tests/dev-safe-clean.test.sh` — 31 assertions
covering dry-run-doesn't-delete, --apply-deletes-only-allowed,
unknown-flags-rejected, retention-keeps-recent + within-age,
--skip-target-honoured, traversal-refused, empty-path-refused.

## Safety constraints

These are the script's invariants. The reviewer will reject any
change that breaks them.

1. **No destructive command without `--apply`.** Default is
   dry-run; `--apply` is always required to actually delete.
2. **No `rm -rf` on interpolated empty variables.** Every
   `delete_path` call checks `[ -z "$path" ]` first.
3. **Explicit path checks via `is_whitelisted`.** Any path
   outside the four prefixes above triggers `FAIL: refused`.
4. **No user files touched.** No `~/.zshrc`, `.env`, configs,
   or repo source files are reachable via the whitelist.
5. **No DB / workspace config touched.** Argo home
   (`~/.argo/argo.db`, secrets) and workspace `argo.toml` are
   outside the whitelist.
6. **Files stay under 500 LOC.** Per the project-wide rule.

## CI integration

For CI runs that want the same gate without a human in the
loop:

```yaml
# .github/workflows/dev-safe.yml (sketch)
- name: Pre-flight + lint + test
  run: |
    ./scripts/dev-safe-test.sh \
      --threshold-gib 30 \
      --apply-cleanup-on-fail
```

The `--apply-cleanup-on-fail` flag auto-runs the cleanup if the
disk gate fails. CI runners are ephemeral so this is safe; for
dev machines the operator should review first.

## When to bump retention defaults

| Situation                                          | Suggested change                  |
|----------------------------------------------------|-----------------------------------|
| Long autonomous build session expected (8h+)       | `--keep-recent 50 --max-age-days 7`  |
| Bursty CI investigating a regression               | `--keep-recent 100`               |
| Disk tight on a small dev box                      | `--max-age-days 3 --keep-recent 10`  |
| Archive run before deleting (out of scope today)   | TODO: `--archive-to <path>`       |
