# mission-genesis-fallback-audit-kind

**Authored against**: `origin/main` `bf8c69f82` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-fallback-audit-kind`.

**Fast-mode eligible**: yes — clone-and-extend of the canonical 4-site recipe.

## Summary

Companion to `web-hire-loud-failure-banner` (Phase 2 of the hire-bug split). When the SPA's Genesis call falls back to `STUB_QUESTIONS` (network / 5xx / schema-mismatch), emit `AuditKind::GenesisFallbackUsed` so operators can see in `/audit` when their workspace's Genesis is failing. Single audit-kind variant + one emit site.

## Acceptance

- [ ] `AuditKind::GenesisFallbackUsed` variant added in `crates/argo-audit/src/event.rs` (4-site recipe: enum variant + `as_kind_str` + `from_kind_str` + `ALL_KINDS`).
- [ ] Variant count assertion in `event_tests.rs` bumped (current count post-#1949 + new #1947/#1948 lands — verify the live number at claim time).
- [ ] String label: `"genesis.fallback_used"`.
- [ ] Emit site: the gateway hire endpoint (`crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`) writes this audit row IF the request body carries a `"genesis_fallback": true` marker (client-set when SPA used stub).
- [ ] SPA hire POST (`web/lib/api/workers.ts`) sets `"genesis_fallback": true` when the worker draft came from the stub. Pre-condition: `web-hire-loud-failure-banner` MERGED (the `source: 'gateway' | 'stub'` signal lands there).
- [ ] One integration test in `routes_workers_hire_tests.rs` covering: request with `genesis_fallback=true` → 1 audit row with `kind="genesis.fallback_used"`.
- [ ] No semantic change to the existing hire flow.

## May touch

- `crates/argo-audit/src/event.rs` — new variant (4-site recipe)
- `crates/argo-audit/src/event_tests.rs` — count assertion + label round-trip
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` — emit site
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire_tests.rs` — integration test
- `web/lib/api/workers.ts` — pass-through of `genesis_fallback` flag

## Must NOT touch

- The fallback UI (Phase 2 brief owns the banner + retry).
- The schema-mirror logic (Phase 1 brief settles).
- The idempotency_key path (Phase 3 brief).
- Other AuditKind variants.

## Sizing

80–150 LOC. 4-site recipe + emit site + 1 test.

## Notes

- Pre-condition: `web-hire-loud-failure-banner` MERGED. The audit gets emitted when the SPA tells the gateway it used the stub, which only matters once the banner makes that signal visible.
- Pattern reference: #1920 (`mission-onboarding-audit-variants`) for the cleanest recent 4-site recipe diff.
- Operator memory: claim-2 procurement story benefits from this — operators can prove Genesis is reliable for their workspace via audit-chain visibility.
