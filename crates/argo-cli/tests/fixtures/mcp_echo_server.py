#!/usr/bin/env python3
"""
Minimal MCP stdio server fixture for the action-level MCP-executor
integration tests. Implements the slice of the protocol the
`argo_mcp::McpClient` exercises:

  * `initialize` → returns the pinned protocol version + a static
    serverInfo block.
  * `notifications/initialized` → accepted (no response per MCP spec).
  * `tools/list` → returns three tools: `echo`, `tool_error`, `slow`.
  * `tools/call` for `echo` → returns the input as a text content block.
  * `tools/call` for `tool_error` → returns is_error=true with a text
    body so the executor surfaces the failure.
  * `tools/call` for `slow` → sleeps `ms` then echoes; used by the
    timeout test.
  * `tools/call` for an unknown tool → JSON-RPC error code -32602.

The server is a single Python file with stdlib-only deps (json, sys,
time) so every dev/CI environment can run it without extra setup.
Mirrors the Rust echo_server.rs in argo-mcp/tests/fixtures so the
two test surfaces stay drift-free.
"""
import json
import sys
import time

PROTOCOL_VERSION = "2024-11-05"


def _success(msg_id, result):
    return {"jsonrpc": "2.0", "id": msg_id, "result": result}


def _error(msg_id, code, message):
    return {"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}}


def _initialize(msg_id):
    return _success(msg_id, {
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "argo-cli-mcp-echo", "version": "0.1.0"},
    })


def _tools_list(msg_id):
    return _success(msg_id, {
        "tools": [
            {"name": "echo", "description": "echo back input",
             "inputSchema": {"type": "object",
                              "properties": {"text": {"type": "string"}}}},
            {"name": "tool_error", "description": "always returns is_error=true",
             "inputSchema": {"type": "object"}},
            {"name": "slow", "description": "sleeps then echoes",
             "inputSchema": {"type": "object",
                              "properties": {"ms": {"type": "integer"}}}},
        ],
    })


def _tools_call(msg_id, params):
    name = params.get("name", "")
    args = params.get("arguments", {}) or {}
    if name == "echo":
        text = str(args.get("text", ""))
        return _success(msg_id, {
            "content": [{"type": "text", "text": text}],
            "isError": False,
        })
    if name == "tool_error":
        return _success(msg_id, {
            "content": [{"type": "text", "text": "tool refused"}],
            "isError": True,
        })
    if name == "slow":
        ms = int(args.get("ms", 0))
        time.sleep(ms / 1000.0)
        return _success(msg_id, {
            "content": [{"type": "text", "text": f"slept {ms}"}],
            "isError": False,
        })
    return _error(msg_id, -32602, f"unknown tool: {name}")


def main():
    # `for line in sys.stdin` is block-buffered when stdin is a pipe,
    # which means the server doesn't see incoming JSON-RPC frames
    # until the kernel pipe buffer fills. Use `readline()` directly
    # so we read each newline-terminated frame as soon as it lands.
    while True:
        line = sys.stdin.readline()
        if not line:
            return
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except Exception:
            continue
        msg_id = req.get("id")
        method = req.get("method", "")
        params = req.get("params") or {}
        if msg_id is None:
            # notifications carry no id; accept silently.
            continue
        if method == "initialize":
            response = _initialize(msg_id)
        elif method == "tools/list":
            response = _tools_list(msg_id)
        elif method == "tools/call":
            response = _tools_call(msg_id, params)
        else:
            response = _error(msg_id, -32601, f"method not found: {method}")
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
