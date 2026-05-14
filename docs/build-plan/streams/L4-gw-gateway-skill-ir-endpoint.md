# L4-gw — `gateway-skill-ir-endpoint` (Track L Phase 1, Tier 1, sibling to L4)

**Owner**: claude. **Depends on**: L2 MERGED.
**Authored against**: origin/main `0ec56ca5` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L4-gw-gateway-skill-ir-endpoint`.

## Scope

Gateway endpoints serving Skill IR JSON to the L4 canvas viewer. LOC ~500. Branch: `claude-L4gw-gateway-skill-ir`. PR title: `feat(gateway): GET /api/v1/skills/<id>/ir + /versions (L4-gw)`.

**Specifics (verbatim from L0 spec)**:

- `GET /api/v1/skills/<skillId>/ir?version=<v>` — returns IR JSON.
- `GET /api/v1/skills/<skillId>/ir/versions` — returns version list with hashes.
- Tenant-scoped via bearer token.
- Audit-emitting (read access logged via existing audit-chain machinery).
- 404 on missing skill or missing version.
- 403 on cross-tenant access.

## Files you may touch

- NEW `crates/argo-cli/src/handlers/gateway/routes_skill_ir.rs` — handler.
- NEW `crates/argo-cli/src/handlers/gateway/routes_skill_ir_tests.rs` — tests.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_workers.rs` or appropriate dispatch sibling — register routes. ≤10 modified lines.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: `mod` declaration. ≤3 modified lines.

## Files you must NOT touch

- L1/L2 surfaces — read-only.
- Web/frontend — locked (L4 owns).
- Any audit-chain machinery — use existing `AuditSink`.

## Acceptance criteria

- [ ] Both endpoints return valid IR JSON with TS-compatible camelCase field names.
- [ ] 404 on missing skill or missing version.
- [ ] 403 on cross-tenant access.
- [ ] 5 integration tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L4gw-gateway-skill-ir`
- PR title: `feat(gateway): GET /api/v1/skills/<id>/ir + /versions (L4-gw)`

## Notes for the implementer

- Sibling to L4; can land in parallel once L2 merged.
- Mount under existing gateway dispatch — note routes_dispatch_get.rs was split into per-domain siblings via #1601; pick the right one.
- See `streams/L0-skill-canvas-track-overview.md` for full track context.
