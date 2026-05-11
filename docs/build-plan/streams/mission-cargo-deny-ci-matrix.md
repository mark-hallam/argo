# Mission load-bearing wire — cargo-deny in CI matrix

**Owner**: claude. **Depends on**: existing `deny.toml` at repo root (verify via `ls deny.toml`); the `mission-cargo-audit-disabled-workflow` (#1325-merged) precedent for the `.disabled`-during-billing-freeze workflow pattern.
**Authored against**: origin/main `47749a1b` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cargo-deny-ci-matrix`.

> SOURCE: MISSION_BACKLOG.md Section 1 #13 ("Add cargo-deny to CI matrix (currently `workflow_dispatch:` only)"). **Tier M (security + supply-chain hygiene).** `cargo-deny` already exists in the repo (per `deny.toml`) but only runs when an operator manually triggers `workflow_dispatch`. That means PRs introducing a banned-license / yanked-crate / known-CVE dep ship silently. This brief moves cargo-deny from manual-trigger to schedule + push, with a documented allowlist mechanism for genuine-but-acceptable findings.

## Scope

NEW `.github/workflows/deny.yml.disabled` (≤ 100 LOC) — mirrors `mission-cargo-audit-disabled-workflow` (#1325) shape. Triggers:
- `on.pull_request` for `main`/`master` (every PR)
- `on.push` to `main` (every commit)
- `on.schedule: cron: '0 6 * * *'` (daily 6am UTC — catches newly-disclosed CVEs against unchanged deps)
- `on.workflow_dispatch` (manual fallback)

Run matrix: `cargo deny check {advisories,bans,licenses,sources}` — the 4 cargo-deny check kinds. Each runs independently so one failure doesn't mask the others. Aggregated job-level result fails the PR/commit if any check failed.

EXTEND existing `deny.toml` (verify shape at brief-claim time; the file likely has `[licenses]`, `[bans]`, `[advisories]`, `[sources]` sections):
- Add a `[advisories].ignore = []` allowlist for explicit acceptance — operators add `RUSTSEC-YYYY-NNNN` IDs here with a comment explaining the acceptance rationale. NO blanket ignores.
- Add `[bans].deny = []` for banned-crate-name pinning — currently empty placeholder; operators add as needed
- Add `[licenses].allow = []` with the documented operator-approved license set (verify what's currently in the file before extending — likely already populated)

NEW local-runnable script `scripts/cargo-deny.sh` (≤ 80 LOC) — installs cargo-deny if absent, runs the same 4 checks the CI workflow runs, prints a summary. Mirrors `scripts/cargo-audit.sh` from #1325. Operators run before push w/o waiting for CI.

NEW runbook `docs/skycot/runbook/cargo-deny.md` (≤ 200 LOC):
- Why cargo-deny exists (license / supply-chain / CVE / source-registry hygiene)
- How to interpret each check kind
- How to add an `[advisories].ignore` entry (process: PR comment from security lead + 2-business-day soak + add w/ link to discussion)
- How to add an `[licenses].allow` entry
- Operator-runnable command: `bash scripts/cargo-deny.sh`

This is NOT in scope: making cargo-deny block-by-default in the pre-commit hook (cargo-deny network-fetches the advisory DB; pre-commit should stay fast); per-crate license-exception tracking (operator adds `[licenses].clarify` if needed but not a default brief deliverable); the actual CI runner cost analysis for billing-restored (operator decision); cargo-vet or cargo-supply-chain (separate tier-Z brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `.github/workflows/deny.yml.disabled` (NEW, ≤ 100 LOC)
- `scripts/cargo-deny.sh` (NEW, ≤ 80 LOC, mode 755)
- `docs/skycot/runbook/cargo-deny.md` (NEW, ≤ 200 LOC)
- `deny.toml` — narrowly scoped: EXTEND with `[advisories].ignore` / `[bans].deny` / `[licenses].allow` allowlist scaffolding (do NOT remove existing entries; only ADD)
- `scripts/precommit-cargo-deny.sh` (NEW OPTIONAL, ≤ 60 LOC) — opt-in pre-push hook that operators can wire via `git config core.hooksPath`. Skips by default if cargo-deny isn't installed (don't fail the push just because a dev hasn't installed cargo-deny yet).

## Files you must NOT touch

- Workspace `Cargo.toml` / member `Cargo.toml`s — cargo-deny is a CI tool, not a workspace member dep
- `crates/argo-store/migrations/**`
- Source code / vertical / channels

## Acceptance criteria

- [ ] `bash scripts/cargo-deny.sh` runs successfully on a clean checkout
- [ ] Script exits 0 when all 4 checks pass; exits non-zero w/ clear summary when any fail
- [ ] `.github/workflows/deny.yml.disabled` is valid GitHub Actions YAML (lint via `actionlint` if available)
- [ ] Workflow triggers documented in PR body: PR-to-main + push-to-main + daily-cron + manual
- [ ] `deny.toml` extended w/ commented-out `[advisories].ignore = []`, `[bans].deny = []`, `[licenses].allow = []` scaffolding; existing entries preserved
- [ ] Runbook explicitly documents the "add advisories.ignore entry" process (security review + 2-day soak + link)
- [ ] Runbook covers all 4 check kinds (advisories, bans, licenses, sources)
- [ ] No Rust changes; clippy/test not relevant
- [ ] `bash -n scripts/cargo-deny.sh` (syntax check) clean
- [ ] `shellcheck scripts/cargo-deny.sh` (if installed) emits no `error:`-level findings
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cargo-deny-ci-matrix`
- PR title: `security: cargo-deny CI workflow (disabled) + local-runnable script + runbook (mission-cargo-deny-ci-matrix)`

## Notes for the implementer

- Mirror the just-merged `mission-cargo-audit-disabled-workflow` (#1325) shape almost exactly — same `.yml.disabled` extension, same `scripts/cargo-<X>.sh` local-runner pattern, same runbook structure. Don't invent a new shape; consistency makes operator onboarding easier.
- `cargo-deny` is a SEPARATE tool from `cargo-audit` — `cargo-audit` checks advisories ONLY; `cargo-deny` checks 4 categories (advisories + bans + licenses + sources). Both belong in CI; this brief addresses the cargo-deny half.
- The 4 check kinds:
  - **advisories** — known CVEs in transitive deps (`RUSTSEC-*` IDs). Most likely to fire.
  - **bans** — explicitly forbidden crate names (e.g. `openssl` if the policy is "ring only"). Configured per-project.
  - **licenses** — license-compatibility check against `[licenses].allow`. SKYCOT is Apache-2.0/MIT; deps with GPL/AGPL/proprietary should fail.
  - **sources** — registry/git-source allowlist. Prevents accidental dep from a random git repo.
- The `[advisories].ignore` mechanism is necessary BUT DANGEROUS — every ignored CVE is a known risk you're accepting. The runbook's "add a CVE-ignore" process (security review + soak + link) is what keeps this honest.
- Local script should `cargo install --locked cargo-deny` if absent. Use `cargo-deny --version` as the existence check.
- DO NOT add cargo-deny to the default pre-push hook by symlink — only via opt-in `git config core.hooksPath`. Pre-push needs to stay fast; cargo-deny fetches the advisory DB on each run (cached but still ~5s on cold cache).
- The `.disabled` extension stays in place until GitHub Actions billing is restored, per the BUILD_STATUS header note. When restored, operators rename to `.yml` — same operator-action as #1325 expects.
