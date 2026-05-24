# ops-open-pr-overlap-triage-script

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `52dc49988` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-open-pr-overlap-triage-script`.

🟡 **P2 conductor-review helper.** Source: this tick's PR set has two mergeable PRs that both touch `crates/argo-cli/src/handlers/gateway/mod.rs`; today that overlap was found manually. The conductor prompt already asks for PR triage when the queue is large, but there is no repo helper that highlights file-overlap merge ordering risk.

## Scope

Add a small script that:

1. Lists open PRs via `gh pr list --state open --json ...`.
2. Collects changed file paths for each PR.
3. Emits any overlapping paths shared by 2+ open PRs.
4. Prints a concise recommendation line per overlap group so the conductor can rebase/order merges intentionally.

Keep it read-only: no comments, no merges, no labels.

## Files you may touch

- `scripts/open-pr-overlap-triage.sh` (NEW, ≤260 LOC)
- `scripts/tests/open-pr-overlap-triage.bats` (NEW, ≤220 LOC)
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — narrowly: mention the helper in Step 5 review guidance

## Files you must NOT touch

- GitHub state
- `docs/build-plan/BUILD_STATUS.md` except the standard row flip
- Feature crates under `crates/`

## Acceptance criteria

- [ ] Script exits `0` when no overlaps exist and prints a short success line
- [ ] Script groups overlaps by file path and lists the PR numbers involved
- [ ] Script ignores docs-only overlap when both paths are under `docs/` and no code paths overlap
- [ ] Tests cover no-overlap, single shared path, and multiple shared-path groups
- [ ] Prompt docs mention the helper as optional conductor review input
- [ ] Every authored file <500 LOC

## Branch + PR

- Branch: `codex-ops-open-pr-overlap-triage-script`
- PR title: `ops: add open PR overlap triage helper (ops-open-pr-overlap-triage-script)`
