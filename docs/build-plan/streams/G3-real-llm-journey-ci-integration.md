# Stream G3 — Real-LLM journey test → GitHub Actions CI integration

**Owner**: claude.
**Depends on**: `mission-real-llm-operator-journey-test` MERGED (the script ships with skip-clean behavior).
**Authored against**: origin/main `761766a4` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh G3-real-llm-journey-ci-integration`.
**Tier tag**: PRODUCT_ROADMAP.md Track G brief 3 of 3 (LAST in Track G).

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| CI provider | GitHub Actions (existing repo CI) | Same | n/a |
| Provider-key storage | GitHub Actions Secret (encrypted at rest by GitHub; rotated by operator quarterly) | Per-tenant KMS-managed secrets with auto-rotation | `mission-enterprise-ci-kms-secrets` |
| Trigger cadence | Nightly cron + manual `workflow_dispatch` | Add tenant-aware scheduling (e.g. EU tenant tests run on different keys) | `mission-enterprise-ci-tenant-aware-scheduling` |
| Cost ceiling | Per-run hard budget enforced by the existing 90s wall-clock + manual review of `--max-steps 5` | Per-workflow-run cost cap with kill-switch on overspend | `mission-enterprise-ci-cost-cap` |
| Result surfacing | GitHub Actions UI + Slack alert on failure (existing CI Slack webhook) | Dashboard with historical pass-rate + drift detection (e.g. did Claude 5.0 change the test outcome?) | `mission-enterprise-ci-result-dashboard` |
| Provider rotation | Manual operator-managed secret swap | Auto-rotation with overlapping validity windows | `mission-enterprise-ci-provider-rotation` |

v1 is acceptable: GitHub Actions Secrets + nightly
cron is standard practice and meets the basic
"is the agent still thinking coherently?" question.
Enterprise hardening adds tenant-isolation + cost
governance + result dashboards later.

## Why

Track B shipped `scripts/smoke-skycot-real-llm.sh`
that runs against a real provider (Anthropic or
OpenAI) with coherence + cost + 90s budget assertions
and a clean skip path when no key is set. The script
is operator-runnable today, but it's not in CI — so
regressions could land silently. This brief wires it
into the existing GitHub Actions setup.

For enterprise: regression detection is what
distinguishes "we have a smoke test" from "we know
the smoke is green." Nightly CI runs against the
shipped agent prove the platform still meets mission
claim 1 ("AI workers humans can bring into their
team").

## Scope

- NEW `.github/workflows/real-llm-smoke.yml`:
  - Triggers: nightly cron (06:00 UTC daily) +
    `workflow_dispatch` (manual button).
  - Job: install Rust + build `argo` binary →
    run `scripts/smoke-skycot-real-llm.sh
    target/debug/argo`.
  - Provider key sourced from `ANTHROPIC_API_KEY`
    secret (Anthropic preferred; OpenAI as
    fallback if Anthropic key absent).
  - 15-minute hard timeout on the job.
- Slack notification on failure via existing
  webhook (operator-configured secret
  `SLACK_NOTIF_WEBHOOK_URL`).
- Bot-friendly output: workflow run summary
  includes the smoke script's stdout (especially
  the `RAN`/`SKIPPED` header + `COST:` line).
- Idempotent: if the workflow file already exists
  in some shape from a prior brief, this brief
  reconciles + replaces.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `.github/workflows/` (NEW
  workflow file).

## Files you may touch

- `.github/workflows/real-llm-smoke.yml` — NEW.
  ≤ 150 LOC.
- `scripts/smoke-skycot-real-llm.sh` —
  narrowly scoped: if the existing script needs
  tiny adjustments for CI (e.g. GitHub Actions'
  non-interactive bash), document and apply. Most
  likely no changes. ≤ 250 LOC after.
- `docs/skycot/getting_started.md` — narrowly
  scoped: add an "Automation" subsection noting
  the CI workflow's existence + how to manually
  trigger.
- `docs/skycot/CI_FALLBACK_RUNBOOK.md` (existing
  file) — narrowly scoped: add a section "Real-LLM
  smoke failures" with triage steps. ≤ 200 LOC
  added.

## Files you must NOT touch

- `scripts/smoke-skycot.sh` (mock-mode smoke) —
  separate workflow + separate brief.
- Existing CI workflows in `.github/workflows/`
  unrelated to this brief.
- `crates/**` — no Rust changes.
- The argo binary itself.

## Acceptance criteria

- [ ] `.github/workflows/real-llm-smoke.yml` is a
      valid YAML GitHub Actions workflow.
- [ ] Triggers:
      - `schedule: - cron: '0 6 * * *'` (06:00 UTC
        daily; matches the existing pattern).
      - `workflow_dispatch:` (manual trigger).
- [ ] Job steps:
      1. Checkout repo.
      2. Setup Rust toolchain (1.75 per
         CLAUDE.md invariants).
      3. Cache `~/.cargo` + `target/`.
      4. `cargo build --release -p argo-cli`.
      5. Run `./scripts/smoke-skycot-real-llm.sh
         target/release/argo`.
- [ ] Provider key:
      - First check `ANTHROPIC_API_KEY` secret.
      - Fall back to `OPENAI_API_KEY` secret.
      - If neither set: workflow run posts a
        warning and the script outputs SKIPPED
        — but the workflow still counts as
        passing.
- [ ] Job-level 15-minute timeout.
- [ ] On failure: Slack notification posted via
      `SLACK_NOTIF_WEBHOOK_URL` secret. Message
      includes the failure section + link to the
      run.
- [ ] Workflow output summary (using GitHub
      Actions `$GITHUB_STEP_SUMMARY`) renders:
      - Provider used (Anthropic / OpenAI /
        SKIPPED)
      - Cost reported (if available)
      - Pass/fail status
- [ ] CI_FALLBACK_RUNBOOK.md has a "Real-LLM
      smoke failures" section.
- [ ] `actionlint` validation of the workflow
      file passes (no syntax errors).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-G3-real-llm-journey-ci-integration`
- PR title: `ci: nightly real-LLM smoke workflow (G3)`

## Notes for the implementer

- **Workflow shape — follow existing patterns.**
  Look at `.github/workflows/ci.yml` (re-enabled
  via #930) for the standard step idioms (Rust
  setup action version, cache paths, runner
  label). Match exactly; don't introduce divergent
  patterns.

- **Anthropic preferred; OpenAI fallback.**
  Reasoning: the operator's primary provider is
  Anthropic per the binary's default; OpenAI is
  documented as a working alternative but not
  the headline. Workflow tries Anthropic first;
  if that secret is absent, tries OpenAI.

- **Skip path is a pass.** A workflow run where
  no provider key is configured should report
  `SUCCESS` with `SKIPPED` in the summary. This
  matches the script's exit-0 skip behavior and
  prevents flapping in environments where the
  operator deliberately omits the key (e.g. forks,
  external contributors).

- **Cost reporting in the summary.** The script
  emits `=== COST: $X.XX (input=N output=M) ===`
  when cost data is available. Parse it from
  stdout via a regex and write to
  `$GITHUB_STEP_SUMMARY`. If not parseable, write
  `Cost: (not reported)`.

- **Slack failure notification — reuse existing
  webhook.** The repo already has `SLACK_NOTIF_
  WEBHOOK_URL` secret per other workflows.
  Format:
  ```
  🚨 Real-LLM smoke FAILED
  Run: https://github.com/.../actions/runs/<id>
  Last log line: <tail of failure>
  ```
  Skip the notification on SUCCESS or SKIPPED.

- **Don't add a `concurrency:` block.** The
  nightly runs should never overlap by design;
  but if a manual `workflow_dispatch` happens
  during a nightly, let both run — they hit
  different timestamps anyway.

- **Cron timezone gotcha.** GitHub Actions cron
  is UTC. `0 6 * * *` is 06:00 UTC = ~7pm NZST
  (operator's tz). That's deliberate — runs
  while the operator sleeps so they see the
  result over morning coffee.

- **Self-test the workflow on a side branch.**
  Before merging, manually trigger via
  `workflow_dispatch` on the PR's branch to
  verify it runs green. Document the verification
  in the PR description.

- **What this brief does NOT do.**
  - Doesn't add EU/regional CI tenants (Track-H).
  - Doesn't add cost dashboards (Track-H).
  - Doesn't add provider key auto-rotation
    (Track-H).
  - Doesn't extend the smoke script's coverage —
    that's a separate brief if needed.
  - Doesn't add mock-mode CI (the existing CI
    already runs mock-mode smoke per #930).

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-ci-kms-secrets` —
  per-tenant KMS-managed secrets.
- `mission-enterprise-ci-tenant-aware-scheduling`
  — different schedules per regional tenant.
- `mission-enterprise-ci-cost-cap` — per-run
  cost ceiling with kill switch.
- `mission-enterprise-ci-result-dashboard` —
  pass-rate + drift dashboard.
- `mission-enterprise-ci-provider-rotation` —
  auto-rotation of provider keys.

## Track G author phase COMPLETE after this commits

- G1 (Windows CU dispatch wiring)
- G2 (Linux Wayland support)
- G3 (real-LLM CI integration)

= 3 briefs. Track G fully authored. Per the
checkpoint rule, the /loop pauses here for
operator review before Track J starts.
