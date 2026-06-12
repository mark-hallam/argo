# SKYCOT single-VM deployment recipe

Targets a single **Ubuntu 24.04 LTS** VM (DigitalOcean droplet, Hetzner
CX22, EC2 `t3.medium`). Not containerised. argo and the web SPA run as
systemd services; nginx reverse-proxies both; postgres 16 is co-located.
For docker-compose see `deploy/docker-compose.yml`; for Kubernetes see `deploy/k8s/`.

> **SLA**: ≤ 15 min from fresh VM to working stack on 2 vCPU / 4 GB RAM.
> The `--use-release` artifact path will drop this to ~3 min once GH
> Releases publishes signed tarballs; v1 builds from source.

## What gets installed

| Component | Location | Service |
|-----------|----------|---------|
| argo binary | `/usr/local/bin/argo` | `argo.service` (:8080 loopback) |
| web SPA | `/var/lib/argo/web` | `web.service` (:3000 loopback) |
| postgres 16 | distro default | `postgresql.service` |
| nginx | distro default | `nginx.service` (:80, :443 after TLS) |
| Source clone | `/opt/argo` | — |
| Config + env | `/etc/argo/{argo,web}.env` | mode 0640 root:argo |
| Install log | `/var/log/argo-install.log` | — |
| Idempotency markers | `/var/lib/argo/.install-state/` | — |

The argo binary stores its SQLite data + credentials under
`/var/lib/argo` (`ARGO_HOME`). Postgres is provisioned with a generated
password persisted at `/var/lib/argo/.pgpass` (0400, owned by `argo`).

## Quick install (manual — recommended)

Operators rightly distrust `curl … | sudo bash`. Prefer a manual clone:

```bash
sudo apt-get update && sudo apt-get install -y git
git clone https://github.com/mark-hallam/Argo-Agent.git && cd Argo-Agent
sudo bash deploy/vm/install.sh --dry-run                    # plan only
sudo bash deploy/vm/install.sh                              # HTTP only
sudo bash deploy/vm/install.sh --with-tls skycot.example.com # HTTPS via LE
```

The installer is idempotent — re-running on a healthy host completes in
< 30 s. Each major step records a marker at
`/var/lib/argo/.install-state/<step>` and is skipped on re-run.

## Quick install (curl-bash — convenience)

Once GH Releases publishes the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/deploy/vm/install.sh \
  | sudo bash -s -- --with-tls skycot.example.com
```

Until then, use the manual git-clone path above.

## Verifying

```bash
systemctl status argo web nginx postgresql      # all four `active (running)`
curl -fsS http://127.0.0.1:8080/api/v1/health   # {"status":"ok",...}
curl -I http://127.0.0.1/                       # 200, or 301 → HTTPS after certbot
journalctl -u argo -u web -f                    # live logs
```

The bearer token + postgres password are printed at the tail of the
install output and persisted in `/etc/argo/argo.env`. Treat the token
as a secret — it's the gateway's only auth.

## Updating

```bash
cd /opt/argo
sudo git pull --ff-only
sudo bash deploy/vm/install.sh --update
```

`--update` skips user creation, postgres init, and env generation — it
resyncs source, rebuilds the binary + web, re-installs systemd units,
and restarts the services.

## TLS / certificates

`--with-tls <domain>` invokes `certbot --nginx --non-interactive
--agree-tos --redirect -d <domain>`. certbot rewrites
`/etc/nginx/sites-available/skycot` in place to add the `listen 443
ssl` block and an HTTP→HTTPS redirect. **Do not hand-edit the 443
server block** — let certbot manage it so renewals don't conflict.

Auto-renewal: `/etc/cron.d/skycot-certbot` runs twice daily at `:23`
past hours 3 and 15 UTC (jittered to dodge the LE thundering-herd at
midnight UTC). Force a renewal dry-run: `sudo certbot renew --dry-run`.

## Backups (offsite, verified, drillable)

> The old `pg_dump` recipe here was stale — the platform's state is a
> **SQLite** store written by the argo binary, not a Postgres database.
> Use the first-class `argo backup` tooling instead.

All platform state lives in the gateway's data dir (`sessions.db` plus,
during live operation, its WAL sidecars). **Backups require the gateway
to store that db under `<workspace>/.argo/`** — i.e. run it with
`ARGO_HOME=/var/lib/argo/.argo` and `WorkingDirectory=/var/lib/argo`, so
`argo backup create` (which snapshots `<cwd>/.argo`) captures the real
db. One-time migration for an existing host:

```bash
sudo systemctl stop argo
sudo -u argo mkdir -p /var/lib/argo/.argo
sudo -u argo bash -c 'mv /var/lib/argo/sessions.db /var/lib/argo/.bootstrap-token /var/lib/argo/credentials.toml /var/lib/argo/.argo/ 2>/dev/null || true'
# set ARGO_HOME=/var/lib/argo/.argo in /etc/argo/argo.env, then:
sudo systemctl start argo
```

Then install the scheduled offsite backup (create → verify → S3 → prune):

```bash
sudo cp scripts/backup-cron.sh /usr/local/bin/argo-backup-cron.sh
sudo cp deploy/systemd/argo-backup.{service,timer} /etc/systemd/system/
# S3_BUCKET / S3_PREFIX / RETENTION_DAYS / ARGO_WORKSPACE=/var/lib/argo
sudoedit /etc/argo/backup.env
sudo systemctl daemon-reload && sudo systemctl enable --now argo-backup.timer
systemctl list-timers argo-backup.timer            # confirm next run
```

`backup-cron.sh` is **truth-bounded**: it fails loud (non-zero, unit goes
`failed`) if the db is missing, a verify fails, or the S3 upload does not
land — it never reports a silent empty backup as ok.

**Restore drill — run it; a backup that has never restored is
unverified.** `restore-drill.sh` pulls the latest tarball from S3,
restores it into a throwaway dir, boots a gateway against the restored
db, and asserts `/readyz` is ok:

```bash
S3_BUCKET=skycot-prod-backups bash scripts/restore-drill.sh            # from S3
bash scripts/restore-drill.sh --local /var/lib/argo/.argo/backups/<ts>.tar.gz
```

Real disaster recovery into the live host: stop argo, `argo restore
<tarball>` into `/var/lib/argo` (reproduces `.argo/`), `systemctl start
argo web`.

## Hardening

systemd units already apply `NoNewPrivileges`, `ProtectSystem=strict`,
`ProtectHome`, `PrivateTmp`, and kernel protections. Operator steps:

```bash
sudo ufw allow 22/tcp && sudo ufw allow 80/tcp && sudo ufw allow 443/tcp
sudo ufw enable                          # verify SSH reachable first!
sudo apt-get install -y fail2ban && sudo systemctl enable --now fail2ban
# In /etc/ssh/sshd_config: PasswordAuthentication no / PermitRootLogin no
sudo systemctl reload ssh
```

## Troubleshooting

1. **`cargo build --release` OOM-kills on 2 GB VMs.** Most common
   failure mode; the SLA assumes ≥ 4 GB. Workaround: add 4 GB swap
   (`fallocate -l 4G /swapfile && chmod 600 /swapfile && mkswap
   /swapfile && swapon /swapfile`) before re-running. Permanent fix:
   resize the VM or wait for `--use-release`.

2. **Postgres init race / `connection refused`.** If `postgresql.service`
   hasn't fully started before the role-create step, the installer
   waits inside `systemctl enable --now`. Stuck install? Check
   `journalctl -u postgresql`; on Hetzner Cloud the usual cause is
   `pg_createcluster` racing cloud-init. Re-run the installer once
   postgres reports `active (running)`.

3. **certbot rate-limit (5 certs / domain / week).** Letting certbot
   loop after failures trips the LE limit (`too many certificates
   already issued`). Wait 7 days, or use `--test-cert` while iterating.

4. **`npm install` fails `EBADENGINE`.** Ubuntu 24.04 ships Node 18; the
   web app targets Node 20. Install from NodeSource and re-run:
   ```bash
   curl -fsSL https://deb.nodesource.com/setup_20.x | sudo bash -
   sudo apt-get install -y nodejs
   ```

5. **Gateway refuses to start: `port 8080 already bound`.** `ss -ltnp
   'sport = :8080'` to find the offender. argo binds 127.0.0.1:8080;
   nginx is the only public listener.

6. **nginx 502 to `/api/`.** argo is down. Check `journalctl -u argo -n
   200`. Post-update, the usual culprit is a schema-migration failure
   (argo exits 78 on chain-hash mismatch); see
   `docs/skycot/runbook/incident-response.md`.

7. **WebSocket disconnects every minute.** nginx's default
   `proxy_read_timeout` (60 s) kills idle approvals-inbox sockets. The
   `/api/v1/inbox/ws` block raises it to 3600 s — restore the long
   timeout if you've hand-edited the config.

## Sensible deviations from the brief

- **Binary command.** Brief cited `argo gateway serve --bind
  127.0.0.1:8080`; real CLI is `argo gateway start --bind 127.0.0.1
  --port 8080`. systemd unit uses the real form.
- **`Type=notify` → `Type=exec`.** argo does not call `sd_notify`, so
  `notify` would block the unit in `activating` until `TimeoutStartSec`
  expired. `exec` matches actual readiness semantics.
- **`node server.js` → `npm start`.** Web app is not configured for
  Next.js standalone output; conventional entrypoint is `next start`
  via `npm start`. Switching to standalone is a follow-up.

## Out of scope

Other distros (Debian, Rocky, Amazon Linux — operators adapt); pre-built
release artifacts (separate brief — `--use-release` is wired but errors
until artifacts land); Docker on VM (use `deploy/docker-compose.yml`); ansible /
chef / puppet providers; multi-VM HA (single VM only; use
`deploy/k8s/`); zero-downtime deploys (single-VM has a restart window).

Bug reports: include `journalctl -u argo -u web -n 500`,
`/var/log/argo-install.log`, and `sudo -u argo /usr/local/bin/argo doctor`.
