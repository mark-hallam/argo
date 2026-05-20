# Browser MCP Server

`argo-mcp-browser` exposes browser automation tools over MCP using the
local Chrome DevTools Protocol (CDP) endpoint.

## Backends

| Backend | v1 status | Notes |
|---|---|---|
| Local CDP | Supported | Start Chrome with `--remote-debugging-port=9222`; tools attach to the first page tab. |
| Browserbase | DEFERRED-v1.x | `server.toml` selection logs a WARN and falls back to local CDP. |
| BrowserUse | DEFERRED-v1.x | `server.toml` selection logs a WARN and falls back to local CDP. |

If `server.toml` contains `backend = "browserbase"` or
`backend = "browseruse"`, the v1 server does not crash or silently
switch behavior. It emits:

```text
WARN argo-mcp-browser: Cloud backend <backend> selected but not wired in v1 - falling back to local CDP. Track v1.x for cloud support.
```

Set `ARGO_MCP_BROWSER_SERVER_TOML=/path/to/server.toml` when the
server is launched from a directory other than the config directory.

## `wait_for`

`wait_for` supports two modes:

```json
{ "mode": "selector", "selector": "#ready", "timeout_ms": 30000 }
```

```json
{ "mode": "network_idle", "quiet_period_ms": 500, "timeout_ms": 30000 }
```

`network_idle` watches CDP `Network.*` events and resolves after the
quiet period has elapsed with zero in-flight requests. The default
quiet period is 500 ms.
