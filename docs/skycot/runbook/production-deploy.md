# SKYCOT Operator Runbook — Production deploy

Reverse-proxy, process-supervisor, and container recipes for putting
the gateway in front of real traffic. For the full table of contents,
see [RUNBOOK.md](../RUNBOOK.md).

These recipes assume the topology already documented in
[Security §11.1](security.md#111-gateway-bearer-token-auth): the
gateway listens on `127.0.0.1:7400` by default, a bearer token is
configured via `ARGO_GATEWAY_BEARER_TOKEN`, and the public read-only
allowlist (`/`, `/health*`, `/version`, `/routes`) stays anonymous so
liveness probes and load balancers keep working.

---

## 12. Production deploy

### 12.1 Nginx reverse proxy

A minimal config that terminates TLS on port 443, forwards to the
gateway on `127.0.0.1:7400`, and preserves client IP via
`X-Forwarded-For`. Drop into `/etc/nginx/sites-available/argo` (or
`/etc/nginx/conf.d/argo.conf` on RHEL-likes), then symlink into
`sites-enabled` and reload.

```nginx
# /etc/nginx/sites-available/argo
server {
    listen 80;
    listen [::]:80;
    server_name argo.example.com;

    # Redirect everything to https. Certbot's webroot challenge
    # path is the only exception.
    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }
    location / {
        return 301 https://$host$request_uri;
    }
}

server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name argo.example.com;

    ssl_certificate     /etc/letsencrypt/live/argo.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/argo.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # Tune for SSE / long-lived audit streams.
    proxy_read_timeout 3600s;
    proxy_send_timeout 3600s;
    proxy_buffering off;

    location / {
        proxy_pass http://127.0.0.1:7400;
        proxy_http_version 1.1;

        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-Host  $host;

        # Server-Sent Events on /audit/stream need these.
        proxy_set_header Connection        '';
        chunked_transfer_encoding off;
    }
}
```

Get the cert and reload nginx:

```bash
sudo apt-get install -y certbot python3-certbot-nginx
sudo certbot --nginx -d argo.example.com --redirect --agree-tos -m ops@example.com
sudo nginx -t && sudo systemctl reload nginx
```

Health check from outside:

```bash
curl -sS https://argo.example.com/health/ready
# {"ready":true}
```

`/audit/stream` (SSE) is the route most likely to surface
proxy-buffer issues — `proxy_buffering off` and the long
`proxy_read_timeout` above are required, not optional.

### 12.2 Caddy reverse proxy

Caddy auto-provisions and renews TLS via Let's Encrypt; the
Caddyfile equivalent of §12.1 is half the bytes. Drop into
`/etc/caddy/Caddyfile` and reload.

```caddy
# /etc/caddy/Caddyfile
argo.example.com {
    encode gzip

    # Long-lived SSE on /audit/stream — disable response buffering
    # and let the upstream drive the connection lifetime.
    reverse_proxy 127.0.0.1:7400 {
        flush_interval -1
        transport http {
            read_timeout  1h
            write_timeout 1h
        }
        header_up X-Real-IP         {remote_host}
        header_up X-Forwarded-For   {remote_host}
        header_up X-Forwarded-Proto {scheme}
        header_up Host              {host}
    }
}
```

Reload after editing:

```bash
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

Caddy resolves the cert on first request to `argo.example.com`, so
make sure the A/AAAA record is in place before reloading or the
ACME challenge will fail. Logs land in `journalctl -u caddy`.

### 12.3 systemd unit

Run the gateway under systemd with a dedicated `argo` user, an
environment file for secrets, automatic restart on failure, and
journal-based logging. Drop into
`/etc/systemd/system/argo-gateway.service`.

```ini
# /etc/systemd/system/argo-gateway.service
[Unit]
Description=SKYCOT (argo) gateway
Documentation=https://github.com/mark-hallam/Argo-Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=argo
Group=argo
WorkingDirectory=/var/lib/argo
EnvironmentFile=/etc/argo.env
ExecStart=/usr/local/bin/argo gateway start
Restart=on-failure
RestartSec=5s

# Tighten the blast radius. Loosen any of these only if a feature
# you actually use stops working — they're conservative defaults.
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/var/lib/argo
ProtectHome=true
PrivateTmp=true
PrivateDevices=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
LockPersonality=true

# Journal captures stdout/stderr; query with `journalctl -u argo-gateway`.
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

The `EnvironmentFile` keeps secrets off the command line. Format
is one `KEY=value` per line (no `export`, no quotes around the
value unless you want them included literally):

```bash
# /etc/argo.env  (chmod 600, chown argo:argo)
ARGO_GATEWAY_BEARER_TOKEN=replace-with-openssl-rand-hex-32
ANTHROPIC_API_KEY=sk-ant-...
# Optional: set XDG_DATA_HOME if you want ~/.argo elsewhere
# XDG_DATA_HOME=/var/lib/argo
```

Bring it up:

```bash
sudo useradd --system --home /var/lib/argo --shell /usr/sbin/nologin argo
sudo install -d -o argo -g argo -m 750 /var/lib/argo
sudo install -m 600 -o argo -g argo /dev/stdin /etc/argo.env <<< 'ARGO_GATEWAY_BEARER_TOKEN=...'
sudo systemctl daemon-reload
sudo systemctl enable --now argo-gateway
sudo systemctl status argo-gateway
journalctl -u argo-gateway -f
```

Log rotation is handled by journald; cap retention via
`SystemMaxUse=` in `/etc/systemd/journald.conf` if disk pressure
becomes a concern.

### 12.4 Docker Compose

For operators who'd rather ship a container than maintain a host
package. The recipe below pins a single-service stack with a
bind-mounted SQLite directory, env file, healthcheck against
`/health/ready`, and a port mapping that keeps the gateway
loopback-only (the reverse proxy on the host or another container
handles TLS).

```yaml
# compose.yml
services:
  argo:
    image: ghcr.io/mark-hallam/argo:latest  # or pin a tag in production
    container_name: argo-gateway
    restart: unless-stopped
    env_file:
      - ./argo.env
    # Bind 7400 to loopback only — terminate TLS upstream (nginx /
    # Caddy / a sibling proxy container).
    ports:
      - "127.0.0.1:7400:7400"
    volumes:
      - ./data:/var/lib/argo:rw
    working_dir: /var/lib/argo
    command: ["argo", "gateway", "start"]
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:7400/health/ready"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    # Match the systemd hardening where the runtime allows.
    read_only: true
    tmpfs:
      - /tmp
    security_opt:
      - no-new-privileges:true
```

`./argo.env` mirrors §12.3:

```bash
# argo.env  (chmod 600 next to compose.yml)
ARGO_GATEWAY_BEARER_TOKEN=replace-with-openssl-rand-hex-32
ANTHROPIC_API_KEY=sk-ant-...
```

Bring it up:

```bash
mkdir -p data
chmod 600 argo.env
docker compose up -d
docker compose ps
docker compose logs -f argo
```

Verify the healthcheck flipped to healthy:

```bash
docker inspect --format '{{.State.Health.Status}}' argo-gateway
# healthy
```

A gateway upgrade is `docker compose pull && docker compose up -d`;
the SQLite state under `./data` survives across container
replacements because it's a host bind mount, not a named volume
that could be pruned with `docker volume prune`.

---

Next: [Where to next + reference](reference.md).
