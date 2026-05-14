# claude-canonicalize-approval-decision-type — Make argo-channels::ApprovalDecision canonical

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-canonicalize-approval-decision-type`.

## Scope

Phase A audit found `ApprovalDecision` defined in 2 crates:
- `crates/argo-channels/src/token.rs:14` — `pub enum ApprovalDecision` (approval workflow type)
- `crates/argo-store/src/approvals.rs:144` — `pub enum ApprovalDecision` (persistence type)

Both are the same concept (a worker-requested approval that came back Allow/Deny/Defer). Currently the transport (argo-channels) and persistence (argo-store) have parallel definitions; risk of silent serialization mismatch.

**Canonical = argo-channels::ApprovalDecision** because: channels owns the approval-workflow domain; store is a sink. Alternative: move to `argo-types` so both layers depend on it cleanly. Implementer chooses; both are acceptable. **Prefer argo-types if either crate already depends on it** (likely).

LOC: ~300.

## Files you may touch

- Canonical site: either `crates/argo-channels/src/token.rs` (keep as-is) OR `crates/argo-types/src/lib.rs` (relocate). Pick one. ≤30 modified lines on canonical.
- Other crate's local def: replace with `use <canonical>::ApprovalDecision`. ≤30 modified lines.
- All call sites (use `rg 'ApprovalDecision'` to enumerate).
- Cargo.toml dep updates if the canonical-home choice requires a new dep edge. ≤5 modified lines.

## Files you must NOT touch

- argo-channels's approval workflow logic.
- argo-store's approval-table schema migrations.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `rg 'pub enum ApprovalDecision' crates/` returns exactly ONE match.
- [ ] Both crates reference the canonical type via `use`.
- [ ] Variant set is the union (no information lost from either prior definition).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-canonicalize-approval-decision-type`
- PR title: `refactor: canonicalize ApprovalDecision; remove duplicate (claude-canonicalize-approval-decision-type)`

## Notes for the implementer

- Mission-claim alignment: claim 6 (multi-channel approval gates — type consistency between transport and persistence layers).
- Verify variant equivalence before consolidating. If argo-store's version carries a persistence-only field (e.g., `decided_at_micros`), keep that on a wrapper struct in argo-store, not on the canonical enum.
- Drift-prevention rule applied: brief removes a parallel surface, doesn't add any.
