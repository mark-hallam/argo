# eval-argo-eval-runner-surface — audit argo-eval framework surface + tenant scoping

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-eval-runner-surface`.

## Why this is P2

Mission gap manifest row `argo-eval` is 🔴 NOT AUDITED. The eval framework runs scoring suites against worker outputs; mission claim 4 (skills shared + learned) depends on it for skill-quality scoring. Need to confirm runner surface and tenant scoping.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-eval/src/lib.rs`.
2. **Runner entry points** — pub fns that execute an eval suite; document each parameter set.
3. **Tenant scoping** — does each entry point require `tenant_id`? Can eval results from tenant_a be read by tenant_b?
4. **Result storage** — where are eval results stored? Is the storage table tenant-scoped?
5. **Pin tests** at `crates/argo-eval/tests/runner_surface_pin_test.rs` (~150-200 LOC).
6. **Report** at `docs/skycot/evals/2026-05-25-argo-eval-runner-surface.md`.

## Files you may touch

- `crates/argo-eval/tests/runner_surface_pin_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-eval-runner-surface.md` — new.

## Files you must NOT touch

- `crates/argo-eval/src/**`.

## Acceptance criteria

- [ ] Surface table.
- [ ] Tenant scoping yes/no per entry point.
- [ ] Pin tests PASS.
- [ ] ≤5 follow-up briefs.

## Branch + PR

- Branch: `claude-eval-argo-eval-runner-surface` or `codex-…`.
- PR title: `eval(eval): audit argo-eval runner surface + tenant scoping (eval-argo-eval-runner-surface)`.
