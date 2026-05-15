# mission-rtbf-dreams-propagation — Right-to-be-forgotten propagation through Dreams output

**Owner**: claude. **Depends on**: existing argo-dreams + argo-skill marketplace.
**Authored against**: origin/main `7598d5d1` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-rtbf-dreams-propagation`.

## Scope

MISSION_BACKLOG item 235. When a tenant exercises right-to-be-forgotten (GDPR Article 17), all personal data must be propagated through the Dreams (distillation) pipeline: any skills derived from that tenant's data are re-derived without it, or removed if not re-derivable. This brief implements the RTBF cascade through Dreams output paths.

LOC: ~600.

## Files you may touch

- NEW `crates/argo-skill/src/rtbf_propagation.rs` — RTBF cascade through Dreams + skill marketplace.
- NEW `crates/argo-skill/src/rtbf_propagation_tests.rs`.
- `crates/argo-cli/src/handlers/tenant_rtbf.rs` — TOUCH. Wire propagation into existing RTBF CLI (if exists; verify at claim time). ≤20 modified lines.

## Files you must NOT touch

- argo-dreams pipeline internals (consume APIs); existing tenant deletion path.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] RTBF on tenant A removes A's contributed trajectories from Dreams output.
- [ ] Skills derived solely from A's data are removed; skills with multi-tenant input are re-derived without A.
- [ ] Idempotent: re-running produces no further changes.
- [ ] Audit chain emits per-skill RTBF action.
- [ ] 5 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-rtbf-dreams-propagation`
- PR title: `Right-to-be-forgotten propagation through Dreams output (mission-rtbf-dreams-propagation)`

## Notes for the implementer

- Mission-claim alignment: claim 4 (Skills are shared and learned + compliance: GDPR Article 17).
- Authored from MISSION_BACKLOG.md [NEW] item — no prior brief existed; check `rg 'pub (struct|enum|fn) <Name>'` for any new names introduced before declaring (drift prevention rule).


## Files you may touch — AUTHORITATIVE 2026-05-15 evening (supersedes original list)

Codex noted brief targets nonexistent `crates/argo-cli/src/handlers/tenant_rtbf.rs`. Current GDPR/delete surfaces live under `crates/argo-cli/src/handlers/gdpr_delete.rs`. Correcting:

- NEW `crates/argo-skill/src/rtbf_propagation.rs` — RTBF cascade through Dreams + skill marketplace. ≤300 LOC.
- NEW `crates/argo-skill/src/rtbf_propagation_tests.rs` — unit tests. ≤200 LOC.
- `crates/argo-cli/src/handlers/gdpr_delete.rs` — narrowly: wire propagation into existing GDPR delete handler. ≤30 modified lines.
- `crates/argo-skill/src/lib.rs` — narrowly: `pub mod rtbf_propagation;`. ≤3 lines.
- Optional: NEW `crates/argo-skill/migrations/NNNN_rtbf_propagation.sql` (if a tombstone schema is needed).
