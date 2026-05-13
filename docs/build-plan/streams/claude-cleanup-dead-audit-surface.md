# claude-cleanup-dead-audit-surface — delete 5 unused AuditKind variants + 2 unused classifier methods

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> Audit cluster 2 found 5 `AuditKind` variants with ZERO production emission sites (`MarketplacePush`, `MarketplacePull`, `SessionReplay`, `LockSet`, `LockViolate`) at `crates/argo-audit/src/event.rs:40-48`. Plus `is_v02()` + `is_vertical()` classifier methods with zero callers anywhere.

## Scope

Delete the 5 variants from the enum + their `label()`/`from_label()`/`all()` arm references. Delete the 2 classifier methods. Chain self-hash is keyed on the LABEL — removing unused variants is safe (no historical row ever carried these).

**NOT in scope**: wiring marketplace push/pull audit emission (separate brief if operator wants it).

## Files you may touch
- `crates/argo-audit/src/event.rs` — narrowly: remove 5 variants + 2 method definitions. ≤40 modified lines.
- `crates/argo-audit/src/event_tests.rs` — narrowly: remove any tests for the deleted variants.
- `crates/argo-cli/src/handlers/audit_*.rs` — narrowly: any match arms over AuditKind that reference deleted variants get `_ => unreachable!()` removed OR exhaustive-match preserved (compile will tell you).

## Acceptance
- [ ] 5 variants + 2 methods removed.
- [ ] `cargo build --workspace` clean (exhaustive matches will surface any missed sites).
- [ ] Workspace clippy clean.
- [ ] Audit-chain verification tests still pass (the chain doesn't care which variants are defined).
- [ ] Net LOC -80+.

## Branch
- `claude-cleanup-dead-audit-surface`
