# argo-plugin ABI stability audit

Date: 2026-05-25
Brief: `eval-argo-plugin-abi-stable`
Scope: read-only audit of the `argo-plugin` JSON-RPC ABI surface.

## Executive summary

`argo-plugin` exposes a small polyglot ABI: line-delimited JSON-RPC 2.0
requests and responses over plugin subprocess stdin/stdout. The public
surface is already better pinned than most crates because
`tests/abi_contract.rs` locks constants, golden payload shapes, additive
unknown-field tolerance, optional defaults, ID polymorphism, verdict
variants, and version-classifier behavior.

The audit adds `tests/abi_stability_pin_test.rs` as a second, explicit
stability pin for host-sendable method envelopes and minimal decoders.
This catches the high-risk drift case from the brief: a public ABI
struct gaining a new non-optional field without reviewers seeing a
wire-contract failure.

Main finding: the versioning story is split. `PROTOCOL_VERSION` is
exchanged only during `handshake`, and `host::handshake` rejects any
non-exact version string. Semver compatibility helpers exist and would
accept same-major newer-minor plugin versions, but they are not wired
into the host path yet.

## Surface inventory

| Surface | Public export | Wire impact | Source |
|---|---|---|---|
| Crate docs | subprocesses speak line-delimited JSON-RPC 2.0; `handshake`; later methods; `shutdown` | Defines the cross-language plugin contract | `crates/argo-plugin/src/lib.rs:1` |
| `abi` module | `JSONRPC_VERSION`, JSON-RPC error codes, version parser/classifier, `is_wire_compatible` | Pins envelope version and intended compatibility rules | `crates/argo-plugin/src/lib.rs:38`, `crates/argo-plugin/src/abi.rs:40` |
| `protocol` module | `PROTOCOL_VERSION`, request/response/error structs, handshake structs, `PreToolCallVerdict` | Public serializable ABI types | `crates/argo-plugin/src/lib.rs:45`, `crates/argo-plugin/src/protocol.rs:16` |
| `PluginHost` | `PluginHost`, `DEFAULT_CALL_TIMEOUT` | Host implementation that emits requests and decodes responses | `crates/argo-plugin/src/lib.rs:44`, `crates/argo-plugin/src/host.rs:63` |
| Compatibility aliases | `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError` | Public aliases for protocol structs; same wire contract | `crates/argo-plugin/src/lib.rs:49` |
| Registry surface | `PluginManifestEntry`, `PluginRegistry`, `PluginRegistryConfig` | Plugin discovery/configuration; outside JSON-RPC wire shape | `crates/argo-plugin/src/lib.rs:53` |

## JSON-RPC envelope

Every host request serializes as:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "method_name",
  "params": {}
}
```

`PluginJsonRpcRequest::new` hardcodes `"2.0"`, numeric IDs, method
string, and params value. `params` defaults to JSON null if omitted on
decode. The public request type allows JSON-RPC IDs of any JSON value on
decode, but host response delivery currently ignores non-numeric response
IDs because `deliver` requires `resp.id.as_u64()`.

Responses serialize as either:

```json
{"jsonrpc":"2.0","id":1,"result":{}}
```

or:

```json
{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"method not found"}}
```

`result` and `error` are optional fields on the struct and omitted when
`None`. `PluginHost::call_with_deadline` requires exactly one semantic
success path: `(Some(result), None)` succeeds, `(None, Some(error))`
returns RPC error, and every other combination returns a decode error.

## Host method catalogue

| Method | Direction | Host entry point | Params schema | Response schema | Notes |
|---|---|---|---|---|---|
| `handshake` | Argo -> plugin request | private startup path in `PluginHost::start_with_timeout` | `{"protocol_version": string}` | result decodes as `HandshakeResponse` | Required before host construction succeeds. Host rejects non-exact `protocol_version`. |
| arbitrary method string | Argo -> plugin request | `PluginHost::call(method, params)` | method-specific JSON value | method-specific JSON value or JSON-RPC error | The host has no registry or enum of legal methods. In-tree tests exercise `ping`, `pre_tool_call`, `slow`, `crash`, `bad_version_handshake`, and unknown-method error behavior through the echo fixture. |
| `pre_tool_call` | Argo -> plugin request by convention | `PluginHost::call("pre_tool_call", params)` | current test convention: `{"tool": string, "args": object}` | `PreToolCallVerdict` result: `allow`, `deny`, or `transform` | The protocol exports the verdict type but not a typed request struct for params. |
| `shutdown` | Argo -> plugin request-shaped shutdown | `PluginHost::shutdown()` | `{}` | best-effort; response is not awaited | Docs call this a notification, but implementation emits a normal request envelope with `id: 0`. |

Notifications: no host-sent notification type is implemented today.
`host.rs` keeps an unused `NotificationTx` alias for future plugin ->
Argo notification work, but no public notification ABI is active.

## Versioning contract

The active versioned ABI is handshake-scoped:

- `PROTOCOL_VERSION` is pinned at `"0.1.0"`.
- Host handshake sends `{"protocol_version":"0.1.0"}`.
- Plugin handshake response must include `protocol_version` and `name`.
- `version` and `capabilities` on `HandshakeResponse` default when
  absent, so older plugins can omit them.
- Host startup rejects any `HandshakeResponse.protocol_version` that is
  not string-equal to `PROTOCOL_VERSION`.

The compatibility helpers document the intended future policy:

- exact match -> `AbiCompatibility::Identical`;
- same-major plugin newer minor/patch -> `PluginNewerMinor` and
  `is_wire_compatible == true`;
- newer major, older plugin, or unparseable versions -> not compatible.

That helper policy is not the runtime policy yet. The runtime policy is
strict exact match in `host::handshake`.

Forward-compatibility on payloads is mostly additive:

- serde defaults allow `params`, response `result`/`error`,
  `PluginJsonRpcError.data`, `HandshakeResponse.version`, and
  `HandshakeResponse.capabilities` to be omitted where documented.
- unknown fields are accepted by default on public wire structs.
- enum expansion is not currently tolerant: unknown
  `PreToolCallVerdict.action` variants fail serde decode. Existing
  `abi_contract.rs` already records this as an ignored finding.

## Breaking-change detection

Current detection is test-based, not schema-lint-based.

Existing coverage:

- `crates/argo-plugin/tests/abi_contract.rs` pins constants, golden
  shapes, forward-compatible unknown fields, optional defaults, ID
  polymorphism, verdict variants, and version classifiers.
- `crates/argo-plugin/tests/host.rs` exercises the live subprocess path
  through the echo plugin.

Added coverage:

- `crates/argo-plugin/tests/abi_stability_pin_test.rs` pins the host
  request envelopes for `handshake`, `pre_tool_call`, generic `ping`,
  and `shutdown`.
- It also decodes minimal valid payloads and asserts currently required
  fields remain required.

If a public ABI struct gains a non-optional field:

- struct-literal tests may fail to compile, forcing a deliberate update;
- golden `to_value` tests fail if serialization adds a new field;
- minimal `from_value` tests fail if older wire payloads no longer
  decode;
- host integration tests fail if handshake or response decode becomes
  stricter.

There is no dedicated CI lint that compares ABI schema changes against
a required `PROTOCOL_VERSION` bump. A reviewer can still update tests
and forget to bump the protocol version unless a future schema/golden
lint makes that relation explicit.

## Follow-up briefs

1. `mission-plugin-verdict-unknown-fallback`: make unknown
   `PreToolCallVerdict.action` variants degrade to a safe deny/unknown
   result instead of a hard serde error.
2. `ops-plugin-abi-semver-negotiation-wireup`: choose and wire the
   runtime policy: keep exact-string match and remove ambiguity, or
   switch `host::handshake` to `is_wire_compatible`.
3. `ops-plugin-abi-schema-golden-ci`: generate or snapshot JSON schemas
   for public ABI structs and require an explicit protocol-version
   review when schema changes.
4. `docs-plugin-shutdown-notification-contract`: reconcile docs that
   describe `shutdown` as a notification with implementation that sends
   a request envelope with `id: 0`.
5. `mission-plugin-method-registry`: define a typed catalogue for
   mission-critical plugin methods, starting with `pre_tool_call`
   params, while preserving `PluginHost::call` as a lower-level escape
   hatch.

## Acceptance status

- Surface table + method catalogue: complete.
- Versioning contract with source citations: complete.
- Pin tests: added in `abi_stability_pin_test.rs`.
- Follow-up briefs: five proposed.
