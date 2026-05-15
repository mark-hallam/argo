# mission-audit-chain-integrity-cli — argo audit verify-chain CLI for operator-driven integrity checks

**Owner**: codex. **Depends on**: nothing (consumes existing argo-audit hashing).
**Authored against**: origin/main `66d07a69` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-chain-integrity-cli`.

## Scope

The audit chain (`argo-audit`) hashes each event into a tamper-evident sequence. Currently the chain is verified on-startup (gateway boot check). Operators want an ad-hoc verification command: `argo audit verify-chain` that walks the chain from genesis to head and reports any break.

Use cases:
1. Post-incident triage: did anyone tamper with the audit log?
2. Migration sanity: confirm chain is intact after a DB restore.
3. Compliance reporting: emit a "chain-verified-at" timestamp + head sequence number.

LOC: ~200.

## Files you may touch

- NEW `crates/argo-cli/src/handlers/audit_verify_chain.rs` — handler. Walks `audit_record` rows via `argo-audit` chain-walker, reports OK/break with seq numbers.
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: `pub mod audit_verify_chain;` declaration. ≤3 lines.
- `crates/argo-cli/src/cli_subcommands_audit.rs` (or wherever audit subcommands live) — narrowly: add `VerifyChain` variant + dispatch. ≤20 modified lines.
- `crates/argo-cli/src/main_dispatch_audit.rs` (or sibling) — narrowly: dispatch arm. ≤15 modified lines.

## Files you must NOT touch

- `crates/argo-audit/` — consume the existing hashing API; do not modify the chain algorithm.
- argo-store audit_record schema — read-only consumer.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `argo audit verify-chain` walks the entire audit chain.
- [ ] Output: human-readable summary + optional `--json` envelope (`{"head_seq":12345,"verified":true,"break_at":null}`).
- [ ] If a break is found, exit code 1 + `break_at` populated.
- [ ] Performance: full chain walk for 100k rows < 5s.
- [ ] 3 unit tests: clean chain, synthetic break, empty chain.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-audit-chain-integrity-cli`
- PR title: `feat(audit): argo audit verify-chain CLI (mission-audit-chain-integrity-cli)`

## Notes for the implementer

- Mission-claim alignment: claim 5 (audit trail) + production hardening (operator integrity tools).
- Drift-prevention: extends argo-cli audit subcommand enum; no parallel surface.
