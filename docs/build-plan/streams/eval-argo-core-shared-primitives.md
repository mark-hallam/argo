# eval-argo-core-shared-primitives — audit argo-core shared primitives surface

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-core-shared-primitives`.

## Why this is P2

Mission gap manifest row `argo-core` is 🔴 NOT AUDITED. Shared primitives used across the workspace — likely `TenantId`, `WorkerId`, `SessionId`, `Principal`, capability types. Audit chain history (2026-05-13/14) found 11+ duplicate `pub struct/enum` definitions; `argo-types` was chosen as the canonical home but `argo-core` may still carry parallel surfaces. Need an audit pin to prevent future drift.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-core/src/lib.rs` — all pub items.
2. **Parallel-surface check** — for each pub type, grep `crates/` for same-named or near-same-named types in other crates. Document every parallel.
3. **Canonical home** — for each parallel, identify whether `argo-core` or another crate is canonical; flag the loser.
4. **Pin tests** at `crates/argo-core/tests/surface_inventory_pin_test.rs` (~150 LOC) — re-export pins + a CI-hookable inventory list.
5. **Report** at `docs/skycot/evals/2026-05-25-argo-core-shared-primitives.md`.

## Files you may touch

- `crates/argo-core/tests/surface_inventory_pin_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-core-shared-primitives.md` — new.

## Files you must NOT touch

- `crates/argo-core/src/**`.

## Acceptance criteria

- [ ] Surface table.
- [ ] Parallel-surface table with canonical-home column.
- [ ] Pin tests PASS.
- [ ] ≤5 follow-up briefs (consolidation candidates).

## Branch + PR

- Branch: `claude-eval-argo-core-shared-primitives` or `codex-…`.
- PR title: `eval(core): audit argo-core shared primitives + parallels (eval-argo-core-shared-primitives)`.
