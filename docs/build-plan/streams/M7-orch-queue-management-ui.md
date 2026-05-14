# M7-orch-queue-management-ui - Track M (M-Orch), Phase 1, Tier 2

**Owner**: claude. **Depends on**: M5 MERGED; M6 MERGED; E0 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M7-orch-queue-management-ui`.

## Scope

/orchestration route: list of in-flight + queued runs across workers. Filters: worker / skill / SLA state / status. SLA cards: live latency vs target with breach indicator. Retry badges. Operator actions: cancel run, reset retry, force-escalate.

LOC: ~1100. Branch: `claude-M7-orch-queue-management-ui`. PR title: `feat(web/orch): queue management UI (M7)`.

## Files you may touch

- NEW web/app/(app)/orchestration/page.tsx (<=350)
- NEW web/components/skycot/orch/QueueTable.tsx (<=350)
- NEW web/components/skycot/orch/SlaCard.tsx (<=250)
- NEW web/components/skycot/orch/RetryBadge.tsx (<=100)
- NEW web/lib/api/orchestration.ts
- NEW crates/argo-cli/src/handlers/gateway/routes_orchestration.rs - gateway endpoints

## Files you must NOT touch

- M5/M6 backend (consume APIs, dont change).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Route loads, shows >= 50 mock runs cleanly.
- [ ] All filters work.
- [ ] Operator actions hit gateway.
- [ ] 8 unit tests + 1 e2e.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M7-orch-queue-management-ui`
- PR title: `feat(web/orch): queue management UI (M7)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 2 (Delegation observability).
