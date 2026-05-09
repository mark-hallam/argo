# CI Fallback Runbook

**State as of 2026-05-09**: GitHub Actions workflows are intentionally
disabled by the operator (all 5 workflow files renamed to `.yml.disabled`
in `.github/workflows/`). Local verification is the canonical trust
signal. CI on every PR is **expected** to be absent — not red, not
broken, just not running.

This runbook documents:
1. What "disabled" means and why CI may be re-enabled later
2. What local verification proves and does not prove
3. The local-CI-check fallback script (`scripts/local-ci-check.sh`)
4. The trust ladder while CI is disabled
5. How to re-enable CI when the time comes

## Why CI is disabled

GitHub Actions billing was burning credit on every PR run. Until the
billing posture changes (paid plan upgrade, self-hosted runner, or
selective re-enabling of the cheapest jobs only), the operator has
disabled all CI workflows entirely. This is a deliberate cost-control
measure, not a degraded state.

The disabled workflows:
- `ci.yml.disabled` — the main fmt + clippy + test + budget gauntlet
- `docs.yml.disabled` — mdbook site build + gh-pages deploy
- `oci-smoke.yml.disabled` — container build + smoke
- `release.yml.disabled` — tagged-release artifact builds
- `schema-doc-check.yml.disabled` — store-schema doc consistency

Operator can re-enable any of these individually by renaming back to
`.yml` (see "Re-enabling CI" below).

## What local verification proves

`scripts/local-ci-check.sh` runs the same gauntlet the disabled
`ci.yml` would, in the same order, with the same gates. On the
operator's machine (macOS arm64) it covers:

- `cargo fmt --check` — formatting matches workspace style
- `cargo clippy --workspace --all-targets -- -D warnings` — workspace
  lint is clean (catches cross-crate drift the per-crate clippy misses)
- `cargo test --workspace` — full workspace test suite passes (skipped
  in `--fast` mode)
- `cargo deny check` — supply-chain audit (advisories + licenses) clean
- `scripts/check-prepared-statements.sh` — argo-store query layer passes
- `scripts/check-span-naming.sh` — tracing span naming consistency
- File-LOC budget — no source file > 500 LOC
- Function-LOC budget — no fn > 80 LOC

`--pr <num>` mode auto-fetches a PR branch into a temp worktree and
runs the full gauntlet on it without disturbing the main checkout.

## What local verification does NOT prove

Local verification on macOS arm64 does **not** catch:
- Linux-only behavioural differences (stat semantics, syscall ordering,
  inotify vs. kqueue, glibc-vs-musl)
- Windows-only path/locale issues
- Race conditions exposed only on the GitHub runner topology
- Vendored dependency CVEs (cargo deny only fails on the bundled
  advisory db at run time — db itself may be stale on a slow update)

We accept these gaps in exchange for the cost reduction. When CI is
re-enabled, the first action should be a full workspace regression
sweep on a representative cross-section of recent merges.

## Trust ladder

| Verification | Trust level | Acceptable for |
|--------------|-------------|----------------|
| Worker self-report (PR body) only | Low | Diff-shape clean refactor, docs/scripts only, no Rust changes |
| Conductor crate-scoped clippy + tests | Medium | Single-crate handler / CLI / migration |
| Worker-side `workspace-clippy-fast-fail.sh` clean | Medium-high | Cross-crate drift caught (the post-#965 evals-field pattern) |
| Full `local-ci-check.sh` clean (one machine) | High | Multi-crate, gateway, store schema, anything user-facing |
| `local-ci-check.sh` clean on **two independent machines** | Highest | Aspirational substitute for the disabled CI |

In practice we have one operator machine, so the highest trust level
is "full local-ci-check.sh on the operator machine + worker-side
workspace-clippy on the worker machine". This is the closest substitute
to two-runner CI while workflows are disabled.

## When to escalate

Escalate to "do not merge until CI is re-enabled" when:
- The PR adds first-time use of an external service (new MCP connector,
  new gateway route, new audit kind)
- The PR changes migration ordering (raw + versioned consistency)
- The PR touches the workspace root `Cargo.toml` (workspace deps,
  resolver version, profile flags)

For these, post a Conductor Tier 1 hold on the PR explaining the
escalation and queue the merge for the next CI re-enabling window.
The diff is too consequential for "two independent local
verifications" to substitute.

## Re-enabling CI

When the billing posture allows (paid plan upgrade, self-hosted runner
ready, or willingness to selectively re-enable):

### Re-enable everything (full restore)

```bash
cd .github/workflows
for f in *.yml.disabled; do mv "$f" "${f%.disabled}"; done
git add .github/workflows
git commit -m "ci: re-enable workflows ($(ls *.yml | wc -l | xargs) files)"
git push
```

The next push to a tracked branch will fire all workflows. Watch
billing carefully for the first few runs.

### Re-enable selectively (cost-controlled)

Each workflow has different cost characteristics. A reasonable phased
restore:

1. **First**: `schema-doc-check.yml.disabled` — small, fast, low cost.
   Verifies store-schema docs stay in sync. Re-enable to restore
   the cheapest signal.
2. **Second**: `ci.yml.disabled` — the main gauntlet. Highest signal
   value but also highest cost. Consider trimming the test matrix
   (e.g., drop windows-latest temporarily) to reduce burn.
3. **Third**: `oci-smoke.yml.disabled` — container build. Useful but
   not load-bearing for code correctness. Defer until paid plan or
   self-hosted runner is in place.
4. **Fourth/last**: `docs.yml.disabled` and `release.yml.disabled` —
   only matter when actively shipping a tag. Keep disabled until
   release cadence justifies the cost.

### Self-hosted runner (no GitHub-side burn)

If GitHub billing remains a constraint long-term, register a runner
on the operator's machine (mac-host) and route workflows to
`runs-on: [self-hosted, macos]`. Steps:

1. Register: `gh actions-runner add` per GitHub docs (operator action,
   one-time)
2. Add `runs-on: [self-hosted, macos]` to one workflow as a pilot
   (e.g., the smallest, `schema-doc-check.yml.disabled`)
3. Verify it runs without GitHub-side burn
4. Migrate remaining workflows incrementally
5. Branch protections updated to require either GitHub-hosted OR
   self-hosted CI (not both, to avoid double-charge if the user later
   re-enables GitHub-hosted)

Not shipping the self-hosted workflow file tonight — requires operator
action to register the runner first. When ready, the new workflow
mirrors the existing `.github/workflows/ci.yml.disabled` job graph
with the runs-on target swapped.

## Audit recommendation when CI returns

When CI is re-enabled (in any form), run a one-time backfill audit on
the most recent 20 merges to catch cross-platform regressions that
local-on-macOS verification missed:

```bash
# After re-enabling ci.yml:
for sha in $(git log --merges --format=%H -n 20); do
    pr=$(git log -1 --format=%s "$sha" | grep -oE '#[0-9]+' | tr -d '#' | head -1)
    [[ -n "$pr" ]] && echo "PR #$pr ($sha): re-trigger via gh workflow run"
done
```

Any failure on a merged PR is structural — file an issue with the
specific cross-platform delta and the merge SHA, then queue a
follow-up brief to fix.
