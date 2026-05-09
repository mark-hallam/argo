# CI Fallback Runbook

**State as of 2026-05-09**: GitHub Actions CI is failing across all PRs
with `FAILURE` and `SKIPPED` statuses on every job — a billing or
account-level infrastructure issue (jobs do not even start, `gh run
view --log-failed` returns "log not found"). Local verification is
currently the only trust signal.

This runbook documents:
1. How to detect that CI is broken (not just one PR's tests failing)
2. What local verification proves and does not prove
3. The local-CI-check fallback script
4. When to trust local verification, when to escalate

## Detecting "CI is broken" vs "this PR has a bug"

**CI is broken** when:
- All jobs report `FAILURE` or `SKIPPED` simultaneously
- `gh run view <run-id> --log-failed` returns "log not found"
- The job's `steps` array is empty or null in the JSON view
- Multiple unrelated PRs show identical failure patterns

**This PR has a bug** when:
- Some jobs pass and others fail
- The failed jobs have non-empty `steps` arrays with actual error logs
- The failure pattern correlates with files this PR touched

When CI is broken, fall through to local verification per this runbook.
When this PR has a bug, do not merge — request changes.

## What local verification proves

`scripts/local-ci-check.sh` runs the same gauntlet GitHub Actions
would. On the conductor's machine (macOS arm64) it covers:

- `cargo fmt --check` — formatting matches workspace style
- `cargo clippy --workspace --all-targets -- -D warnings` — workspace
  lint is clean (catches cross-crate drift the per-crate clippy misses)
- `cargo test --workspace` — full workspace test suite passes
- `cargo deny check` — supply-chain audit (advisories + licenses) clean
- `scripts/check-prepared-statements.sh` — argo-store query layer passes
- File-LOC budget — no source file > 500 LOC
- Function-LOC budget — no fn > 80 LOC

## What local verification does NOT prove

Local verification on macOS arm64 does **not** catch:
- Linux-only behavioural differences (stat semantics, syscall ordering)
- Windows-only path/locale issues
- Race conditions exposed only on the GitHub runner topology
- Vendored dependency CVEs (cargo deny only fails on the bundled
  advisory db at run time — db itself may be stale)

A merge that passes local verification but fails on a healthy CI is
a cross-platform regression. We accept this risk while CI is broken;
when CI is restored, the first action is a full workspace regression
sweep to surface anything missed.

## Trust ladder while CI is broken

| Verification | Trust level | Acceptable for |
|--------------|-------------|----------------|
| Worker self-report (PR body) only | Low | Diff-shape clean refactor, docs/scripts only |
| Conductor crate-scoped clippy + tests | Medium | Single-crate handler / CLI / migration |
| Full `local-ci-check.sh` clean | High | Multi-crate, gateway, store schema, anything user-facing |
| `local-ci-check.sh` clean on **two independent machines** | Highest | Required equivalent for "Tier 2 verified" while CI is broken |

In practice we have one conductor machine, so "two independent
machines" is aspirational. The worker's local clippy + the conductor's
re-verification on the same artifact is the closest current substitute
and is acceptable for everything except gateway-boundary work.

## When to escalate

Escalate to "do not merge until CI is restored" when:
- The PR adds first-time use of an external service (new MCP connector,
  new gateway route, new audit kind)
- The PR changes migration ordering (raw + versioned consistency)
- The PR touches the workspace root `Cargo.toml` (workspace deps,
  resolver version, profile flags)

For these, post a Conductor Tier 1 hold on the PR explaining the
escalation and wait for CI to come back. The diff is too consequential
for "two independent local verifications" to substitute.

## Restoring CI

When the GitHub billing or infra issue is resolved:
1. Re-trigger the most recent failed workflow on a representative PR:
   `gh workflow run ci.yml --ref <branch>`
2. If it passes, run a backfill on all currently-IN_REVIEW PRs:
   `for pr in $(gh pr list --state open --json number --jq '.[].number'); do
      gh pr checks "$pr" >/dev/null 2>&1 || gh workflow run ci.yml --ref refs/pull/$pr/head;
    done`
3. Audit the most recent 20 merges for cross-platform regressions:
   pull each, run `local-ci-check.sh --linux-via-docker` (when that
   variant ships), report any breakage.

## Self-hosted runner (future)

If the GitHub billing remains unresolved long-term, the cleanest path
is a self-hosted runner. The user has a MacBook (mac-host) capable of
hosting one. The wiring:

1. Register a runner via `gh actions-runner add` on mac-host
2. Add a `.github/workflows/ci-self-hosted.yml` parallel workflow that
   runs on `[self-hosted, macos]` with the same job matrix
3. Branch protections updated to require either GitHub-hosted OR
   self-hosted CI (not both, to avoid double-charge)

Not shipping the self-hosted workflow tonight — requires user action
to register the runner. When ready, a one-liner workflow file mirrors
the existing `.github/workflows/ci.yml` job graph with the runs-on
target swapped.
