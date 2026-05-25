# eval-argo-forge-promote-fail-closed-path — audit argo-forge promote path + judge fail-closed contract

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-forge-promote-fail-closed-path`.

## Why this is P2

Mission gap manifest row `argo-forge` is 🔴 NOT AUDITED. Per MISSION.md status: "Judge is consulted by forge promote (fail-CLOSED — promotion blast-radius is larger)". Forge promotes a candidate worker variant to live; if the judge fail-closed contract is broken, a low-quality variant could promote without verification.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-forge/src/lib.rs`.
2. **Promote path** — find `pub fn promote_*`; trace to the judge call site; document the fail-closed branch (judge result missing or error → promote refused).
3. **Tenant scoping** — confirm promote is tenant-bound; worker_a in tenant_a cannot promote a variant of worker_b in tenant_b.
4. **Pin tests** at `crates/argo-forge/tests/promote_fail_closed_test.rs` (~150-200 LOC) — fail-closed contract + cross-tenant negative.
5. **Report** at `docs/skycot/evals/2026-05-25-argo-forge-promote-fail-closed-path.md`.

## Files you may touch

- `crates/argo-forge/tests/promote_fail_closed_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-forge-promote-fail-closed-path.md` — new.

## Files you must NOT touch

- `crates/argo-forge/src/**`.

## Acceptance criteria

- [ ] Surface table.
- [ ] Fail-closed branch cited with line number.
- [ ] Cross-tenant negative test PASSes.
- [ ] ≤5 follow-up briefs.

## Branch + PR

- Branch: `claude-eval-argo-forge-promote-fail-closed-path` or `codex-…`.
- PR title: `eval(forge): audit promote path + judge fail-closed (eval-argo-forge-promote-fail-closed-path)`.
