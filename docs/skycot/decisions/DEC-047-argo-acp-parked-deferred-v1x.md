# DEC-047 — `argo-acp` stays parked (DEFERRED-v1.x), reason recorded

- **Date:** 2026-06-10
- **Status:** **active** — operator-decided 2026-06-10 ("for the argo-acp keep it
  parked, with the reason written down"), after the 4-agent codebase audit
  flagged `argo-acp` as an orphan crate (~1,092 LOC, zero workspace consumers)
  whose `zero-caller-allowed` marker carried no recorded rationale.
- **Owner:** conductor (Claude); applies to anyone tempted to wire or delete the crate.

## 1. What `argo-acp` is

A complete, tested server library for the **Agent Client Protocol (ACP)** — the
standard that lets developer tools (VS Code, Cursor, Zed, JetBrains, Copilot
CLI) plug an external agent into their UI. It implements JSON-RPC wire
envelopes, multi-harness capability negotiation, auth (Copilot OAuth / Cursor
token / generic bearer → `Principal::Acp`), permission mapping onto
`argo_capability::CapabilityEngine`, and server dispatch for the four advertised
methods. Surfaces are pinned by `crates/argo-acp/tests/surface_contract.rs`.

The crate is **finished but unwired**: the `argo acp serve` CLI subcommand that
would switch it on was never added, so nothing in the workspace depends on it.

## 2. Decision — keep it parked (DEFERRED-v1.x), do NOT wire and do NOT delete

First-principles reasoning the operator endorsed:

- **Not in scope now.** Editor/IDE integration is not part of the paying-ready
  sprint (DEC-034). Wiring `argo acp serve` would add a **new externally-reachable
  surface to secure and maintain** for **zero current users** — the wrong trade
  while the public-preview perimeter is still being hardened.
- **Plausible later, for the right buyer.** "Drive your SKYCOT teammate from
  inside your editor" is a credible enterprise feature; the work is done and
  shouldn't be thrown away.
- **Carrying cost is low.** The library is complete and surface-pinned, so it
  just compiles; the drift risk is bounded by the existing pin test.
- **Therefore:** neither wire (adds attack surface + maintenance for no user) nor
  delete (destroys finished, tested, roadmap-plausible work). Park it, with the
  reason recorded here and on the marker — this is the version we don't revisit.

## 3. Review trigger (when to un-park)

Promote `argo-acp` from parked to wired (a `argo acp serve` brief + perimeter
review) when EITHER:
1. editor/IDE integration becomes an explicit sprint/roadmap item, OR
2. a concrete external ACP-harness consumer (a paying or pilot customer driving a
   teammate from their editor) is confirmed.

Until then the `zero-caller-allowed` marker in `crates/argo-acp/Cargo.toml`
points here, so the deferral is loud at the code, not silent.

## 4. Verifiable

- `crates/argo-acp/Cargo.toml` marker references DEC-047.
- The crate still compiles and its `surface_contract.rs` pin still passes (carrying
  cost stays bounded).
- No `argo acp serve` wiring lands without a new DEC flipping this one to
  superseded.
