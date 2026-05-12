# Mission load-bearing wire — Zero-trust networking design for multi-host

**Owner**: claude. Design doc only (no code in this brief).
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #229. Single-host SKYCOT today implicitly trusts that gateway / workers / store all share localhost. The first multi-host deployment will discover dozens of latent trust assumptions. This brief is the design doc that documents the threat model + the migration path before code lands.

## Scope

NEW `docs/skycot/security/zero-trust-multi-host.md` (≤ 500 LOC) covering:

1. **Current trust assumptions** (audit of what today's code implicitly trusts about its peers — gateway↔worker, gateway↔store, store↔store, worker↔external API)
2. **Threat model** (adversary inside the network, adversary on a compromised worker, supply-chain compromise of the SKYCOT binary itself)
3. **Identity primitives** (mTLS via per-host x509, SPIFFE-style workload identity, or a simpler PSK-per-host) — recommend ONE primary with trade-offs of the alternatives
4. **Per-RPC authorization** model (capability tokens scoped to: caller-identity, target-resource, action, expiry)
5. **Migration sequence** (single-host stays unchanged; opt-in multi-host gates behind `[security.zero_trust] enabled = true`; per-tenant rollout)
6. **What this brief does NOT decide** — service mesh (Consul / Linkerd / Istio adapter is operator choice, not core SKYCOT)

No code; doc only.

## Files you may touch

- `docs/skycot/security/zero-trust-multi-host.md` (NEW, ≤ 500 LOC)

## Files you must NOT touch

- Any code; this is a design exercise.

## Acceptance criteria

- [ ] Design doc covers the 6 sections above
- [ ] Recommends ONE primary identity primitive with reasoning
- [ ] Names ≥3 specific code locations today that would need rework (e.g. `routes_auth.rs`, `argo-channels::token`, etc.)
- [ ] Lists ≥5 follow-up briefs that implement the design
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-zero-trust-multi-host-design`
- PR title: `security: zero-trust multi-host design doc (mission-zero-trust-multi-host-design)`

## Notes

- This is the BLUEPRINT, not the implementation. The doc names which briefs follow (capabilities-on-RPCs, per-host x509 issuance, workload-identity bootstrap, etc.) but doesn't implement them.
- mTLS via cert-manager is the well-trodden path. SPIFFE is more powerful but operationally heavier. PSK is simplest but doesn't survive rotation well. Pick one; defend the pick.
- Document explicitly that single-host operators are unaffected — `enabled = false` (default) means the design here is opt-in.
