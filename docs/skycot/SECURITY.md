# Skycot Security Notes

## Gateway bind address — loopback by default

`argo gateway start` ships without TLS. To prevent an operator from
exposing the gateway publicly (e.g. by copy-pasting `--bind 0.0.0.0`
from a tutorial), the gateway **refuses to start** if `--bind` resolves
to anything outside the loopback range (`127.0.0.0/8`, `::1`).

If you intentionally run the gateway behind a TLS-terminating reverse
proxy (nginx, Caddy, k8s service mesh, Cloudflare Tunnel), opt in with
`--insecure-bind`:

```sh
argo gateway start --bind 0.0.0.0 --port 7400 --insecure-bind
```

`--insecure-bind` emits a single yellow warning to stderr on every
boot. The flag exists purely as a deliberate operator opt-in — the
gateway itself does not perform TLS termination, bearer auth alone is
not a substitute for TLS, and exposing the gateway directly to the
internet without TLS is unsupported.

Hostnames (e.g. `localhost`) are treated as non-loopback by the check —
the parser only accepts literal IP addresses. Bind to `127.0.0.1` or
`::1` explicitly for the loopback fast path.
