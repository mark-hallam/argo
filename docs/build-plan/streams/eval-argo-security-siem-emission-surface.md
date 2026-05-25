# eval-argo-security-siem-emission-surface — audit argo-security crate surface + SIEM event emission

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-security-siem-emission-surface`.

## Why this is P2

Mission gap manifest row `argo-security` is 🔴 NOT AUDITED. Likely owns SIEM event emission. Need to confirm: (a) what `pub` surface exists, (b) what events fire and when, (c) whether emission is tenant-scoped, (d) whether downstream `argo-siem` actually receives the events. Mission claim 6 (multi-tenant) and the audit-trail / SOC-2-ready compliance claim on `/pricing` both depend on this substrate.

## Scope (read-only audit)

1. **Surface inventory** — `crates/argo-security/src/lib.rs` pub re-exports + all `pub fn` / `pub struct` / `pub enum`.
2. **Event-emission catalogue** — every place where a security event is constructed; document the tenant_id bind (or absence) for each.
3. **Downstream wire** — find the consumer (`argo-siem`?) and document the channel (function call, queue, async sink).
4. **Cross-tenant pin test** — write a `crates/argo-security/tests/surface_pin_test.rs` (~150 LOC) covering surface + tenant binding (FINDINGS-ONLY `#[ignore]` allowed for known wire gaps).
5. **Report** at `docs/skycot/evals/2026-05-25-argo-security-siem-emission-surface.md` (~600-1000 words).

## Files you may touch

- `crates/argo-security/tests/surface_pin_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-security-siem-emission-surface.md` — new.

## Files you must NOT touch

- `crates/argo-security/src/**`.
- `crates/argo-siem/src/**`.

## Acceptance criteria

- [ ] Surface table covers all `pub` items in `argo-security::lib`.
- [ ] Event catalogue with line citations for every emission site.
- [ ] Pin tests compile + run (`cargo test -p argo-security`).
- [ ] Report ends with ≤5 follow-up brief candidates.

## Branch + PR

- Branch: `claude-eval-argo-security-siem-emission-surface` or `codex-…`.
- PR title: `eval(security): audit argo-security surface + SIEM emission (eval-argo-security-siem-emission-surface)`.

## Notes

- ~150-200 LOC test code + report.
- Read-only audit.
