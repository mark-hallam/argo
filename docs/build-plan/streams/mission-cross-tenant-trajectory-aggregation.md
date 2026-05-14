# mission-cross-tenant-trajectory-aggregation — Sample-trajectory aggregation across tenants (with consent)

**Owner**: claude. **Depends on**: L11 MERGED (CU-recording-to-IR); existing argo-skill marketplace + per-tenant consent infrastructure.
**Authored against**: origin/main `7598d5d1` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cross-tenant-trajectory-aggregation`.

## Scope

MISSION_BACKLOG item 203. Cross-tenant learning loop: when a worker produces a novel successful trajectory, opt-in tenants can contribute the trajectory (with PII stripped) to a shared marketplace pool. Other tenants benefit from the resulting refined skills without seeing the source data. **Consent is opt-in per tenant** and per skill; PII redaction follows E1-e patterns; provenance + trust grade carried through.

LOC: ~1000.

## Files you may touch

- NEW `crates/argo-skill/src/trajectory_aggregation.rs` — opt-in collection + PII strip.
- NEW `crates/argo-skill/src/trajectory_aggregation_tests.rs`.
- NEW `crates/argo-skill/migrations/NNNN_trajectory_aggregation.sql` — opt-in flag per tenant per skill.
- `crates/argo-skill/src/marketplace.rs` — narrowly: aggregation feed into marketplace pool. ≤30 modified lines.

## Files you must NOT touch

- E1-e PII redaction internals (reuse); marketplace.rs core; tenant identity layer.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Opt-in flag CRUD per tenant + per skill.
- [ ] PII redaction applied before aggregation (test fixture with synthetic PII strips clean).
- [ ] Trajectory contributes to marketplace pool only when both opt-in flags set.
- [ ] Provenance + trust grade propagated.
- [ ] Tenant-isolation: no cross-leak when opt-out.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-cross-tenant-trajectory-aggregation`
- PR title: `Sample-trajectory aggregation across tenants (with consent) (mission-cross-tenant-trajectory-aggregation)`

## Notes for the implementer

- Mission-claim alignment: claim 4 (Skills are shared and learned, with provenance + trust).
- Authored from MISSION_BACKLOG.md [NEW] item — no prior brief existed; check `rg 'pub (struct|enum|fn) <Name>'` for any new names introduced before declaring (drift prevention rule).
