# eval-argo-replay-find-orphaned-references-after-1992 — confirm #1992 fix has no residual cross-tenant call sites

**Owner**: claude or codex. **Depends on**: `eval-argo-replay-cross-tenant-isolation` (the surface audit) — that brief produces the surface table this one uses.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-replay-find-orphaned-references-after-1992`.

## Why this is P3

#1992 closed a cross-tenant `audit_detail` reconstruction path in argo-replay. The gap manifest flags `argo-replay` for "re-verify" because the original fix was localised — we want to confirm no other call site still passes `audit_id` without `tenant_id`.

## Scope (read-only)

1. After `eval-argo-replay-cross-tenant-isolation` produces a surface table, grep the workspace for every callsite of every replay entry point.
2. For each callsite, document the `tenant_id` source. Flag any callsite where tenant_id is hardcoded, defaulted, or absent.
3. Cross-check the #1992 PR diff: confirm the fix landed in every callsite at that time + check whether new callsites have been added since.
4. Pin tests at `crates/argo-replay/tests/orphan_callsite_pin_test.rs` (~100 LOC) that grep at compile time using `include_str!` on key files + assert no patterns match the orphan shape.
5. Report at `docs/skycot/evals/2026-05-25-argo-replay-orphan-references-post-1992.md`.

## Files you may touch

- `crates/argo-replay/tests/orphan_callsite_pin_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-replay-orphan-references-post-1992.md` — new.

## Files you must NOT touch

- `crates/argo-replay/src/**` — read-only.
- Other crate sources.

## Acceptance criteria

- [ ] Callsite table covers every replay pub fn.
- [ ] Orphan-shape pin test PASSes.
- [ ] Report covers #1992 + post-#1992 deltas.
- [ ] ≤3 follow-up briefs if orphans found.

## Branch + PR

- Branch: `claude-eval-argo-replay-find-orphaned-references-after-1992` or `codex-…`.
- PR title: `eval(replay): orphan callsite sweep post-#1992 (eval-argo-replay-find-orphaned-references-after-1992)`.

## Notes

- Depends on the partner audit `eval-argo-replay-cross-tenant-isolation` — claim partner first.
- Read-only audit.
