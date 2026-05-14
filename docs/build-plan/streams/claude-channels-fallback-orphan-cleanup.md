# claude-channels-fallback-orphan-cleanup — Remove or wire orphan `FallbackChain::with_operator_preferred`

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-channels-fallback-orphan-cleanup`.

## Scope

Phase B audit found one confirmed orphan in the workspace: `FallbackChain::with_operator_preferred()` at `crates/argo-channels/src/fallback.rs:88`. Pub method defined; zero call sites; intended to integrate operator preferences into fallback channel order.

Two paths — implementer reads the method body + recent git history (`git log --follow crates/argo-channels/src/fallback.rs`), then chooses:

- **Path A (remove)**: if operator-preferences integration is out of v1 scope, delete the method + any helper types it uses. Document the removal in v1.x backlog as `mission-channels-operator-preferred-fallback` for future re-introduction.
- **Path B (wire)**: complete the operator-preferences resolver in `argo-identity` (or wherever preferences live) and wire it through to `argo-channels`. ~150 LOC.

LOC: 50 (Path A) or 150 (Path B).

## Files you may touch

- `crates/argo-channels/src/fallback.rs` — narrowly: remove method (Path A) or wire it (Path B). ≤50 modified lines either way.
- `crates/argo-channels/src/fallback_tests.rs` (if exists) — remove orphan tests (Path A) or add wired-path tests (Path B). ≤30 modified lines.
- Path B only: `crates/argo-identity/src/policy.rs` or similar — narrowly: expose operator-preferences accessor. ≤30 modified lines.
- Path A only: NEW `docs/build-plan/streams/mission-channels-operator-preferred-fallback.md` (deferred-v1.x placeholder).

## Files you must NOT touch

- argo-channels approval-flow logic (separate concern; this is fallback chain only).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `rg 'with_operator_preferred' crates/` returns either zero matches (Path A) OR matches with active call sites (Path B).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-channels` passes.
- [ ] PR description states which path was taken + why.
- [ ] If Path A: a follow-up brief stub exists for future re-introduction.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-channels-fallback-orphan-cleanup`
- PR title: `cleanup: remove or wire FallbackChain::with_operator_preferred orphan (claude-channels-fallback-orphan-cleanup)`

## Notes for the implementer

- Mission-claim alignment: production hardening (eliminates one of the Phase B confirmed orphans).
- Phase B audit report: `docs/skycot/audit/2026-05-14-phase-B-per-crate-deep-dive.md` §2.
- Read the method body first; if it's complete and ready to wire (Path B), prefer that. If it's stubby/aspirational, prefer removal (Path A).
- Drift-prevention: brief removes one parallel surface; adds zero new pub types.
