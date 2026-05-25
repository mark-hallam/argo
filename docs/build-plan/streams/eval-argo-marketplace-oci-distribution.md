# eval-argo-marketplace-oci-distribution — audit argo-marketplace OCI distribution path + tenant scoping

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-marketplace-oci-distribution`.

## Why this is P2

Mission gap manifest row `argo-marketplace` is 🔴 NOT AUDITED. Mission claim 4 distribution surface — skills/packs distributed via OCI (Open Container Initiative — same format Docker images use, repurposed here for skill artefacts) registries. Need to verify: who can push, who can pull, how tenant identity is bound, what authentication is used.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-marketplace/src/lib.rs`.
2. **Push path** — pub fn signature; tenant_id bind; auth (token? cert?); rate-limit (if any).
3. **Pull path** — same.
4. **Provenance trail** — does the marketplace record who pushed what when, in a way the install side can verify?
5. **Pin tests** at `crates/argo-marketplace/tests/oci_distribution_pin_test.rs` (~150-200 LOC).
6. **Report** at `docs/skycot/evals/2026-05-25-argo-marketplace-oci-distribution.md`.

## Files you may touch

- `crates/argo-marketplace/tests/oci_distribution_pin_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-marketplace-oci-distribution.md` — new.

## Files you must NOT touch

- `crates/argo-marketplace/src/**`.

## Acceptance criteria

- [ ] Surface table.
- [ ] Push/pull path tenant binds cited.
- [ ] Provenance trail yes/no with citations.
- [ ] Pin tests PASS.
- [ ] ≤5 follow-up briefs.

## Branch + PR

- Branch: `claude-eval-argo-marketplace-oci-distribution` or `codex-…`.
- PR title: `eval(marketplace): audit OCI distribution + tenant scoping (eval-argo-marketplace-oci-distribution)`.
