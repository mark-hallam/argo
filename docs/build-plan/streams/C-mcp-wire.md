# Stream C — Wire MCP into agent dispatch

**Owner**: codex. **Depends on**: A-executor-trait merged.

## Scope

The `argo-mcp` crate already has a working stdio JSON-RPC
client + server. The agent-runner's tool-dispatch loop in
`crates/argo-agent-runner/src/dispatch.rs` does NOT currently
call MCP tools. This brief wires MCP tools into the dispatch
chain so workers can call any registered MCP server's tool
mid-turn.

This is the SINGLE highest-leverage Codex PR. Once it merges,
the 5 connector PRs (gmail, slack, gcal, hubspot, notion) all
unlock.

## Files you may touch
- `crates/argo-agent-runner/src/dispatch.rs`
- `crates/argo-agent-runner/src/dispatch_tests.rs` (or sibling
  test file)
- `crates/argo-agent-runner/src/lib.rs` (re-exports)
- `crates/argo-tool/src/registry.rs` (tool registry)
- `crates/argo-mcp/src/client.rs` (any tweaks needed to make
  the client embeddable)

## Files you must NOT touch
- `crates/argo-cli/**` (executor dispatch is Stream A)
- Anything in `argo-channels`
- Any vertical's manifest

## Acceptance criteria
- [ ] `ToolRegistry` gains a `register_mcp_server(name, transport)`
      method that lazy-discovers tools via
      `McpClient::list_tools()` on first use
- [ ] The agent's `dispatch_one()` checks the registry for the
      requested tool and, if it's an MCP tool, calls
      `McpClient::call_tool()` with the model's args
- [ ] MCP tool errors are surfaced to the model as
      tool-call-error responses (not propagated as runtime
      panics)
- [ ] Per-MCP-server connection pool keyed by `name`; one
      session reused across calls within a turn
- [ ] Manifest schema extension (in `argo-vertical`): a
      `[mcp_servers]` section listing MCP servers the worker
      may use, e.g.
      `gmail = { transport = "stdio", command = "npx", args = ["@modelcontextprotocol/server-gmail"] }`
- [ ] Integration test: register a fake MCP server (in-process
      stdio with a `hello` tool), call from the dispatch loop,
      assert the result reaches the model
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-C-mcp-wire`
- PR title: `mcp: wire MCP tools into agent dispatch (C-mcp-wire)`

## Notes for the implementer

- The existing `argo-mcp::McpClient` spawns a subprocess for
  the server. Reuse it; don't fork.
- Tool name collisions (a builtin tool and an MCP tool with
  the same name): builtin wins, log a warning.
- `dispatch.rs:dispatch_one()` is where the existing 8 builtin
  tools route. Extend it; don't replace.
