# V1.4 — How-to: build a vertical (tutorial)

**Owner**: claude. **Depends on**: claude-docs-site-build (#934).

**Drift-fix amendment 2026-05-13** (vs HEAD 6159b789): brief-cleanup sweep updated may-touch paths — `book/src/how-to/...` → `book/howto/...` (current `book.toml` sets `src = "book"`; existing how-to dir is `book/howto/`, no hyphen). Existing files in `book/howto/`: `approvals.md`, `pick-a-vertical.md`, `troubleshoot.md`. SUMMARY.md is at `book/SUMMARY.md`. No `index.md` exists today — keep optional. Tutorial cross-link target in claude-quickstart-guide-rewrite must use `../howto/build-vertical.md`.

## Why

The Quickstart shows an operator how to *run* a pre-built
vertical. The natural next question is: "I want a worker that
does X — how do I make it?"

Today there is no end-to-end tutorial answering that. Genesis
flow exists, manual TOML editing exists, and eval-seed
verification exists, but they are documented separately in
crate AUTHORING.md files. An operator following the Quickstart
hits a cliff.

This brief writes the bridge: a single tutorial that walks
from "I have an idea" to "I have a verified working vertical
TOML on disk".

## Scope

One tutorial page covering three modes, in order:

1. **Genesis flow (recommended path)** — `argo genesis
   "describe what you want"`, walk the interview, accept the
   generated `vertical.toml`, what each section means at a
   glance.
2. **Manual TOML editing** — open the generated TOML, modify
   one capability, modify one channel binding, modify one
   judge rubric. Show the diff. Explain schema-version
   compatibility (link to RUNBOOK troubleshooting).
3. **Verify with eval seed** — `argo eval run --seed
   tutorial`, read the report, interpret pass/fail, what to
   do when a rubric fails.

Each mode has a worked example (a small concrete vertical:
"daily LinkedIn engagement nudges" or similar — pick something
realistic but trivial to reason about).

End with a "where to next" pointer block: production rollout
(RUNBOOK), more rubrics (judge reference), more channels
(channels reference).

## Files you may touch
- `book/howto/build-vertical.md` (NEW)
- `book/SUMMARY.md` (add the new entry under How-to)
- `book/howto/index.md` (extend How-to landing if it
  exists; create if it doesn't — currently no index page)

## Files you must NOT touch
- Any `crates/**` Rust source
- `docs/skycot/MISSION.md`
- Other How-to pages (`approvals.md`, `pick-a-vertical.md`,
  `troubleshoot.md` — those exist separately and are not in
  scope here)
- `book/quickstart/*` (claude-quickstart-guide-rewrite
  owns those)
- `book/reference/*` (CLI reference is auto-gen)

## Acceptance criteria
- [ ] `mdbook build` succeeds with no warnings
- [ ] `mdbook test` passes (all internal links resolve)
- [ ] Tutorial covers all three modes (Genesis, manual,
      eval-verify) with a single worked example threaded
      through
- [ ] Every CLI command shown is a real command in current
      `argo --help` output (cross-check before merging)
- [ ] Schema-version mismatch path links to RUNBOOK
      troubleshooting (claude-troubleshooting-guide-expand)
- [ ] Eval-seed section names a real seed that ships with the
      repo, not a fictional one
- [ ] Page is ≤ 600 lines rendered (skim-friendly)
- [ ] Cross-link from Quickstart "next steps" resolves

## Branch + PR
- Branch: `claude-howto-build-vertical-tutorial`
- PR title: `docs: how-to build a vertical end-to-end (claude-howto-build-vertical-tutorial)`

## Notes

Pick the worked example carefully — it must be small enough
to fit in the page yet realistic enough that an operator sees
how the pieces compose. Avoid examples that require external
account setup the reader cannot reasonably do (no real
Salesforce instance).

If the Genesis flow CLI surface diverges from what's
documented mid-stream, prefer the actual current behaviour
and note any rough edges in the PR description rather than
documenting an aspirational UX.
