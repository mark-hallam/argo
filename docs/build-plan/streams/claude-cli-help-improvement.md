# V1.4 — CLI help text improvement (examples on every subcommand)

**Owner**: claude. **Depends on**: nothing (independent of
docs-site-build, but improves auto-gen reference once both
land).

## Why

`argo --help` and per-subcommand help is uneven: some commands
have rich examples, most have a one-liner description and
nothing else. Operators reading help in a terminal —
especially newcomers following the Quickstart — bounce off
when there's no concrete invocation to copy.

Auto-generated CLI reference (`scripts/build-cli-reference.sh`)
is only as good as the underlying clap text. Improving the
help text improves both the terminal experience and the
docs-site reference for free.

## Scope

Audit clap definitions across the workspace. For every
subcommand and every flag with non-obvious semantics:

1. Ensure `about` (short) is a single-line imperative.
2. Ensure `long_about` exists and explains *when* to use it
   (one paragraph max).
3. Ensure at least one `#[command(after_help = "Examples:\n
   ...")]` block with one or more realistic invocations.
4. Flags with enum-like values list the values inline.

Add a CI check (`scripts/check-cli-help-examples.sh`) that
walks `argo --help` output, asserts every leaf subcommand has
a non-empty `Examples:` block, and fails CI if any command
regresses.

This is a workspace-wide *narrow* edit pass: only clap
attributes change, no logic, no signatures, no behaviour.

## Files you may touch
- `crates/cli/**/*.rs` (clap attributes only — no logic,
  no signature changes)
- Any other crate's `main.rs` or clap definitions consumed by
  the CLI surface
- `scripts/check-cli-help-examples.sh` (NEW)
- `.github/workflows/ci.yml` (add the check to CI matrix —
  narrow extension)

## Files you must NOT touch
- Any non-clap Rust logic
- Function signatures or public APIs
- `scripts/build-cli-reference.sh` (consumes help, doesn't
  produce it)
- Test fixtures unrelated to help-smoke tests
- `docs/**` (changes flow through auto-gen, not by hand)

## Acceptance criteria
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds (no behaviour
      regressions; help-smoke tests still pass)
- [ ] `scripts/check-cli-help-examples.sh` passes locally and
      in CI
- [ ] Every leaf subcommand of `argo --help` has an
      `Examples:` block with ≥ 1 realistic invocation
- [ ] No clap `long_about` exceeds 80 columns × 12 lines
      (skim-friendly in a terminal)
- [ ] No example references flags or subcommands that don't
      exist
- [ ] PR diff is clap-attributes-only outside of the new
      script and CI extension (reviewer can skim with
      confidence)
- [ ] mdBook auto-gen CLI reference picks up the new examples
      (verify by running `scripts/build-cli-reference.sh`
      locally and grepping output)

## Branch + PR
- Branch: `claude-cli-help-improvement`
- PR title: `cli: help-text examples on every subcommand + CI guard (claude-cli-help-improvement)`

## Notes

Watch the diff size — workspace-wide attribute edits balloon
fast. If a subcommand has 12 flags and only 2 need attention,
edit only those 2; don't touch the others for symmetry.

Examples should be paste-runnable on a clean install. Avoid
examples that reference fictional vertical names or seed
data. Prefer short examples that exercise the common case
over long examples that exercise every flag.

If an example needs an env var (`ANTHROPIC_API_KEY`), state
it on its own line before the command rather than inlining
`ANTHROPIC_API_KEY=... argo ...` (operators copy-paste, env
exports are cleaner).

The CI check should be conservative — parse `argo help` output
once, assert per-command, fail loudly with the offending
subcommand name. Don't try to validate the *quality* of
examples; presence is enough for a guard.
