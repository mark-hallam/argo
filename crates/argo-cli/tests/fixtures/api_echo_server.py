#!/usr/bin/env python3
"""Tiny HTTP echo fixture for ApiExecutor integration tests."""

import json
from http.server import BaseHTTPRequestHandler, HTTPServer


class Handler(BaseHTTPRequestHandler):
    def _handle(self):
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length).decode("utf-8") if length else ""
        try:
            body = json.loads(raw) if raw else None
        except Exception:
            body = raw
        response = {
            "method": self.command,
            "path": self.path,
            "header": self.headers.get("X-Api-Smoke"),
            "body": body,
        }
        data = json.dumps(response).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        self._handle()

    def log_message(self, *_args):
        return


def main():
    server = HTTPServer(("127.0.0.1", 0), Handler)
    print(server.server_port, flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
