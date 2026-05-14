# claude-canonicalize-decision-type — Resolve Decision duplicate (argo-cli vs argo-identity)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-canonicalize-decision-type`.

## Scope

Phase A audit found `Decision` defined in 2 crates:
- `crates/argo-identity/src/auto_approval.rs:69` — `pub enum Decision` (auto-approval policy decision: Allow/Ask/Deny)
- `crates/argo-cli/src/handlers/handoff_policy.rs:91` — `pub enum Decision` (worker handoff decision — likely different domain)

These are **probably different concepts**, not a duplicate to merge. The fix is renaming to disambiguate. Read both definitions, confirm domain split, then:

- If same concept: canonicalize to `argo-identity::Decision` (it owns approval policy); `argo-cli` imports it.
- If different concepts (likely): rename `argo-cli`'s version to `HandoffDecision` and `argo-identity`'s to `AutoApprovalDecision` for clarity. Or pick the broader name for the more central one.

LOC: ~300.

## Files you may touch

- `crates/argo-identity/src/auto_approval.rs` — possibly rename `Decision` → `AutoApprovalDecision`. ≤20 modified lines.
- `crates/argo-cli/src/handlers/handoff_policy.rs` — possibly rename `Decision` → `HandoffDecision`. ≤20 modified lines.
- All call sites of either type (use `rg ': Decision[ ,)>]'` and `rg 'Decision::'` to find).
- Tests touching either type.
- `crates/argo-types/src/lib.rs` — IF concepts are the same, move canonical here. ≤20 modified lines.

## Files you must NOT touch

- argo-identity's auto-approval policy logic (behavior unchanged; rename only).
- argo-cli's handoff-policy logic (behavior unchanged; rename only).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `rg 'pub enum Decision\b' crates/` returns at most ONE match (or zero if both renamed).
- [ ] Disambiguated names are stable and reflect the domain (AutoApprovalDecision, HandoffDecision).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-canonicalize-decision-type`
- PR title: `refactor: disambiguate Decision enum (auto-approval vs handoff) (claude-canonicalize-decision-type)`

## Notes for the implementer

- Mission-claim alignment: production hardening (type clarity; prevents accidental mis-typing).
- **First step is the audit:** read both enum bodies, compare variants. If variants differ → they are different concepts → rename both. If variants are identical → consolidate to one canonical home (likely argo-types since both crates can depend on it).
- Drift-prevention rule applied: brief removes a parallel surface, doesn't add any.
