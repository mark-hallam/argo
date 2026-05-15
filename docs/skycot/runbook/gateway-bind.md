# Gateway Bind Runbook

Argo gateway defaults to `127.0.0.1:7400` so the HTTP API is reachable
only from the operator host. That default matters because local
development can run with `[gateway].auth_required=false`; binding the
same unauthenticated surface to `0.0.0.0` exposes it to every host that
can reach the machine.

## Default Model

Run the gateway on loopback:

```bash
argo gateway start --bind 127.0.0.1 --port 7400
```

The Argo CLI and local probes connect over loopback. Remote hosts cannot
reach the gateway unless the operator deliberately places another
network component in front of it.

## External Exposure Modes

### Reverse Proxy Recommended

Keep the gateway on loopback and put a TLS-terminating proxy in front:

```bash
argo gateway start --bind 127.0.0.1 --port 7400
```

Use nginx, caddy, traefik, envoy, or a Cloudflare tunnel to terminate
TLS and enforce the site auth policy. Set
`[gateway.behind_proxy].expected_proxy=true` only after the proxy is in
place and `trusted_proxies` lists the load-balancer egress IPs. See
`docs/skycot/runbook/gateway-tls-deployment.md` for the proxy contract.

### In-Process TLS Alternative

Native gateway TLS is not wired in this build. If a future deployment
adds in-process TLS, use that path instead of binding plaintext HTTP to
a non-loopback address. Until then, use the reverse proxy model above.

### Plaintext Non-Loopback Development Only

For local lab or recovery work where plaintext non-loopback access is
deliberate:

```bash
argo gateway start --bind 0.0.0.0 --port 7400 \
  --bind-non-loopback --accept-insecure-bind-warning
```

This prints a warning and writes a `gateway_insecure_bind` audit-chain
row before `TcpListener::bind`, so even a failed bind attempt leaves a
forensic record. Do not use this mode for production exposure.

The deprecated single flag is intentionally rejected:

```bash
argo gateway start --bind 0.0.0.0 --insecure-bind
```

Use the two-flag form so operator intent is explicit.

## Verify The Listener

On Linux:

```bash
ss -tlnp | grep 7400
```

Expected loopback output includes `127.0.0.1:7400`, not
`0.0.0.0:7400`. IPv6 loopback is `::1`; broader IPv6 bind hardening is
tracked separately.

## Audit Row

Alert on any audit row with:

```text
kind = gateway_insecure_bind
```

The payload includes:

```json
{
  "event_type": "gateway_insecure_bind",
  "severity": "warn",
  "actor": "system",
  "bind": "0.0.0.0",
  "port": 7400,
  "pid": 12345,
  "tls_enabled": false,
  "bind_source": "cli"
}
```

Treat this as a development/recovery exception. If it appears on a
production host, confirm that a TLS and auth boundary exists or move the
gateway back to `127.0.0.1:7400`.
