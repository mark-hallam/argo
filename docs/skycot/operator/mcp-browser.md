# Operator Notes - Browser MCP

`argo-mcp-browser` is a local-CDP v1 server. It attaches to Chrome via
`--remote-debugging-port=9222` and drives the shipped browser tools
against that local endpoint.

## Backend Status

| Backend | v1 status | Operator action |
|---|---|---|
| Local CDP | Supported | Run Chrome with remote debugging enabled. |
| Browserbase | DEFERRED-v1.x | Do not advertise as production-ready in v1. |
| BrowserUse | DEFERRED-v1.x | Do not advertise as production-ready in v1. |

If a `server.toml` selects `browserbase` or `browseruse`, the server
logs a WARN and falls back to local CDP. Use
`ARGO_MCP_BROWSER_SERVER_TOML=/path/to/server.toml` when the config is
not in the launch directory.

## `wait_for network_idle`

`wait_for` now supports:

```json
{
  "mode": "network_idle",
  "quiet_period_ms": 500,
  "timeout_ms": 30000
}
```

The server tracks CDP network events and resolves once there are zero
in-flight requests for the quiet period. `timeout_ms` remains the hard
cap.
