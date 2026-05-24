# eval-argo-marketplace-crate-surface

**Owner**: claude
**Priority**: P2 - marketplace substrate audit
**Authored against**: origin/main `ef55f5422` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-marketplace-crate-surface`.

## Source

The mission-vs-code gap manifest marks `argo-marketplace` as `NOT AUDITED`. The crate contains bundle, vertical bundle, registry, OCI auth, and OCI distribution surfaces that support skill/vertical distribution.

## Goal

Audit and pin the `argo-marketplace` public surface so bundle format, OCI distribution behavior, and registry contracts cannot drift silently.

## May Touch

- `crates/argo-marketplace/tests/surface_contract.rs` (NEW)
- `crates/argo-marketplace/tests/` (NEW sibling tests if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-marketplace-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production `argo-marketplace/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not contact a live OCI registry.
- Do not edit web marketplace UI or pricing copy.

## Acceptance

- [ ] Pin exported bundle, vertical bundle, registry, OCI auth, and OCI distribution APIs from `crates/argo-marketplace/src/lib.rs`.
- [ ] Cover bundle round-trip and vertical bundle round-trip behavior with existing fixture style.
- [ ] Cover OCI request/descriptor formatting without network calls.
- [ ] Static-grep current consumers and report live callsites versus dormant substrate.
- [ ] Write an eval report with verdict, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-marketplace --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Keep all registry interaction fake or fixture-backed. This is a substrate confidence brief, not an integration deployment.
