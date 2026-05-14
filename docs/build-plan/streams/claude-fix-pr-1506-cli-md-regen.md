# claude-fix-pr-1506-cli-md-regen — regenerate cli.md and unblock #1506

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `b46b0fa2` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-fix-pr-1506-cli-md-regen`.

## Scope

PR #1506 (`claude-mission-cli-help-public-api-reference`) has been CI-failing for 35h+. The `cli-reference-drift` workflow rejects the checked-in `cli.md` artifact because main has moved 200+ commits and many new subcommands exist.

Worker action: check out the PR, run the CLI reference generator against current tree, commit the regenerated `cli.md`, push.

**Specific actions**:

1. `gh pr checkout 1506 && git fetch origin main && git merge origin/main`.
2. Resolve any conflicts on `cli.md` itself (take generator output; not manual edits).
3. Regenerate `cli.md` using whatever the PR's generator script invokes (likely `cargo run -p argo-cli -- <help-dump-command> > docs/cli.md` or similar — check the cli-reference-drift workflow YAML for the exact command).
4. Verify the workflow passes locally if possible: `gh workflow view cli-reference-drift` to see the exact check.
5. `git push --force-with-lease`.
6. Confirm CI `cli-reference-drift` flips green.

**Not in scope**: changing the generator itself; widening the brief's may-touch.

## Files you may touch

- `docs/cli.md` (or wherever the PR puts the artifact — verify against the PR's diff).
- That's it.

## Files you must NOT touch

- The `.github/workflows/cli-reference-drift.yml` workflow itself.
- Any source code that's not part of the PR's existing diff.

## Acceptance criteria

- [ ] `cli.md` regenerated against current main.
- [ ] `git push --force-with-lease origin claude-mission-cli-help-public-api-reference`.
- [ ] PR #1506 CI `cli-reference-drift` goes GREEN.
- [ ] PR mergeable=true.

## Branch + PR

- Branch: `claude-mission-cli-help-public-api-reference` (existing).
- PR title: existing.

## Notes for the implementer

- ~10 min mechanical regen.
- If the generator command isn't obvious from the PR or the workflow, BLOCK with a one-line note asking for the exact regen command.
- `--force-with-lease` only.
