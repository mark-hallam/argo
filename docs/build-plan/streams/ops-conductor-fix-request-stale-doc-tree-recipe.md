# ops-conductor-fix-request-stale-doc-tree-recipe

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `8da732a7d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-conductor-fix-request-stale-doc-tree-recipe`.

🟢 **P3 CONDUCTOR PROCESS HYGIENE.** Recent stale-IN_REVIEW PRs repeatedly
carried stale generated doc-tree changes after worker rebases. The conductor
fix-request comments eventually converged on a safe recipe, but the live
protocol does not yet spell it out. This brief adds a short reusable comment
template so future fix requests are precise and do not invite broad checkout
commands.

## Source

- Tick history around PR #2131 stale doc-tree clobbers.
- `docs/skycot/evals/scheduled/2026-05-24/round-45-pr-2169-cron-surface-contract-anti-theater-verify.md`
  and `docs/skycot/agents/SHARED.md` both surfaced the need for a clearer
  stale-doc-tree cleanup recipe.

## Scope

- Add a short conductor fix-request template to the live loop prompt.
- The template must show explicit path checkout from `origin/main` for generated
  auditor/evaluator/heartbeat trees when a worker branch accidentally carries
  stale copies.
- The template must warn against `git checkout origin/main -- .` and any
  broad reset that can delete the worker's intended code changes.

## May-touch

- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md`
- `docs/build-plan/CONTRIBUTING_AGENT.md` — only if a one-line cross-reference
  is needed.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip only.

## Must not touch

- `docs/build-plan/PROTOCOL.md` unless the worker proves the live prompt is
  not the operative source.
- Any source code.
- Any worker branch.

## Acceptance criteria

- [ ] Live protocol includes a copy-ready fix-request template for stale
      generated doc-tree changes.
- [ ] Template uses explicit paths and `origin/main` as the source of truth.
- [ ] Template includes a clear warning against broad checkout/reset commands.
- [ ] Wording distinguishes generated doc-tree cleanup from legitimate brief
      documentation edits.
- [ ] `git diff --check` clean.
- [ ] Every modified file stays under 500 LOC.

