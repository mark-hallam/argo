#!/usr/bin/env bash
# check-spendgate-coverage.sh — prove the hard `[budget.ceiling]` spend gate
# (`launch-wire-spendgate-llm-cost-ceiling`, WS5 open-wallet close) is a SINGLE
# choke point that no LLM path can forget.
#
# Coverage is primarily enforced by the TYPE SYSTEM: `build_transport` and
# `build_transport_for_tenant` (crates/argo-agent-runner/src/transport.rs)
# both REQUIRE a `TransportGate` argument, so a bare/un-gated call does not
# compile. This script is the belt-and-braces grep the brief asks for: it
# fails if any `build_transport(` / `build_transport_for_tenant(` call site is
# bare (single `manifest`-only argument), which would mean someone added a new
# raw builder overload that skips the gate.
#
# Exit 0 = every call site passes a gate. Exit 1 = a bare call was found.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# Match a call `build_transport(<ident>)` or `build_transport(<ident>).await`
# with NOTHING after the manifest arg (no comma) — i.e. the pre-gate shape.
# The two `fn build_transport(` / `fn build_transport_for_tenant(` definitions
# and the private `_inner` builders are intentionally excluded.
bare=$(grep -rnE 'build_transport(_for_tenant)?\([a-z_]+\)(\.await)?' crates/ \
  | grep -vE 'fn build_transport|_inner' || true)

if [ -n "$bare" ]; then
  echo "FAIL: un-gated build_transport call site(s) found — every call MUST pass a TransportGate:" >&2
  echo "$bare" >&2
  exit 1
fi

echo "OK: all build_transport(_for_tenant) call sites pass a TransportGate (spend gate is the single choke point)."
