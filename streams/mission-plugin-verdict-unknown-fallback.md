# mission-plugin-verdict-unknown-fallback

**Priority**: P2 (forward-compat hardening — fix-brief from #2118 FINDINGS-ONLY pin)
**Owner**: claude (fast-mode candidate)
**Type**: Wire-protocol forward-compat — `#[serde(other)]` fallback variant on `PreToolCallVerdict`
**Size**: ~50-100 LOC (1 source file + new test cases + un-`#[ignore]` the existing FINDINGS pin)
**Authored against**: origin/main `576e48f5c` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-plugin-verdict-unknown-fallback`

## Why

PR #2118 (`eval-plugin-abi-stable`) shipped a FINDINGS-ONLY ignored
test at `crates/argo-plugin/tests/abi_contract.rs:391` pinning a
forward-compat gap in the plugin wire protocol:

> **FINDING:** `PreToolCallVerdict` is exhaustive on deserialize.
> If a newer plugin SDK adds a verdict variant (e.g.
> `{"action": "defer"}`), the current host will fail to decode
> with a serde error, breaking the call rather than gracefully
> rejecting the verdict.
>
> Forward-compat policy says additive variants on a wire enum
> SHOULD decode to an "unknown — treat as deny" fallback so a
> newer plugin against an older host degrades safely instead of
> crashing the call.

This is a v1 invariant — **Polyglot core (Rust core + JSON-RPC
plugin ABI; plugins are subprocesses)** per `CLAUDE.md`. A
plugin SDK upgraded ahead of the host should never hard-crash the
host; the host should refuse the call and continue.

Today (verified live on `origin/main` `576e48f5c`):

```rust
// crates/argo-plugin/src/protocol.rs:100
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PreToolCallVerdict {
    Allow,
    Deny { reason: String },
    Transform { args: serde_json::Value },
}
```

`#[serde(tag = "action")]` plus a closed variant list = exhaustive
deserialize. The fallback variant required by serde's `other`
attribute doesn't exist.

## What

Add an `Unknown` fallback variant to `PreToolCallVerdict` that
captures the raw verdict payload **and** documents the
"unknown → treat as deny" host policy in the type's rustdoc. The
host's verdict-handling code (the `match` over verdict in the
pre-tool-call dispatcher) must add the `Unknown` arm and route
it through the same path as `Deny { reason: "unknown verdict
variant: <action>" }`.

### Source-side change (`crates/argo-plugin/src/protocol.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PreToolCallVerdict {
    Allow,
    Deny { reason: String },
    Transform { args: serde_json::Value },
    /// Forward-compat fallback. Any `action` value that doesn't
    /// match a known variant deserialises here. The host treats
    /// this as a structured `Deny { reason: "unknown verdict
    /// variant: <action>" }` — a newer plugin SDK against an
    /// older host degrades safely instead of hard-crashing the
    /// call.
    #[serde(other)]
    Unknown,
}
```

**Caveat:** `#[serde(other)]` requires the variant to be unit-style
(no payload), so `Unknown` cannot carry the original raw JSON. The
host-side error message must therefore say "unknown verdict
variant" without naming the variant — that's acceptable for v1
(the host's audit log still carries the raw incoming JSON-RPC
payload one level up).

If preserving the unknown action name is load-bearing, the
alternative is a manual `Deserialize` impl that catches the
`UnknownVariant` error and produces `Unknown { action: String }` —
~30 extra LOC. Recommend starting with `#[serde(other)]` and
upgrading only if a test surfaces the need.

### Host-side change

Grep for every `match` against `PreToolCallVerdict` in the
workspace (`rg 'PreToolCallVerdict::' crates/`) and add the
`Unknown` arm. Expected sites: the plugin dispatcher in
`argo-plugin` and the pre-tool-call wrapper in the runtime tier.
Each arm routes to the same outcome as `Deny`.

### Test changes

1. **Un-ignore** the existing FINDINGS-ONLY pin at
   `crates/argo-plugin/tests/abi_contract.rs:392` (delete the
   `#[ignore = "FINDINGS-ONLY: …"]` attribute). Update the assert
   to expect `PreToolCallVerdict::Unknown` (currently the test
   only asserts `result.is_ok()`).
2. Add two new test cases in the same file:
   - `unknown_verdict_round_trips_through_host_match` — assert
     the host's verdict-to-action mapping treats `Unknown` like
     `Deny`.
   - `transform_with_extra_unknown_field_still_decodes_as_transform`
     — pin that unknown FIELDS on known variants (vs unknown
     VARIANTS) still tolerate via `#[serde(deny_unknown_fields)]`
     NOT being set (already implicit, but pin it as a contract).

## Acceptance

- `cargo test -p argo-plugin --test abi_contract` → all tests
  pass including the previously-`#[ignore]`'d
  `finding_unknown_verdict_variant_should_decode_as_deny`
  (now asserts `Unknown`, not just `Ok`).
- `cargo clippy -p argo-plugin --all-targets -- -D warnings` clean.
- `rg 'PreToolCallVerdict::' crates/ -l` enumerates every match
  site; PR body lists each one and confirms the `Unknown` arm.
- Workspace clippy fast-fail unchanged.
- PR body includes a 5-line "behaviour change" note: newer plugin
  SDKs against older hosts no longer hard-crash the call.

## May-touch

- `crates/argo-plugin/src/protocol.rs` (add `Unknown` variant).
- `crates/argo-plugin/tests/abi_contract.rs` (un-ignore + new
  cases).
- Every workspace site `match`ing `PreToolCallVerdict` (expected
  ≤3 files; enumerate at claim time via `rg`).

## Source

- PR #2118 FINDINGS-ONLY pin at `abi_contract.rs:391`.
- `eval-plugin-abi-stable` ledger row at `BUILD_STATUS.md:159`.
- v1 invariant #1 (Polyglot core) per `CLAUDE.md`.
- Mission claim 1/2/3 (worker safety against newer plugins).

Fast-mode candidate: pure mechanical extension of an existing
enum + 3 known test additions. Should ship in one round.
