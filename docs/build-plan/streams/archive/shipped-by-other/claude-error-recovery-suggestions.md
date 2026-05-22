# V1.4 — Error recovery suggestions (LLM-assisted)

**Owner**: claude. **Depends on**: H-action-replay (#826),
H-audit-query-extensions (#833) merged.

## Scope

When a session fails, the operator gets an audit chain ending
in `action.failed` + a stderr blob. Diagnosing it manually
takes time. Add `argo recover <session-id>` — LLM-assisted
analysis that:

- Walks the audit chain
- Identifies the failing step
- Reads stderr / stdout / tool-result blobs
- Calls Anthropic with context: brief + manifest + audit
  chain + failure detail
- Prints structured suggestions: likely cause, 3 candidate
  fixes (ranked by confidence), command snippets to try

NOT auto-applied. Suggestions only — operator decides.
Caches results per (session-id, audit-chain-hash) so
re-runs are free.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/recover.rs` (NEW)
- `crates/argo-cli/src/handlers/recover_prompt.rs` (NEW —
  prompt assembly logic for testability)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 2 pub mods
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly:
  register `Recover` subcommand
- `crates/argo-store/migrations/<next>__recovery_cache.sql`
  (NEW — caches per session-id + chain-hash)
- `crates/argo-cli/tests/recover_smoke.rs` (NEW —
  gated on `ANTHROPIC_API_KEY`)

## Acceptance criteria
- [ ] Identifies failing step from audit chain
- [ ] Suggestions returned in structured format (markdown)
- [ ] Cache hit returns identical suggestions w/o LLM call
- [ ] `--json` envelope universal
- [ ] No auto-apply of suggestions (advisory only)
- [ ] 8 unit tests (chain parsing + prompt assembly) + 1
      smoke gated on ANTHROPIC_API_KEY
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-error-recovery-suggestions`
- PR title: `cli: argo recover — LLM-assisted error recovery suggestions (claude-error-recovery-suggestions)`
