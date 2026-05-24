# codex-cli-json-output-coverage-audit

**Authored against**: `origin/main` `72e782df` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cli-json-output-coverage-audit`.

**Fast-mode eligible**: yes — pure mechanical audit + additive.

## Summary

[`CLAUDE.md`](CLAUDE.md) project invariant: "JSON output for CLI list/show/status commands". Codex audits every `argo` CLI subcommand under `crates/argo-cli/src/handlers/` and confirms every list/show/status command supports `--json`. Add the flag + serde output for any missing one.

## Acceptance

- [ ] Audit script (one-shot Python or bash, can be temporary or kept) enumerates every CLI subcommand and reports which support `--json`. Output written to PR body as a table.
- [ ] For each list/show/status command that does NOT support `--json`, add it. Use the same JSON-shape conventions as existing supporting commands (read 2–3 examples first; pick the dominant pattern).
- [ ] Each newly-added `--json` path has at least one unit test covering shape correctness.
- [ ] CLI help text updated for each (no breaking changes to existing flags).
- [ ] No semantic change to any subcommand's non-JSON output.

## May touch

- `crates/argo-cli/src/handlers/**/*.rs` (any subcommand that needs `--json` added)
- `crates/argo-cli/src/cli_*.rs` (only command-schema files needed for the audited list/show/status gaps)
- `crates/argo-cli/src/cli_command_enum.rs` (only if a new flag needs registering on the enum)
- `crates/argo-cli/src/main_dispatch_setup.rs` (only dispatch arms for newly-added JSON paths)
- Tests under `crates/argo-cli/tests/` (additive only)

## Must NOT touch

- The semantic behavior of any command. JSON output is additive.
- The single-binary architecture invariant.
- Any non-CLI crate.

## Notes

- Project CLAUDE.md is the authority on the "JSON output for CLI list/show/status" invariant.
- Audit-first approach: report the gap, then fill it. Don't blindly add `--json` to every command — only list/show/status per the rule.
- If the audit shows full coverage already, the PR body documents that and the brief closes as RESOLVED-NO-CHANGE.
- Sizing: 200–600 LOC depending on coverage gap.
