# Gateway TLS deployment

Argo's gateway (`argo gateway start`) ships **without native TLS**. Today the
listener is a hand-rolled HTTP/1.1 loop on plaintext TCP — fine for local dev
(`127.0.0.1:7400`) and SSO-mesh workloads where TLS terminates upstream, but a
load-bearing risk if exposed to the public internet directly.

**This runbook is the operator contract for production deployment.** It covers:

- The `[gateway.behind_proxy]` config block, refusal-to-boot guard, and
  `X-Forwarded-For` honoring rules (the in-process half of the contract).
- Five reverse-proxy front-end recipes — caddy / nginx / traefik / envoy /
  cloudflare-tunnel — sufficient to deploy a public-facing gateway with
  TLS terminated at a hardened, audited HTTP server.

> When native TLS lands as a separate stream (cert hot-reload, ACME
> integration, SNI multi-tenancy are all explicit follow-ups), the
> `[gateway.tls]` block will be the operator's signal that no upstream
> proxy is required — but until then, **the gateway is plaintext-only and
> a reverse proxy is mandatory** for any non-loopback deployment.

## The contract in one TOML block

Add to `argo.toml`:

```toml
[gateway.behind_proxy]
expected_proxy   = true
trusted_proxies  = ["10.0.0.5"]
```

- `expected_proxy = true` is the operator's promise that a TLS-terminating
  proxy fronts the gateway. The bind-safety guard refuses to boot a
  non-loopback bind without this (or the `--insecure-bind` CLI escape hatch,
  which is dev/recovery-only).
- `trusted_proxies` lists the TCP-layer peer IPs allowed to set
  `X-Forwarded-For`. **Loopback peers (`127.0.0.1`, `::1`) are always
  trusted** — same-host caddy / nginx / envoy deployments work without
  listing `127.0.0.1` explicitly. Remote-LB deployments MUST list the LB's
  egress IP(s).

## What the guard does at boot

```
argo gateway start --bind 0.0.0.0:7400
```

- **No config / no opt-in:** exits non-zero with a message pointing at this
  runbook. The pre-`mission-gateway-tls-termination-or-proxy-doc` behavior —
  silently serving plaintext to the world — is gone.
- **`[gateway.behind_proxy].expected_proxy = true`:** boots, prints a
  one-line stderr acknowledgment with the `trusted_proxies` count.
- **`--insecure-bind`:** boots with a single yellow stderr warning (intended
  for dev / recovery; do not leave on in production).
- **Loopback bind (`127.0.0.1`, `::1`, `127.0.0.5`):** boots unconditionally
  with no warning. Local dev is unaffected.

## What `X-Forwarded-For` honoring looks like

When `expected_proxy = true`:

| TCP peer                                | `X-Forwarded-For` honored? |
| --------------------------------------- | -------------------------- |
| `127.0.0.1` / `::1` (same-host proxy)   | Yes (default-allow)        |
| IP listed in `trusted_proxies`          | Yes                        |
| Anything else (LAN host, public IP)     | No — `warn` logged         |

When `expected_proxy = false` (default) the header is ignored regardless of
source — the per-IP rate limiter and audit-log attribution always see the
TCP peer.

## Recipe 1 — Caddy

Drop-in `Caddyfile`. Caddy handles ACME automatically.

```caddy
api.example.com {
    reverse_proxy 127.0.0.1:7400 {
        header_up X-Forwarded-For   {remote_host}
        header_up X-Forwarded-Proto {scheme}
    }
}
```

`argo.toml` (same-host — loopback peer auto-trusted):

```toml
[gateway.behind_proxy]
expected_proxy = true
```

Boot: `argo gateway start --bind 127.0.0.1:7400`. Caddy on `:443` fronts the
gateway, ACME issues + renews `api.example.com` on first hit.

## Recipe 2 — nginx

`/etc/nginx/sites-enabled/argo.conf`:

```nginx
server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate     /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:7400;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket upgrade (gateway emits SSE + WS feeds).
        proxy_http_version 1.1;
        proxy_set_header Upgrade    $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;
    }
}
```

`argo.toml`: same as caddy (loopback peer auto-trusted).

Pair with `certbot --nginx -d api.example.com` for ACME.

## Recipe 3 — Traefik

`traefik.yml` snippet (file provider; adapt to Kubernetes IngressRoute or
Docker labels as needed):

```yaml
http:
  routers:
    argo:
      rule: "Host(`api.example.com`)"
      entryPoints: [websecure]
      service: argo
      tls:
        certResolver: letsencrypt
  services:
    argo:
      loadBalancer:
        servers:
          - url: "http://127.0.0.1:7400"
        passHostHeader: true
```

Traefik already sets `X-Forwarded-For` from the connecting client and
appends its own hop. Same `argo.toml` as caddy / nginx.

## Recipe 4 — Envoy

Minimal envoy listener fronting the gateway over a Unix domain socket would
be more idiomatic; here's the TCP variant for parity with the others:

```yaml
static_resources:
  listeners:
    - name: argo_https
      address: { socket_address: { address: 0.0.0.0, port_value: 443 } }
      filter_chains:
        - filter_chain_match: { server_names: ["api.example.com"] }
          transport_socket:
            name: envoy.transport_sockets.tls
            typed_config:
              "@type": type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.DownstreamTlsContext
              common_tls_context:
                tls_certificates:
                  - certificate_chain: { filename: "/etc/envoy/tls/fullchain.pem" }
                    private_key:       { filename: "/etc/envoy/tls/privkey.pem"   }
          filters:
            - name: envoy.filters.network.http_connection_manager
              typed_config:
                "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
                use_remote_address: true
                xff_num_trusted_hops: 0
                route_config:
                  virtual_hosts:
                    - name: argo
                      domains: ["api.example.com"]
                      routes:
                        - match: { prefix: "/" }
                          route: { cluster: argo_gateway }
  clusters:
    - name: argo_gateway
      type: STATIC
      load_assignment:
        cluster_name: argo_gateway
        endpoints:
          - lb_endpoints:
              - endpoint:
                  address: { socket_address: { address: 127.0.0.1, port_value: 7400 } }
```

`use_remote_address: true` ensures envoy sets `X-Forwarded-For` to the
downstream peer's IP rather than its own (default behavior depends on
hop-count config).

## Recipe 5 — Cloudflare Tunnel (cloudflared)

Skip exposing port 443 entirely. The tunnel runs on the same host as the
gateway and dials out to Cloudflare's edge:

`config.yml`:

```yaml
tunnel: 00000000-0000-0000-0000-000000000000   # from `cloudflared tunnel create`
credentials-file: /etc/cloudflared/argo.json

ingress:
  - hostname: api.example.com
    service: http://127.0.0.1:7400
  - service: http_status:404
```

Cloudflare handles TLS at the edge + injects `Cf-Connecting-IP` (the real
client IP) and `X-Forwarded-For`. With `expected_proxy = true` and the
tunnel running on the same box as the gateway (peer = loopback), the
gateway will trust the LB's `X-Forwarded-For` automatically.

```bash
cloudflared service install
```

## Restart for cert rotation (HARD edge)

The gateway has **no cert hot-reload** today (a separate brief). The
upstream proxy owns the cert lifecycle; rotations happen there and the
gateway stays up.

If/when native TLS lands in the gateway itself, rotation will require
`argo gateway stop && argo gateway start` — operators should script that
into their renewal hook. This runbook will be updated then.

## Migration notes

Operators upgrading from the pre-`mission-gateway-tls-termination-or-proxy-doc`
release:

- `argo gateway start --bind 0.0.0.0` worked **with** `--insecure-bind` —
  that escape hatch still works, but the production path is now
  `[gateway.behind_proxy].expected_proxy = true` in `argo.toml`.
- `ARGO_TRUST_XFF=1` is still honored as a per-process opt-in for the
  per-IP rate limiter, but `[gateway.behind_proxy]` is the canonical
  config-driven path and takes precedence when set.

## Related

- `crates/argo-cli/src/handlers/gateway/proxy_guard.rs` — guard
  implementation + `X-Forwarded-For` extractor.
- `crates/argo-cli/src/handlers/gateway/startup_check.rs` —
  `validate_bind_safety` entry point.
- `docs/skycot/runbook/ws-token-refresh.md` — long-lived WS sessions
  through a reverse proxy.
- `docs/skycot/SECURITY.md` — broader threat model.
