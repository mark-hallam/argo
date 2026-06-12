# fix-plugin-verdict-unknown-fallback

**Owner**: claude or codex — plugin ABI forward-compat; conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `de0e01ea3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-plugin-verdict-unknown-fallback

## Problem (discovery-verified 2026-06-12)

`PreToolCallVerdict` (`crates/argo-plugin/src/protocol.rs:~100-115`) is an
internally-tagged serde enum (`#[serde(tag = "action")]`) with exactly
three variants (Allow / Deny / Transform) and **no unknown-variant
fallback**. A newer plugin SDK that sends a 4th `action` value fails
deserialization — and on the security-critical pre-tool-call gate, a
deserialize error must fail CLOSED (treat as Deny), never crash the host or
silently allow. This is a polyglot-plugin-ABI forward-compat invariant
(core invariant #1: JSON-RPC plugin ABI). A `#[ignore]`'d test pins it
(grep `#[ignore]` in argo-plugin to confirm the test name at claim time).

## Fix

Add an `Unknown` (or `#[serde(other)]`) fallback variant to
`PreToolCallVerdict` so an unrecognized `action` deserializes to a known
"unknown" state, and map that state to **Deny** (fail-closed) at the
enforcement site. Update the 3-5 match arms that exhaustively match the
enum. Un-ignore the pinned test. Document the fail-closed contract in the
rustdoc.

## May touch

- `crates/argo-plugin/src/protocol.rs` (+ its tests)
- the enforcement site(s) that match `PreToolCallVerdict` (map Unknown→Deny)

## Acceptance / verifier

1. The pinned test un-ignored + GREEN: an `action: "<future-variant>"`
   payload deserializes to Unknown and is enforced as Deny (fail-closed),
   not a panic or Allow.
2. Existing Allow/Deny/Transform behavior unchanged; `cargo test -p
   argo-plugin` green; clippy clean.

~50-100 LOC. **🟡 P2 — plugin-ABI forward-compat, fail-closed on the
pre-tool-call gate (security-adjacent).**
