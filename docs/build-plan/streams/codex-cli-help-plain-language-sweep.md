# codex-cli-help-plain-language-sweep

**Authored against**: `origin/main` `6fda55b5` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cli-help-plain-language-sweep`.

**Fast-mode eligible**: yes — pure copy editing.

## Summary

Operator preference (canonical: [~/.claude/CLAUDE.md](~/.claude/CLAUDE.md) "Plain language" section): every dev term gets a one-line plain-English summary when expanded; abbreviations should not be unexplained. CLI help text (`#[arg(help = "...")]` and `#[command(about = "...")]` attributes in `crates/argo-cli/`) is the operator-facing surface — sweep it for jargon and rewrite in plain language.

## Acceptance

- [ ] Enumerate every `#[arg(help = "...")]` and `#[command(about = "...")]` string in `crates/argo-cli/src/`. Output count + list to PR body.
- [ ] For each string, classify:
  - **OK** — already plain language ("List active workers").
  - **DEV-JARGON** — uses unexplained terms ("Boot the JWT-signing keystore" — JWT unexplained).
  - **ABBREVIATION-ONLY** — uses initialisms without expansion ("Show MCP status" — MCP unexplained).
- [ ] Rewrite the DEV-JARGON and ABBREVIATION-ONLY strings to follow the operator's expand-then-summarize pattern. Examples:
  - "Boot the JWT-signing keystore" → "Boot the JWT (JSON Web Token — encrypted login receipt) signing keystore"
  - "Show MCP status" → "Show MCP (Model Context Protocol — how external tools plug in) status"
- [ ] All command behavior unchanged. Pure copy edit.
- [ ] No regression on snapshot tests (`cargo test -p argo-cli` clean). If snapshot tests fail because they pinned the old help text, update the snapshots in the same commit and note in PR body.
- [ ] PR body includes before/after table for every rewritten string.

## May touch

- `crates/argo-cli/src/**/*.rs` — only the `help = "..."` and `about = "..."` literal strings.
- Snapshot test files (`crates/argo-cli/tests/snapshots/`) if they pin the old strings.

## Must NOT touch

- Command behavior, flag names, or argument signatures.
- Non-CLI crates.
- Documentation files (separate sweep would handle docs).

## Notes

- Canonical voice authority: [~/.claude/CLAUDE.md](~/.claude/CLAUDE.md) "Plain language — expand abbreviations + add one-line summaries". Read it first.
- Don't expand every term — only the dev-jargon and unexplained-abbreviation ones. "Worker", "tenant", "session" are fine as-is for the CLI-using audience.
- Sizing: 200–500 LOC depending on how many strings need rewriting.
- This is high-leverage operator-facing polish — the CLI is the primary interface the operator will use, and the demo will show it.
