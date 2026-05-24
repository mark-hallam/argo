# eval-argo-config-crate-surface

**Owner**: claude
**Priority**: P3 - config substrate audit
**Authored against**: origin/main `ef55f5422` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-config-crate-surface`.

## Source

The mission-vs-code gap manifest marks `argo-config` as `NOT AUDITED`. The crate owns workspace and tenant-facing configuration sections, provider config, credentials config, loader overrides, and defaults used across the platform.

## Goal

Audit and pin the `argo-config` public surface so defaults, section schemas, source tracking, and override precedence cannot drift silently.

## May Touch

- `crates/argo-config/tests/surface_contract.rs` (NEW)
- `crates/argo-config/tests/` (NEW sibling tests if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-config-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production `argo-config/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not edit user config files outside temp fixtures.
- Do not change config semantics in this audit brief.

## Acceptance

- [ ] Pin exported public modules, config structs, and default constructors from `crates/argo-config/src/lib.rs`.
- [ ] Cover loader precedence for file/env/override sources using temp fixtures.
- [ ] Cover at least five mission-relevant sections: gateway, auth, telemetry, siem, executor, planner, or worker resource config.
- [ ] Cover source-tracking behavior if exposed by the current API.
- [ ] Write an eval report with verdict, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-config --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Keep this audit-only. Config default changes can have wide blast radius and should be separate fix briefs.
