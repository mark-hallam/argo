# V1.4 — Quickstart guide rewrite (operator-first)

**Owner**: claude. **Depends on**: claude-docs-site-build (#934),
claude-homebrew-formula-publish (#928).

## Why

Today's `book/src/quickstart/*` is dev-focused: it assumes the
reader can `cargo build`, edit Rust, and reason about the
workspace. The target audience for v1 is an **operator** —
marketing intern, SDR, support lead — who has never written
Rust and just wants to install Argo and watch a worker run.

Goal: a stranger lands on the docs site, follows the
Quickstart, and has a working worker dispatching real actions
in **10 minutes**, with zero Rust toolchain.

## Scope

Rewrite the Quickstart section end-to-end as an operator
walkthrough:

1. **Install** — Homebrew one-liner (`brew install argo`),
   verify with `argo --version`. Include Linux + Windows
   alternatives (binary releases) but lead with macOS Homebrew.
2. **Initialise a workspace** — `argo init my-worker`, what
   the directory contains, what `argo.toml` is.
3. **Pick a vertical** — short menu of the 5 demo verticals
   (marketplace-seller / customer-support / sales-sdr /
   marketing-intern / finance-ar) with one-line "use this if".
4. **Configure credentials** — `ANTHROPIC_API_KEY` env var,
   one channel (Slack or email) via `argo channels add`.
5. **First run** — `argo run`, watch the executor pick up a
   seed task, observe action emitted, confirm it landed.
6. **Next steps** — links to How-to (build a vertical),
   Reference (CLI), Operations (RUNBOOK).

Each step has: copy-paste command, expected output snippet,
"if you see X instead, see troubleshooting".

## Files you may touch
- `book/src/quickstart/install.md` (rewrite)
- `book/src/quickstart/init.md` (rewrite or NEW)
- `book/src/quickstart/pick-vertical.md` (NEW)
- `book/src/quickstart/credentials.md` (NEW)
- `book/src/quickstart/first-run.md` (rewrite)
- `book/src/quickstart/next-steps.md` (NEW)
- `book/src/SUMMARY.md` (update Quickstart entries)

## Files you must NOT touch
- Any `crates/**` Rust source
- `docs/skycot/MISSION.md` (immutable mission paragraph)
- `scripts/build-cli-reference.sh` (auto-gen, not authored)
- Other doc sections (`how-to/`, `reference/`,
  `architecture/`, `operations/`)

## Acceptance criteria
- [ ] `mdbook build` succeeds with no warnings
- [ ] `mdbook test` passes (all internal links resolve)
- [ ] Walkthrough is end-to-end runnable on a clean macOS
      box (mentally walk it; no `cargo` invocations anywhere
      in the user-facing path)
- [ ] Each step states expected output explicitly
- [ ] Total reading + run time fits inside 10 minutes for a
      non-Rust operator (validate by reading aloud + timing)
- [ ] Cross-links to `how-to/build-vertical.md` (claude-howto-
      build-vertical-tutorial) and RUNBOOK troubleshooting
      resolve
- [ ] No mention of `cargo`, `rustup`, or workspace internals
      in the operator path
- [ ] Demo-vertical menu lists exactly the 5 verticals from
      #937 with accurate one-liners

## Branch + PR
- Branch: `claude-quickstart-guide-rewrite`
- PR title: `docs: operator-first Quickstart rewrite (claude-quickstart-guide-rewrite)`

## Notes

Keep prose tight. Operator readers skim; lead with the command,
follow with one-sentence justification. Screenshots are not
required for v1 — text-mode terminal output snippets are
enough.

If a step depends on a CLI affordance that does not yet
exist (e.g. `argo channels add` Slack flow), flag it in the
PR description rather than inventing the UX. Co-ordinate
with the CLI brief authors before papering over a gap.
