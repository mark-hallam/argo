# mission-skill-oci-pagination — OCI catalog pagination beyond 100 items

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `93f662b6` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-oci-pagination`.

## Scope

Phase B audit (TODO #1 from `docs/skycot/audit/2026-05-14-phase-B-per-crate-deep-dive.md` §5) flagged: `argo-skill/marketplace_oci.rs:9` carries a TODO comment "Pagination beyond first `_catalog` page is TODO". Current behavior: OCI registry listing defaults to 100 items; no multi-page support. Blocks skill discovery for registries with 100+ skills.

Implementation: extend the OCI catalog reader to follow the `Link: <...>; rel="next"` header per [OCI Distribution Spec §6](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#listing-repositories). Paginate transparently; expose all results to the caller. Add a configurable cap (default 1000) to prevent runaway pagination.

LOC: ~350.

## Files you may touch

- `crates/argo-skill/src/marketplace_oci.rs` — narrowly: add `fetch_all_pages()` helper that loops with Link-header following. Replace single-page fetcher's call sites. ≤200 modified lines.
- `crates/argo-skill/src/marketplace_oci_tests.rs` (if exists, else create) — test multi-page fetch with mock HTTP responses returning Link header. ≤150 modified lines.

## Files you must NOT touch

- The OciRegistry trait surface (extend, don't break).
- argo-marketplace (this is consumer-side pagination, not registry-side).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `fetch_all_pages()` follows Link headers until exhausted or cap hit.
- [ ] Default cap = 1000 items; configurable via `[marketplace] max_catalog_items`.
- [ ] Single-page test still passes.
- [ ] Multi-page test: 3 mock pages × 100 items each = 300 results.
- [ ] Cap test: 5 mock pages × 100 = 500 but cap = 200 → 200 results + warning log.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-skill` passes.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-mission-skill-oci-pagination`
- PR title: `feat(skill): OCI catalog pagination beyond 100 items (mission-skill-oci-pagination)`

## Notes for the implementer

- Mission-claim alignment: claim 4 (Skills shared+learned — enables enterprise multi-skill registries 100+).
- Drift-prevention: extends existing `argo-skill::marketplace_oci` surface; no parallel implementation.
