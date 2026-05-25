# eval-argo-siem-dispatch-surface — audit argo-siem dispatch surface + tenant routing

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-siem-dispatch-surface`.

## Why this is P2

Mission gap manifest row `argo-siem` is 🔴 NOT AUDITED. SIEM (Security Information and Event Management — central log/event collector consumed by enterprise security teams) dispatch was added via migration 103. Multi-tenant requirement: each tenant's SIEM destination (Splunk URL, Datadog API key, etc.) must be isolated; events for tenant A must never land in tenant B's SIEM.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-siem/src/lib.rs`.
2. **Dispatch path** — from event emission to outbound HTTP/queue post: trace the full path, document every tenant_id bind point.
3. **Per-tenant destination configuration** — find where tenant SIEM URLs/credentials are stored; cross-tenant leak vectors (env-var fallback, default destination, …).
4. **Pin tests** at `crates/argo-siem/tests/dispatch_surface_pin_test.rs` (~150-200 LOC) — surface + cross-tenant negative test.
5. **Report** at `docs/skycot/evals/2026-05-25-argo-siem-dispatch-surface.md`.

## Files you may touch

- `crates/argo-siem/tests/dispatch_surface_pin_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-siem-dispatch-surface.md` — new.

## Files you must NOT touch

- `crates/argo-siem/src/**`.

## Acceptance criteria

- [ ] Surface table.
- [ ] Per-tenant destination yes/no with citations.
- [ ] Cross-tenant negative test included (FINDINGS-ONLY `#[ignore]` if wire is broken).
- [ ] ≤5 follow-up briefs.

## Branch + PR

- Branch: `claude-eval-argo-siem-dispatch-surface` or `codex-…`.
- PR title: `eval(siem): audit argo-siem dispatch + tenant routing (eval-argo-siem-dispatch-surface)`.
