# `argo-acp` crate surface audit

**Date**: 2026-05-24
**Brief**: `eval-argo-acp-crate-surface`
**Mission link**: §28.Y "Agent Client Protocol" operator-tooling substrate — VS Code / Cursor / Zed / JetBrains / Copilot / generic harnesses speak ACP into the same capability engine that backs CLI / TUI / gateway. Pre-audit the public surface so the eventual `argo acp serve` wiring lands on a contract that is already pinned.
**Verdict**: GREEN. Surface is coherent, standalone-status is honest, and the 5-module public API is now pinned by an active contract test. No new `argo-acp` substrate fix-brief is required.

## Public Surface Inventory

`crates/argo-acp/src/lib.rs` re-exports **19** names across 5 modules:

| Slice | Re-exports |
|---|---|
| Auth (`auth.rs`) | `verify`, `AuthError`, `AuthScheme`, `AuthToken` |
| Harness (`harness.rs`) | `Harness`, `HarnessCapabilities` |
| Permission (`permission.rs`) | `map_permission`, `AcpPermission` |
| Server dispatch (`server.rs`) | `AcpSendMessageRequest`, `AcpServer`, `AcpService`, `ForkRequest`, `ListSessionsResponse`, `StreamChunk` |
| JSON-RPC wire (`wire.rs`) | `AcpError`, `AcpNotification`, `AcpRequest`, `AcpResponse`, `JSONRPC_VERSION` |

The new contract test at `crates/argo-acp/tests/surface_contract.rs` pins all 19 names with a count sentinel plus function-pointer fences (`verify` signature, `map_permission` signature, `HarnessCapabilities::default_for`, `AcpResponse::ok` / `::err`, `AcpError::method_not_found`, `Harness::id`) so renames or arity changes break the test rather than silently slipping past the boundary.

`wire::ClientHandshakeParams` is intentionally **not** re-exported — it is the deserialization shape for the optional `handshake` params blob and is reachable through `AcpRequest::handshake_params()`. The audit does not propose changing that boundary.

## Standalone-Status Honesty

The crate's `Cargo.toml` carries `# zero-caller-allowed: Agent Coding Protocol SDK — consumed by external ACP clients` and the description declares "standalone ACP library; not wired to a compile-time workspace consumer in v1." A workspace grep confirms that:

- No other crate in the workspace declares `argo-acp = { path = ... }` in its `Cargo.toml`.
- No `argo_acp::` import exists outside the crate itself (the single hit in `src/lib.rs` is the rustdoc example).

This is consistent with the in-source claim. The crate is operator-tooling substrate waiting for the eventual `argo acp serve` CLI handler; pinning the surface now means that wiring lands without churn.

## Test Coverage Added

`crates/argo-acp/tests/surface_contract.rs` adds 10 contract pins:

| Contract | Pin |
|---|---|
| Re-export inventory | 19-name list + count sentinel + function-pointer fences for `verify`, `map_permission`, `AcpResponse::ok/err`, `AcpError::method_not_found`, `HarnessCapabilities::default_for`, `Harness::id`. |
| Harness capability defaults | All four §28.Y methods (`send_message`, `stream`, `list_sessions`, `fork_session`) advertised by VS Code, Cursor CLI, Generic, plus Copilot CLI / Zed / JetBrains. `HarnessCapabilities::empty()` denies every method. |
| Harness id stability | The six `Harness::id()` outputs are unique and sorted to `["copilot_cli", "cursor_cli", "generic", "jetbrains", "vscode", "zed"]`. |
| Auth success shape | `verify(CopilotOAuth \| CursorToken \| GenericBearer, ...)` returns `Principal::Acp { harness, identity }` with the documented harness string `"copilot" \| "cursor" \| "generic"` and a non-empty identity prefix. |
| Auth failure shape (no live tokens) | Empty + whitespace tokens → `AuthError::Empty`; Copilot OAuth wrong-prefix → `AuthError::Malformed { scheme: CopilotOAuth, detail contains "gho_/ghu_" }`; Cursor wrong-prefix → `AuthError::Malformed { scheme: CursorToken, .. }`. `AuthError::Rejected.to_string()` pinned for upstream pattern-matching. |
| Permission → Capability | Every `AcpPermission` variant pinned: `FileRead` → `FsRead`, `FileWrite` → `FsWrite`, `NetEgress { host }` → `NetEgress { host_glob, port: None }`, `Terminal` → `Spawn { Subprocess }`, `Subagent` → `Spawn { Subagent }`. |
| Permission → Engine | Real `argo_capability::CapabilityEngine` round-trip: an `acp_ide` role with `FsRead "src/**"` allows `AcpPermission::FileRead "src/main.rs"` and denies `FileWrite` on the same path. |
| JSON-RPC request/response shape | `session/send_message` round-trips through `AcpRequest::new` with `jsonrpc="2.0"`, `id=42`, `method="session/send_message"`, nested `params.text="ping"`. `AcpResponse::ok` omits `error`; `AcpResponse::err` omits `result` and surfaces `code=-32601` with the missing method echoed in the message. `invalid_params` / `unauthenticated` / `permission_denied` codes pinned at `-32602 / 1001 / 1003`. |
| Notification wire shape | `AcpNotification::new("session/stream_chunk", ...)` carries `jsonrpc="2.0"`, the method, and no `id` field (per JSON-RPC 2.0). `StreamChunk`, `ListSessionsResponse`, and `ForkRequest` payloads all round-trip. |
| `AcpService` object-safety | A `ContractService` `impl AcpService` proves object-safety via `&dyn AcpService`. `AcpServer::new(Harness, AuthScheme, S)` constructor signature pinned. |

All assertions are deterministic and run with no network, no live tokens, no LLM, no spawned harness — only the auth shape check (`gho_` / `cursor_` prefixes), the static permission map, and serde over the documented wire envelopes.

## Production Consumers

None. As declared in `Cargo.toml`, `argo-acp` is standalone operator-tooling substrate awaiting `argo acp serve` wiring. The audit explicitly does **not** propose wiring it now; that work belongs in a follow-up CLI-bound brief that touches `crates/argo-cli/**` paths outside this brief's may-touch list.

## Findings

No new finding in `argo-acp` itself. The crate's public surface is internally consistent, the auth / permission / wire seams are testable without network or live tokens, and the standalone-status marker matches the workspace reality.

Two pre-existing observations are recorded for context, neither requiring action under this brief:

1. `wire::ClientHandshakeParams` is reachable only via `AcpRequest::handshake_params()`. If a future ACP harness wants to build a handshake response without going through the request envelope, that struct may need re-exporting. Not a v1 blocker.
2. `AuthError::Rejected` is defined but never returned by `verify()` — the doc comment explicitly defers policy-revocation to upstream callers ("Real network-bound auth (OAuth refresh, OIDC userinfo lookup) lives in the wiring crate"). The contract test pins the Display string so downstream pattern-matching is stable.

## Follow-Up Briefs

None required.

When `argo acp serve` wiring lands, the follow-up brief should:

1. Add an integration test that exercises `AcpServer::dispatch` from inside the CLI handler with a `Principal::Acp` flowing into the real `CapabilityEngine`.
2. Re-evaluate whether `ClientHandshakeParams` needs to leave the wire module.

These are scope notes for the eventual wire-up brief, not standalone work this audit creates.

## Verification

Required commands (run from `crates/argo-acp` worktree):

```bash
cargo test -p argo-acp --tests
cargo clippy -p argo-acp --all-targets -- -D warnings
```

Both green at audit time: 21 existing tests + 10 new contract tests pass; clippy is clean with `-D warnings`.
