# mission-marketplace-multi-tag-semantics — Document + clarify multi-tag selection in marketplace

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `bfe849f4` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-marketplace-multi-tag-semantics`.

## Scope

Phase B audit P2 follow-up (`docs/skycot/audit/2026-05-14-phase-B-per-crate-deep-dive.md` §5 TODO #5). `argo-marketplace/marketplace_oci.rs:10` has a TODO: "Multi-tag handling v1-minimal: most-recent wins". When a marketplace skill has multiple tags (e.g., v1, v1.2, latest), the current implementation picks alphabetically-last. Operators see surprising behavior.

This brief: document the current semantics + add a config knob for tag-selection policy + improve the default to be timestamp-based instead of alphabetical.

LOC: ~100.

## Files you may touch

- `crates/argo-marketplace/src/marketplace_oci.rs` — narrowly: replace alphabetical-last with timestamp-newest selection. Add `MarketplaceTagPolicy` enum (`TimestampNewest`, `AlphabeticalLast`, `Explicit(String)`). ≤80 modified lines.
- `crates/argo-marketplace/src/marketplace_oci_tests.rs` (if exists, else new sibling) — test each policy variant. ≤50 modified lines.
- NEW `docs/skycot/operator/marketplace-tag-policy.md` — one-page operator-facing doc explaining the policy + how to override per-tenant.

## Files you must NOT touch

- OCI registry trait internals.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `MarketplaceTagPolicy` enum exists with 3 variants.
- [ ] Default is `TimestampNewest` (matches operator expectation when they push a new tag).
- [ ] Backward-compat: existing tests that asserted alphabetical pick are updated to either use `AlphabeticalLast` explicitly or assert new timestamp behavior.
- [ ] 3 unit tests covering each policy variant.
- [ ] Operator doc exists and is referenced from PRODUCT_ROADMAP or RUNBOOK.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-mission-marketplace-multi-tag-semantics`
- PR title: `feat(marketplace): timestamp-based multi-tag selection + policy knob (mission-marketplace-multi-tag-semantics)`

## Notes for the implementer

- Mission-claim alignment: claim 4 (Skills shared and learned — predictable tag resolution).
- This is post-v1 polish. Operator-decisional whether to backport.
- Drift-prevention: adds one new `pub enum MarketplaceTagPolicy` — no duplicate of an existing surface.


## Files you may touch — AUTHORITATIVE 2026-05-15 evening (supersedes original list)

Codex noted current OCI marketplace code is under `crates/argo-skill/src/marketplace_oci*`, not `crates/argo-marketplace`. Correcting:

- `crates/argo-skill/src/marketplace_oci.rs` — narrowly: add `MarketplaceTagPolicy` enum + timestamp-newest default. ≤80 modified lines.
- `crates/argo-skill/src/marketplace_oci_tests.rs` (or sibling) — narrowly: 3 unit tests per policy variant. ≤80 modified lines.
- NEW `docs/skycot/operator/marketplace-tag-policy.md` — operator doc.
