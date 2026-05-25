# eval-argo-replay-cross-tenant-isolation — audit argo-replay surface + cross-tenant audit replay

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-replay-cross-tenant-isolation`.

## Why this is P2

Mission gap manifest row `argo-replay` is 🔴 NOT AUDITED — flagged "re-verify" because #1992 closed a prior cross-tenant fix on `audit_detail`. Replay surface lets operators reconstruct a session from audit records; if tenant_id is dropped at any step, an operator could reconstruct another tenant's session.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-replay/src/lib.rs`.
2. **Replay entry points** — every `pub fn` that takes session_id / audit_id; confirm each binds `tenant_id` from the caller principal.
3. **`#1992` regression pin** — re-verify the cross-tenant fix is still in place; cite the line that enforces the check.
4. **Negative test** — attempt to replay tenant_a's session with tenant_b's principal; assert NotFound (not Forbidden, to avoid existence leak).
5. **Pin tests** at `crates/argo-replay/tests/cross_tenant_isolation_test.rs` (~150-200 LOC).
6. **Report** at `docs/skycot/evals/2026-05-25-argo-replay-cross-tenant-isolation.md`.

## Files you may touch

- `crates/argo-replay/tests/cross_tenant_isolation_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-replay-cross-tenant-isolation.md` — new.

## Files you must NOT touch

- `crates/argo-replay/src/**`.

## Acceptance criteria

- [ ] Surface table.
- [ ] `#1992` regression line cited.
- [ ] Cross-tenant negative test PASSes (NotFound on wrong-tenant lookup).
- [ ] ≤5 follow-up briefs.

## Branch + PR

- Branch: `claude-eval-argo-replay-cross-tenant-isolation` or `codex-…`.
- PR title: `eval(replay): audit argo-replay cross-tenant isolation (eval-argo-replay-cross-tenant-isolation)`.
