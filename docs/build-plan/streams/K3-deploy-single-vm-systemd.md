# K3 — Single-VM systemd recipe (Ubuntu 24.04 + nginx + certbot)

**Owner**: claude. **Depends on**: existing `cargo build --release` produces a working `argo` binary; existing `web/` builds via `npm run build`. NO dependency on K1 / K2.
**Authored against**: origin/main `08c4ff04` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh K3-deploy-single-vm-systemd`.

> SOURCE: DEPLOYMENT-PATH DIRECTIVE v3 §3 (Tier 5a). **Tier K (deployment recipe).** Third of three packaging recipes. Targets a single Ubuntu 24.04 LTS VM (DigitalOcean droplet / Hetzner / EC2 t3.medium baseline). NOT containerised. systemd-managed; nginx reverse-proxy; Let's Encrypt TLS via certbot. The "I just want a VM I can SSH into" deployment.

## Scope

NEW `deploy/vm/` directory (≤ 650 LOC across 6 files):

### Files

- `deploy/vm/install.sh` (NEW, ≤ 280 LOC) — idempotent bootstrap:
  - Detect Ubuntu 24.04 LTS (refuse to run on other distros / versions with clear error)
  - apt update + install: `build-essential pkg-config libssl-dev libsqlite3-dev curl git nginx postgresql-16 postgresql-contrib certbot python3-certbot-nginx nodejs npm`
  - Create system user `argo` (non-login, home `/var/lib/argo`)
  - Initialize postgres role + db: `argo` / `argo` (password generated + stored at `/var/lib/argo/.pgpass` mode 0400 owned by argo)
  - Fetch + build argo binary from source (default; flag `--use-release` for pre-built artifact when one is published)
  - Install `argo` to `/usr/local/bin/argo`
  - Copy systemd units to `/etc/systemd/system/`
  - Copy `nginx-skycot.conf` to `/etc/nginx/sites-available/skycot` + symlink in `sites-enabled`
  - `systemctl enable --now argo web nginx postgresql`
  - Run certbot interactive ONLY if `--with-tls <domain>` flag passed; otherwise leave HTTP-only with banner
  - Idempotency: re-run skips already-completed steps via marker files at `/var/lib/argo/.install-state/`
  - Final smoke: `curl -s localhost:8080/api/v1/health` + `curl -s localhost:3000/`
  - Print operator-readable summary: bearer token, postgres password, URLs

- `deploy/vm/argo.service` (NEW, ≤ 50 LOC) — systemd unit:
  - `[Unit]` After=postgresql.service network-online.target
  - `[Service]` Type=notify, User=argo, Group=argo
  - WorkingDirectory=/var/lib/argo
  - EnvironmentFile=/etc/argo/argo.env
  - ExecStart=/usr/local/bin/argo gateway serve --bind 127.0.0.1:8080
  - Restart=on-failure, RestartSec=10s
  - Security hardening: NoNewPrivileges, ProtectSystem=strict, ProtectHome=yes, PrivateTmp=yes, ReadWritePaths=/var/lib/argo
  - `[Install]` WantedBy=multi-user.target

- `deploy/vm/web.service` (NEW, ≤ 45 LOC) — systemd unit:
  - After=argo.service
  - ExecStart=/usr/bin/node /var/lib/argo/web/server.js (Next.js standalone)
  - EnvironmentFile=/etc/argo/web.env
  - WorkingDirectory=/var/lib/argo/web
  - User=argo Restart=on-failure

- `deploy/vm/nginx-skycot.conf` (NEW, ≤ 100 LOC) — nginx site config:
  - `server { listen 80 default_server; }` redirects to https when TLS configured (certbot inserts the redirect)
  - `server { listen 443 ssl; }` — TLS terminates here
  - `location / { proxy_pass http://127.0.0.1:3000; }` — web
  - `location /api/ { proxy_pass http://127.0.0.1:8080/; }` — argo gateway
  - WebSocket upgrade headers for `/api/v1/inbox/ws` route
  - Security headers: HSTS, X-Frame-Options, Content-Security-Policy (basic)
  - Client max body 10M (for upload routes if any)

- `deploy/vm/certbot-renew.cron` (NEW, ≤ 30 LOC) — `/etc/cron.d/skycot-certbot`:
  - Renew twice daily at 03:23 and 15:23 (jittered to avoid LE rate-limit thundering herd)
  - Post-renew hook reloads nginx

- `deploy/vm/README.md` (NEW, ≤ 200 LOC) — operator runbook:
  - One-line install: `curl -fsSL https://<host>/install.sh | sudo bash -s -- --with-tls skycot.example.com` (commented note: in v1, operator clones repo + runs `sudo bash deploy/vm/install.sh` since no published artifact yet)
  - Manual install steps for operators who don't want curl-bash
  - Verifying: systemctl status, journalctl, curl checks
  - Updating: `cd /opt/argo && git pull && sudo bash deploy/vm/install.sh --update`
  - Backups: pg_dump via cron, copy /var/lib/argo
  - Hardening: ufw rules, fail2ban, ssh key-only auth
  - Troubleshooting: most-common failure modes (postgres init race, npm out-of-memory on small VMs, certbot rate-limit)
  - SLA expectation: **≤ 15 min** from fresh VM to working stack (NOT the v2-cited 10 min — v1 builds from source; pre-built artifact path is a follow-up)

This is NOT in scope: distros other than Ubuntu 24.04 LTS (Debian / Rocky / Amazon Linux — operators adapt; v1 is one supported target); pre-built binary release artifacts (separate brief — once GH Releases publishes signed tarballs, install.sh's `--use-release` flag activates; v1 builds from source); Docker on VM (operators who want Docker use K1); ansible / chef / puppet (operators integrate the script with their existing provisioning — out of scope to write providers); multi-VM HA setup (single VM only; HA is operator-managed via K2 k8s or external load-balancers); zero-downtime deploys (single-VM has restart-window; HA is upstream); systemd resource limits (operator tunes per workload; defaults are conservative); SELinux / AppArmor policies (Ubuntu default AppArmor profiles are fine for v1).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `deploy/vm/install.sh` (NEW, ≤ 280 LOC)
- `deploy/vm/argo.service` (NEW, ≤ 50 LOC)
- `deploy/vm/web.service` (NEW, ≤ 45 LOC)
- `deploy/vm/nginx-skycot.conf` (NEW, ≤ 100 LOC)
- `deploy/vm/certbot-renew.cron` (NEW, ≤ 30 LOC)
- `deploy/vm/README.md` (NEW, ≤ 200 LOC)

## Files you must NOT touch

- `deploy/docker/**` (K1's scope)
- `deploy/k8s/**` (K2's scope)
- `crates/**` (no Rust changes)
- `web/**` (no frontend changes)
- `.github/workflows/**` (K5's scope)

## Acceptance criteria

- [ ] `bash deploy/vm/install.sh --dry-run` on Ubuntu 24.04 prints planned actions without modifying system
- [ ] Fresh Ubuntu 24.04 VM (t3.medium / 2vCPU / 4GB RAM) → working stack in ≤ 15 min
- [ ] `systemctl status argo web nginx postgresql` all show `active (running)` post-install
- [ ] Curl localhost:80 returns 200 (HTTP) or 301 (redirect to HTTPS when `--with-tls` used)
- [ ] Curl localhost:8080/api/v1/health returns 200
- [ ] Re-running `install.sh` is a no-op (idempotent) — second run completes in < 30s, no errors
- [ ] `--with-tls <domain>` invokes certbot non-interactively + cert is installed
- [ ] systemd units use security hardening directives (NoNewPrivileges, ProtectSystem)
- [ ] postgres password is generated, stored at `/var/lib/argo/.pgpass` 0400 owned by argo user
- [ ] Bearer token is generated via `openssl rand -hex 32` + printed in install summary
- [ ] All install.sh stderr/stdout goes to `/var/log/argo-install.log` for forensics
- [ ] README documents update path + backup recipe + 5+ troubleshooting entries
- [ ] No new file > 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-K3-deploy-single-vm-systemd`
- PR title: `deploy: single-VM systemd recipe (Ubuntu 24.04 + nginx + certbot) (K3)`

## Notes for the implementer

- Refuse-to-run-on-wrong-OS check at top of install.sh: `[ "$(lsb_release -si)" = "Ubuntu" ] && [ "$(lsb_release -sr)" = "24.04" ] || { echo "Ubuntu 24.04 required; got $(lsb_release -d)"; exit 1; }`
- Idempotency via marker files: each step `step_install_postgres`, `step_create_argo_user`, etc. checks for its marker at `/var/lib/argo/.install-state/<step>` and skips if present. Marker is `touch`'d on success.
- The `--update` flag does NOT re-run user creation / postgres init / etc. — it pulls + rebuilds the binary, copies new systemd units (if changed), and restarts services.
- Build-from-source on small VMs is the failure mode: 2GB RAM VMs OOM during `cargo build --release`. Document this loudly in the README and recommend ≥ 4GB. The `--use-release` flag (when published artifacts exist) sidesteps this.
- nginx config's `proxy_pass` for `/api/` strips the prefix: `proxy_pass http://127.0.0.1:8080/;` (trailing slash matters). Test that `/api/v1/health` correctly reaches argo's `/api/v1/health`.
- WebSocket upgrade for `/api/v1/inbox/ws` (used by approvals-inbox live tier per E1-f) requires `proxy_http_version 1.1` + `Upgrade $http_upgrade` + `Connection "upgrade"` headers — document in the nginx config inline.
- DO NOT use `curl -fsSL | bash` as the SOLE install path. Operators rightfully distrust curl-pipe-bash. The README must show the manual git-clone + bash install.sh path FIRST, with the curl-bash as a convenience option after.
- Certbot's `--nginx` plugin handles the nginx config rewrite automatically. Don't try to hand-write the TLS server block; let certbot do it on first run.
- The cron entry `23 3,15 * * *` (jittered 23 minutes) avoids the LE rate-limit thundering-herd that happens when everyone runs at midnight UTC.
- Backup recipe in README: `0 4 * * * sudo -u argo pg_dump argo > /var/lib/argo/backups/$(date +%F).sql` — pair with rsync to off-host storage.
- DO NOT bundle the binary into the install.sh script. v1 builds; the script is a recipe, not a release vehicle.
- DO NOT auto-configure firewalls; the README recommends operator-managed `ufw allow 22 80 443` after verifying SSH still works.
- The K5 smoke test workflow does NOT exercise this recipe (K5 uses K1 docker-compose for speed). Manual verification on a real VM is the acceptance gate.
