# Mission load-bearing wire — QA-engineer vertical

**Owner**: claude. **Depends on**: existing vertical-template scaffold (`verticals/executive-assistant/` MERGED #1336 + `verticals/research-analyst/` MERGED #1337 as canonical patterns).
**Authored against**: origin/main `fef33298` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-qa-engineer`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #152 ("QA-engineer vertical"). **Tier A (vertical).** Test-strategy + test-authoring + regression-investigation drafts. Mirrors the legal-paralegal "drafts only, attorney supervises" pattern: the QA engineer proposes tests + reports + repro recipes; the human reviewer signs off before tests land in the main branch.

## Scope

NEW content-only `verticals/qa-engineer/` (11 files, ≤ 500 LOC total — pattern budget: system ≤ 130, persona ≤ 35, skills 4×≤65, schema ≤ 35, auto_approval ≤ 25, inventory ≤ 55, vertical.toml ≤ 55, eval/seed.jsonl ≥ 8 fixtures):

- `vertical.toml` — id=`qa-engineer`, version=`0.1.0`, optional github/gitlab/jira MCP, `[escalation].human_approval_required = ["merge_test_branch", "close_bug_as_wont_fix", "modify_production_test_data", "disable_failing_test_in_main"]`
- `prompts/system.md` — **Reproducibility constraint load-bearing**: every bug report includes (a) exact steps to reproduce, (b) environment fingerprint (OS+browser+app-version+commit-hash), (c) expected vs actual, (d) frequency (always / N-of-M / once-only). Tests authored MUST be deterministic — no `sleep()`, no `rand()` without seed, no wall-clock dependencies. Escalation triggers (test-data-contains-PII / flake-suspected-but-unconfirmed / proposed-fix-touches-pricing-or-billing / disable-test-in-main / production-data-modification).
- `prompts/persona.md` — Cam, QA engineer; assumes the bug is in the code first, the test second, the test-author third; never assumes "user error" without evidence; never closes a flaky test as "intermittent" without a per-cause explanation
- 4 skills:
  - `propose_regression_test` — bug report + reproduction in; proposed test (language-agnostic shape) + assertion checklist + setup-cleanup notes out
  - `author_bug_report` — observed-behavior + expected-behavior in; full bug report w/ steps + environment + frequency + severity-suggestion + escalation-required-Y/N out
  - `investigate_flake` — failing-CI-log + recent-commits in; per-cause hypothesis ranking + recommended-next-investigation + classification (test-bug / code-race / external-dep / environment) out
  - `propose_test_plan` — feature spec in; layered test plan (unit / integration / e2e / smoke) w/ per-layer responsibilities + dropped-coverage-rationale + estimated count out
- `memory/schema.toml` — `flake_history` (per-test flake-rate + cause-classifications), `test_data_inventory` (named test fixtures + PII-status), `known_dependencies` (external services tests rely on + their SLA), `release_test_matrix` (per-version test-coverage snapshots)
- `eval/seed.jsonl` — 8+ fixtures: 2 regression-test-proposals (one happy + one with PII data hint requiring escalation), 1 bug-report-authoring, 1 flake-investigation w/ test-bug hypothesis, 1 flake-investigation w/ code-race hypothesis, 1 test-plan w/ explicit dropped-coverage rationale, 3 escalation (PII-test-data / disable-test-in-main / proposed-fix-touches-billing)
- `auto_approval.toml` — wake list includes propose_regression_test + author_bug_report + investigate_flake + propose_test_plan only; merge_test_branch / close_bug_as_wont_fix / modify_production_test_data / disable_failing_test_in_main intentionally absent
- `inventory.json` — sample `flake_history` (3 synthetic entries showing the cause-classification shape), sample `test_data_inventory` (4 named fixtures: 2 PII-free + 2 PII-tagged), sample `known_dependencies` (3 external service SLA entries — all synthetic)

NO Rust. NO migrations. NO crate changes. Pattern is identical to the existing verticals — re-use shapes 1:1.

This is NOT in scope: actual test execution (the vertical proposes; CI executes); CI integration (separate brief — vertical is GitHub-action-agnostic in v1); per-language test framework templates (Rust criterion / Python pytest / TypeScript Jest — vertical describes the test SHAPE, not the framework syntax); load-test design (separate — covered by mission-criterion-load-tests + mission-soak-test-nightly); production-incident postmortem authoring (separate vertical — ops-on-call already covers this).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/qa-engineer/vertical.toml` (NEW, ≤ 55 LOC)
- `verticals/qa-engineer/prompts/system.md` (NEW, ≤ 130 LOC)
- `verticals/qa-engineer/prompts/persona.md` (NEW, ≤ 35 LOC)
- `verticals/qa-engineer/skills/propose_regression_test.md` (NEW, ≤ 65 LOC)
- `verticals/qa-engineer/skills/author_bug_report.md` (NEW, ≤ 65 LOC)
- `verticals/qa-engineer/skills/investigate_flake.md` (NEW, ≤ 65 LOC)
- `verticals/qa-engineer/skills/propose_test_plan.md` (NEW, ≤ 65 LOC)
- `verticals/qa-engineer/memory/schema.toml` (NEW, ≤ 35 LOC)
- `verticals/qa-engineer/eval/seed.jsonl` (NEW, ≥ 8 fixtures)
- `verticals/qa-engineer/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/qa-engineer/inventory.json` (NEW, ≤ 55 LOC)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory

## Acceptance criteria

- [ ] `argo vertical list` includes `qa-engineer 0.1.0`
- [ ] `argo vertical lint qa-engineer` exits 0 errors (warns of the executive-assistant / research-analyst / legal-paralegal baseline shape are acceptable)
- [ ] `argo vertical show qa-engineer --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 8 / vertical_toml)
- [ ] All 4 skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] System prompt names reproducibility-constraint + determinism-rule explicitly
- [ ] `eval/seed.jsonl` has ≥ 8 fixtures, ≥ 3 of which are escalation cases
- [ ] `auto_approval.toml` wake list contains only the 4 propose/author/investigate skills; merge / close / modify / disable gated through `[escalation].human_approval_required`
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-qa-engineer`
- PR title: `vertical: qa-engineer (drafts-only test + bug-report + flake-investigation) (mission-vertical-qa-engineer)`

## Notes for the implementer

- Treat the just-merged `verticals/legal-paralegal/` (#1357) and `verticals/research-analyst/` (#1337) as 1:1 templates — same file shape, same YAML frontmatter, same `[escalation]` block style. The drafts-only pattern is shared.
- Reproducibility constraint is the load-bearing differentiator vs other verticals. EVERY bug report should include the 4 elements (steps / environment / expected-vs-actual / frequency); EVERY proposed test should be deterministic (no `sleep`, no `rand` without seed, no wall-clock dependencies). Eval fixtures should check that proposed-test outputs respect determinism.
- Flake-investigation skill should produce a RANKED hypothesis list — not just "this is flaky," but "this is most likely cause A because <evidence>, less likely cause B because <evidence>, ..." — operators can act on a ranked list, not a binary verdict.
- The "PII test data" escalation is the privacy/security gate — any proposed test or bug report that includes (or might include) PII routes to human review. Memory's `test_data_inventory` tagging surfaces this; eval fixture exercises it.
- Cam's persona: methodical, skeptical of "user error" without evidence. This counteracts the LLM tendency to say "this is probably user-side" — Cam should assume code-side first, test-side second, user-side last.
- `propose_test_plan` should explicitly enumerate what's NOT covered + why (cost-bounded / out-of-scope / lower-value). Operators get the "we ship 80%; here's the 20% we explicitly dropped" tradeoff visible.
- DO NOT add language-specific test syntax. The vertical describes the SHAPE — "an integration test that boots component X w/ mock Y and asserts Z" — not the Rust/Python/TS syntax. Operators use the shape to author in whatever framework they like.
- Two escalation fixtures should specifically exercise: (1) a flake-investigation that lands on "test-data contains PII" requiring escalation, (2) a `disable_failing_test_in_main` proposal where the human must confirm before the test gets the disable-attribute.
