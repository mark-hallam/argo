# Mission load-bearing wire — Nightly silent-skip integration tests

**Owner**: claude. **Depends on**: existing silent-skip pattern (verify count via `rg -c '\\bskip\\b.*api_key|skip-if-no-env' crates/`); just-merged `.disabled`-workflow precedent from `mission-cargo-audit-disabled-workflow` (#1325) and the queued `mission-criterion-load-tests` / `mission-soak-test-nightly` briefs (consistent pattern across the CI-billing-freeze period).
**Authored against**: origin/main `fef33298` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-silent-skip-nightly-integration`.

> SOURCE: MISSION_BACKLOG.md Section 1 #11 ("Add nightly job that runs all 65 silent-skip integration tests against real test accounts (with secrets in repo secrets)"). **Tier M (real-coverage).** Today ~65 integration tests are gated by env-var presence (`ARGO_TEST_*_KEY` or similar) and silently skip when the operator running `cargo test` doesn't have those keys. That means real-account coverage is operator-discretion; CI never exercises it. This brief lands the nightly runner that injects test-account secrets from CI environment and runs the full silent-skip suite once per day, surfacing failures via the existing channel routing.

## Scope

NEW `.github/workflows/silent-skip-nightly.yml.disabled` (≤ 120 LOC) — mirrors `mission-cargo-audit-disabled-workflow` (#1325) and the queued criterion/soak workflows. Triggers:
- `on.schedule: cron: '0 5 * * *'` (daily 5am UTC, between cargo-audit 6am and cargo-deny 6am to spread load)
- `on.workflow_dispatch` (manual fallback)

The workflow:
1. Injects test-account credentials from GitHub repo secrets (matrix: anthropic / openai / gemini / hubspot / slack / etc. — full list maintained in the workflow file itself, NOT in code).
2. Runs `bash scripts/silent-skip-nightly.sh` which discovers all silent-skip tests via the `ARGO_TEST_*` env-var convention and runs them with secrets present (so they no longer silently skip).
3. Posts the results-summary JSON as an artifact + comments on the most recent push commit if any test failed (using the existing argo-channels dispatch path if a channel is configured, else just GitHub-comment).

NEW discovery+runner script `scripts/silent-skip-nightly.sh` (≤ 200 LOC):
- Discover silent-skip tests via `cargo test -- --list 2>&1 | grep -E '(_smoke|_integration|_live)'` AND grep crate sources for `if std::env::var("ARGO_TEST_.*_KEY").is_err() { return; }` patterns
- Print discovered count (for sanity-check; brief targets ~65; deviation outside [40, 90] suggests drift)
- Run each test in its own `cargo test -p <crate> --test <name>` invocation so a single hang doesn't block the rest
- 5-minute per-test timeout (gracefully kill + record as TIMEOUT)
- Aggregate to `target/silent-skip-report-<ts>.json` w/ shape `{ started, completed, total, passed, failed, timeout, skipped, failures: [{test, crate, exit_code, stderr_tail}] }`
- Stderr summary (also printable to operator-side via `bash scripts/silent-skip-nightly.sh --local` for local debugging)

NEW operator runbook `docs/skycot/runbook/silent-skip-nightly.md` (≤ 240 LOC):
- What the silent-skip pattern is and why it exists
- How to add a new silent-skip test (env-var contract: `ARGO_TEST_<PROVIDER>_KEY` returns from test if absent)
- How to add a new repo-secret to GitHub Actions (operator process)
- How to interpret a failure (real-API breakage / test-account quota / network flake — runbook covers all 3 modes)
- How to run locally w/ `--local` for debug

EXTEND `docs/build-plan/KNOWN_FLAKES.md` — narrowly scoped: add a `[Silent-Skip Nightly]` section documenting any pre-existing silent-skip tests known to fail intermittently against real accounts (e.g. quota-flaky providers, dependency-on-external-state). Today the doc tracks unit-test flakes; this extends it to nightly-integration flakes.

This is NOT in scope: the actual rewrite of silent-skip → VCR/cassette fixtures (separate brief, #12); per-provider test-account creation workflow (operator concern, not code); cost-budget guardrails for the nightly run (each silent-skip test should be cheap by design — that's a follow-up brief); fail-fast vs continue-on-failure policy in CI (the script continues all, CI step doesn't fail unless ≥50% of tests fail — heuristic; operator can adjust).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `.github/workflows/silent-skip-nightly.yml.disabled` (NEW, ≤ 120 LOC) — workflow definition
- `scripts/silent-skip-nightly.sh` (NEW, ≤ 200 LOC, mode 755) — discovery + runner
- `scripts/silent-skip-nightly-tests.sh` (NEW, ≤ 100 LOC) — meta-test: validates the runner correctly discovers ≥N tests, exits 0 on a fresh repo
- `docs/skycot/runbook/silent-skip-nightly.md` (NEW, ≤ 240 LOC)
- `docs/build-plan/KNOWN_FLAKES.md` — narrowly scoped: ADD `[Silent-Skip Nightly]` section (≤ 30 LOC delta)

## Files you must NOT touch

- Any `crates/**` test files (this brief discovers + runs; doesn't rewrite them — that's the VCR brief)
- `Cargo.toml` (workspace or member)
- `crates/argo-store/migrations/**`
- Source / verticals / channels

## Acceptance criteria

- [ ] `bash scripts/silent-skip-nightly.sh --help` prints usage including the local-mode flag
- [ ] `bash scripts/silent-skip-nightly.sh --discover-only` enumerates ≥ 40 silent-skip tests across the workspace
- [ ] `bash scripts/silent-skip-nightly.sh --local` runs against the operator's local env (no GH secrets — so most tests still skip, but the run completes + emits report)
- [ ] Summary JSON has shape `{ started, completed, total, passed, failed, timeout, skipped, failures: [...] }` — schema is documented in the runbook
- [ ] Each test runs in its own `cargo test --test` invocation (one hung test doesn't block the rest)
- [ ] 5-minute per-test timeout enforced + recorded as `timeout`
- [ ] Workflow file is valid GitHub Actions YAML; uses `secrets.ARGO_TEST_*` references; daily 5am UTC schedule
- [ ] Runbook covers all 3 failure-classification modes (API-breakage / quota / flake)
- [ ] `KNOWN_FLAKES.md` extended w/ `[Silent-Skip Nightly]` section
- [ ] No Rust changes; `cargo clippy` / `cargo test` not in scope
- [ ] `bash -n scripts/silent-skip-nightly.sh` (syntax check) clean
- [ ] `shellcheck scripts/silent-skip-nightly.sh` (if installed) emits no `error:`-level findings
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-silent-skip-nightly-integration`
- PR title: `tools(ci): nightly silent-skip integration runner + workflow + runbook (mission-silent-skip-nightly-integration)`

## Notes for the implementer

- The discovery heuristic is two-pronged: (a) `cargo test -- --list` matches test names with `_smoke|_integration|_live`; (b) `rg -l 'is_err\\(\\)|env::var.*\\?'` on `tests/` directories to catch the rest. Document the heuristic explicitly in the runbook — if a new silent-skip test uses a different env-var convention, it gets missed; that's a known limitation.
- Each test runs in its own `cargo test --test <name>` invocation. This is the load-bearing design — it isolates failures + isolates per-test timeout enforcement. Don't try to optimize via `cargo test --tests` which would aggregate.
- The 5-minute per-test timeout is via `gtimeout` (macOS, gnu-coreutils) or `timeout` (Linux). Document the macOS fallback in the runbook.
- GitHub repo-secret name convention: `ARGO_TEST_<PROVIDER>_KEY` for primary keys, `ARGO_TEST_<PROVIDER>_<KIND>` for secondary (e.g. `ARGO_TEST_HUBSPOT_PORTAL_ID`). Operators add secrets via the GH UI; document this process.
- `--local` mode: useful for operators to test the runner-shape without needing CI secrets. Discovery still runs, tests still get invoked, most still skip due to absent env-vars. The report shape is preserved.
- If a CI run shows `failed > 0`, the workflow's last step posts to the `github.com/...` issue tracker via `gh issue create` w/ the failures summary. Document the issue-template shape in the runbook.
- Mirror the `mission-cargo-audit-disabled-workflow` (#1325) `.disabled` extension exactly — operators rename to `.yml` when GitHub Actions billing is restored. Document this in the workflow file's header comment AND in the runbook.
- The brief's acceptance "≥ 40 silent-skip tests" lower bound is intentionally generous — the backlog text says "~65" but the actual count may vary as tests are added/removed. The script just reports the discovered count; an operator who's surprised reads the runbook to understand discovery heuristics.
- DO NOT add cost-budget enforcement (e.g. "abort if total cost > $5"). That's a separate brief — silent-skip tests should be cheap-by-design and adding the budget code here is scope creep.
