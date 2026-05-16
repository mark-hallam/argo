#!/usr/bin/env python3
"""Slack API stub for scripts/demo-dry-run.sh.

Listens on a single port, records every POST (method + path + body) to
a log file, and replies with `{"ok": true}` so the SKYCOT Slack
connector accepts the response shape. `/healthz` returns 200 for the
launch readiness probe.

Usage:  python3 slack_stub.py <port> <log-path>
"""

from __future__ import annotations
import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer


def make_handler(log_path: str):
    class Handler(BaseHTTPRequestHandler):
        def _ok(self, body: bytes) -> None:
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self) -> None:  # noqa: N802 (http.server convention)
            if self.path == "/healthz":
                self._ok(b'{"ok":true}')
                return
            self.send_response(404)
            self.end_headers()

        def do_POST(self) -> None:  # noqa: N802
            length = int(self.headers.get("content-length") or 0)
            payload = self.rfile.read(length) if length else b""
            entry = {
                "method": "POST",
                "path": self.path,
                "body": payload.decode("utf-8", errors="replace")[:4000],
            }
            with open(log_path, "a", encoding="utf-8") as fh:
                fh.write(json.dumps(entry) + "\n")
            self._ok(b'{"ok":true,"ts":"0000000000.000000"}')

        def log_message(self, *_args, **_kwargs) -> None:
            return

    return Handler


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: slack_stub.py <port> <log-path>", file=sys.stderr)
        return 2
    port = int(sys.argv[1])
    log_path = sys.argv[2]
    open(log_path, "w").close()
    httpd = HTTPServer(("127.0.0.1", port), make_handler(log_path))
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
