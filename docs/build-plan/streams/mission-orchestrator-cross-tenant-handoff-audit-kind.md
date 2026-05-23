# mission-orchestrator-cross-tenant-handoff-audit-kind

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-orchestrator-cross-tenant-handoff-audit-kind`.

🟢 **P2 CLAIM-6.** Add HandoffCrossTenantAttempt AuditKind variant.

## Scope

From #2091 internal docstring — cross-tenant orchestrator handoff attempts should emit an audit row so operators see attempted leaks even when scoping refuses them. New AuditKind variant + emission inside the existing refusal path.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- crates/argo-audit/src/event_kind.rs, crates/argo-orchestrator/src/persist_handoffs.rs
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Production source outside the may-touch list.

## Acceptance criteria

- [ ] Per-brief acceptance (see Scope above).
- [ ] Cross-tenant negative test where applicable.
- [ ] `cargo clippy` clean for affected crates.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-orchestrator-cross-tenant-handoff-audit-kind`
- PR title: `mission-orchestrator-cross-tenant-handoff-audit-kind`

## Notes for the implementer

- ~100-200 LOC expected.
- Cite the gap manifest verbatim in PR body.
