# claude-cli-handlers-audit-split — split argo-cli/src/handlers/audit.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main dd9362a0 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/audit.rs` is at 446/500 LOC. Pre-emptive split keeps headroom for future feature briefs (audit is a heavily-extended surface — recently grew with new audit kinds).

Note: distinct from `claude-cli-handlers-audit-export-split` which targets a different subset of the same file. Coordinate with that brief if both land concurrently.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by audit subcommand (e.g. list vs show vs export) or by AuditKind dispatch. Read the file end-to-end before deciding the cut.
2. **Extract to sibling** `audit_<topic>.rs` in the same directory.
3. **Re-export via `pub(crate) use`** so dispatcher import surface stays unchanged.
4. **Keep public API unchanged.**
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/audit.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/audit_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `audit.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-handlers-audit-split`
- PR title: `argo-cli: split handlers/audit.rs below 500-LOC ceiling (claude-cli-handlers-audit-split)`
