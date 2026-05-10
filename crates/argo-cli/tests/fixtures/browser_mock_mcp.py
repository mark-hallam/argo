#!/usr/bin/env python3
"""
Minimal MCP stdio fixture for the action-level Browser-executor tests.

Implements the slice of the protocol that `argo_mcp::McpClient`
exercises plus the browser-specific tools the action-level executor
calls (`navigate`, `click`, `fill`, `submit`, `screenshot`, `cu_loop`).

`navigate` honours a `?slow_ms=N` query string by sleeping N
milliseconds before responding — the timeout test uses this to drive
a wallclock deadline blow-up without forcing the whole pool.

`screenshot` returns an MCP `image` content block with a tiny
deterministic base64 payload so the executor can write it into the
action's evidence bundle and assertions can verify it lands on disk.

stdlib-only deps (json, sys, time, urllib.parse) — runs on any
dev/CI machine with `python3` in PATH. Mirrors the
`mcp_echo_server.py` shape so the two fixtures stay drift-free.
"""

import json
import sys
import time
from urllib.parse import urlparse, parse_qs

PROTOCOL_VERSION = "2024-11-05"

# Tiny 1x1 transparent PNG, base64-encoded — keeps the screenshot
# content under the line-buffer ceiling without dragging in a real
# PNG dep.
TINY_PNG_B64 = (
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
)


def _success(msg_id, result):
    return {"jsonrpc": "2.0", "id": msg_id, "result": result}


def _error(msg_id, code, message):
    return {"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}}


def _initialize(msg_id):
    return _success(msg_id, {
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "argo-cli-browser-mock", "version": "0.1.0"},
    })


def _tools_list(msg_id):
    return _success(msg_id, {
        "tools": [
            {"name": "navigate", "description": "Open a URL.",
             "inputSchema": {"type": "object",
                              "properties": {"url": {"type": "string"}},
                              "required": ["url"]}},
            {"name": "click", "description": "Click selector.",
             "inputSchema": {"type": "object",
                              "properties": {"selector": {"type": "string"}},
                              "required": ["selector"]}},
            {"name": "fill", "description": "Fill selector with value.",
             "inputSchema": {"type": "object",
                              "properties": {"selector": {"type": "string"},
                                              "value": {"type": "string"}},
                              "required": ["selector", "value"]}},
            {"name": "submit", "description": "Submit form.",
             "inputSchema": {"type": "object",
                              "properties": {"selector": {"type": "string"}},
                              "required": ["selector"]}},
            {"name": "screenshot", "description": "Capture page.",
             "inputSchema": {"type": "object"}},
            {"name": "cu_loop", "description": "Run a CU loop.",
             "inputSchema": {"type": "object",
                              "properties": {"goal": {"type": "string"},
                                              "target_url": {"type": "string"}}}},
        ],
    })


def _maybe_sleep(url):
    """Honour `?slow_ms=N` so the timeout test can blow past its deadline."""
    if not url:
        return
    parsed = urlparse(url)
    qs = parse_qs(parsed.query or "")
    if "slow_ms" in qs:
        try:
            ms = int(qs["slow_ms"][0])
            time.sleep(ms / 1000.0)
        except (ValueError, TypeError):
            pass


def _tools_call(msg_id, params):
    name = params.get("name", "")
    args = params.get("arguments", {}) or {}
    if name == "navigate":
        _maybe_sleep(args.get("url", ""))
        return _success(msg_id, {
            "content": [{"type": "text", "text": f"navigated to {args.get('url', '')}"}],
            "isError": False,
        })
    if name in ("click", "fill", "submit"):
        return _success(msg_id, {
            "content": [{"type": "text", "text": f"{name} ok"}],
            "isError": False,
        })
    if name == "screenshot":
        return _success(msg_id, {
            "content": [{
                "type": "image",
                "data": TINY_PNG_B64,
                "mimeType": "image/png",
            }],
            "isError": False,
        })
    if name == "cu_loop":
        return _success(msg_id, {
            "content": [{"type": "text", "text": "cu_loop completed"}],
            "isError": False,
        })
    return _error(msg_id, -32602, f"unknown tool: {name}")


def main():
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
