# Mission load-bearing wire — Auto-generated CLI API reference from `argo --help`

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `01dc04cd` on `2026-05-13`.

> SOURCE: MISSION_BACKLOG #167. The mdBook documentation site (#168 SHIPPED) currently lacks a comprehensive CLI reference. Operators discover commands via `argo --help` only. Add a script that materializes the full command tree (every `argo <cmd>`, `argo <cmd> <sub>`, `argo <cmd> <sub> --help`) into a markdown reference page hosted in the existing mdBook. **Tier-Doc.**

## Scope

1. **NEW** `scripts/gen-cli-reference.sh` (≤ 200 LOC, bash) — walks the `argo` binary's command tree depth-first using `clap`-style output. Strategy: parse `--help` text, identify "SUBCOMMANDS:" section, recurse. Idempotent.
2. **NEW** `docs/skycot/book/src/reference/cli.md` (generated; checked in for traceability) — the output of the script. Auto-generated header banner.
3. **NEW** `.github/workflows/cli-reference-drift.yml` — on PRs that touch `crates/argo-cli/`, runs the script and fails if `docs/skycot/book/src/reference/cli.md` is out of date. Operator response: re-run `scripts/gen-cli-reference.sh` locally and commit the diff.
4. **EDIT** `docs/skycot/book/src/SUMMARY.md` — add `Reference` chapter pointing at the new page.
5. **EDIT** `docs/skycot/RUNBOOK.md` — operator-facing reference: how to regenerate the doc if your local clippy fails the drift check.

NOT in scope: prettifying the `--help` output (use it as-is); rendering options as tables (markdown code-fenced text is fine for v1); cross-linking commands to brief slugs; automated `mdbook build` in CI (separate brief).

## Files you may touch

- `scripts/gen-cli-reference.sh` (NEW, ≤ 200 LOC)
- `docs/skycot/book/src/reference/cli.md` (NEW, auto-generated; typically ≤ 2000 LOC depending on command count)
- `docs/skycot/book/src/SUMMARY.md` — append 1 line under existing `Reference` section (or create one)
- `.github/workflows/cli-reference-drift.yml` (NEW, ≤ 80 LOC)
- `docs/skycot/RUNBOOK.md` — append 1 subsection (≤ 20 LOC)

## Files you must NOT touch

- Any `crates/argo-cli/**` source — this brief is doc-only; the `--help` output is the API
- The `mdbook` build workflow — additive
- Any other doc page

## Acceptance criteria

- [ ] `scripts/gen-cli-reference.sh` walks every subcommand level
- [ ] `docs/skycot/book/src/reference/cli.md` regenerates cleanly + stable byte-for-byte across runs (same git revision)
- [ ] SUMMARY.md links to the new page
- [ ] `cli-reference-drift.yml` action fires on `argo-cli/**` touches and errors out on drift
- [ ] RUNBOOK explains the regen flow
- [ ] BUILD_STATUS.md row updated

## Branch + PR

- Branch: `claude-mission-cli-help-public-api-reference`
- PR title: `docs: auto-generated CLI reference page + drift CI (mission-cli-help-public-api-reference)`

## Notes

- The `clap` derive-API used by `argo-cli` produces deterministic help text only if the binary's output is stable across builds. Verify: run the script twice and `diff` the outputs — they should be byte-identical.
- The generated `cli.md` must include the auto-generated-do-not-edit banner so PR reviewers don't try to edit it by hand.
- For the drift CI: keep it FAST. Build `argo` once, then walk the tree. Should complete in <60s on a CI runner.
- If `argo --help` returns terminal-color escape codes, strip them in the script (`sed 's/\x1b\[[0-9;]*m//g'`).
- The auto-generated content lives in the same tree as authored docs. Worker should NOT mix the two. Authored references can cross-link to `cli.md#section`; the reverse is not allowed (auto-gen never references authored pages).
