# SKYCOT — Morning staging runbook: stage the gated private preview (2026-06-10)

> **What this gets you:** a private, password-gated URL (`https://try.skycot.com`)
> you can point investors / a waitlist at. Behind it: the gateway on the host
> loopback (so the seeded demo stays viewable, no login wall), the SPA showing the
> seeded demo, and **nginx** as the only public door — TLS + an HTTP basic-auth
> gate (one shared username + password). Architecture lock: DEC-032 / DEC-034 WS5.
>
> **Plain-language glossary (terms used below):**
> - **gateway** — the SKYCOT server (the Rust `argo` binary) that serves the API and data.
> - **SPA** (Single-Page Application) — the website front-end (Next.js) that the browser shows.
> - **nginx** — a web server that sits in front and routes traffic; here it adds TLS + the password gate.
> - **TLS / cert** — the padlock (HTTPS). The cert is the file that proves the site is `try.skycot.com`.
> - **basic-auth / `.htpasswd`** — a simple username+password prompt the browser shows before letting anyone in.
> - **loopback (`127.0.0.1`)** — "this machine only"; the gateway listens here so it is NOT public and stays in demo-viewable mode. nginx is the public part.
> - **EC2 instance** — a rented Linux computer in AWS where all of this runs.
> - **Elastic IP** — a fixed public address for that computer (so DNS keeps pointing at it).
> - **DNS `A` record** — the phone-book entry mapping `try.skycot.com` → that fixed address.
> - **certbot / Let's Encrypt** — the free tool that issues the TLS cert.
> - **container / docker compose** — the three pieces (gateway, SPA, nginx) packaged and started together with one command.
> - **secret / env var** — a password-like value (API key, DB key) the gateway reads from a file at start-up.
> - **seed** — load the polished demo data (workers, approvals, skills, the workflow, audit rows) so the URL isn't empty.

**This is the CANONICAL stack:** `deploy/Dockerfile.gateway` + `deploy/Dockerfile.web` +
`deploy/docker-compose.yml` + `deploy/nginx.conf`. **Do NOT use `deploy/docker/`** —
that legacy stack does not bundle the gateway's policies/skill-library and is being
retired.

---

## ✅ READ FIRST — TWO DEPLOY-BLOCKERS found 2026-06-09, now FIXED IN-REPO by the conductor

**Both blockers below were reproduced by an actual `docker build` and then FIXED in-repo
the same night** (`deploy/Dockerfile.web` + `deploy/Dockerfile.gateway`). So if you are
building from current `main` you do **not** need to hand-patch anything — `docker compose
build` should succeed. (Web fix build-verified by the conductor; gateway fix is the
standard missing-system-lib add. If you are building from a checkout *before* the fix
commit, apply the minimal changes documented below.)

1. **Web image — missing build-arg.** `deploy/Dockerfile.web` runs `pnpm run build`,
   whose `prebuild` guard (`web/scripts/check-canonical-gateway-port.mjs`) FAILS a
   production build unless `NEXT_PUBLIC_GATEWAY_URL` is set to an **HTTPS** URL. The
   Dockerfile only passes the two demo-mode flags. Error:
   `STRICT: NEXT_PUBLIC_GATEWAY_URL ... is unset. ... Refusing to ship build.`
   **Fix:** add `NEXT_PUBLIC_GATEWAY_URL=https://try.skycot.com` as a build-arg in
   `deploy/Dockerfile.web` + `deploy/docker-compose.yml` web `args:`. (The script
   explicitly accepts any `https://` override — line 86 — and bakes it into the CSP.)

2. **Gateway image — missing system library in the builder stage.** The Rust build
   pulls in `libdbus-sys` (via `keyring`/`dbus-secret-service` in `argo-config`, the
   Linux OS-keychain path), which needs the `dbus-1` dev library at compile time. The
   builder stage installs only `pkg-config libssl-dev`. Error (verbatim from the build):
   `The system library dbus-1 required by crate libdbus-sys was not found ... 'libdbus-1-dev' and 'pkg-config' are installed`.
   **Fix:** add `libdbus-1-dev` to the builder-stage `apt-get install` line in
   `deploy/Dockerfile.gateway` (line ~19–20).

Until both land, Step 6's `build` will fail. If the overnight brief hasn't merged by
the time you start, apply the two one-line edits above by hand before Step 6.

**Deploy discipline (DEC-029, no theater):** run this line-by-line. Read → understand →
run → confirm the output matches the "✅ success looks like" note → only then go to the
next step. **If any output is not what the step predicts, STOP — do not continue.**

---

## PREREQUISITES — things only you (the operator) provide

Have these ready before you start. Nothing below can proceed without them.

| # | What | How to get it / note |
|---|------|----------------------|
| P1 | **AWS account** in **ap-southeast-2** (Sydney) with the AWS credits applied | Console login. The whole thing costs ~$16–18/month, so the credits last years. |
| P2 | **An EC2 instance** (we launch it in Step 1) | Amazon Linux 2023, ARM `t4g.small` (2 vCPU / 2 GB). Bump to `t4g.medium` only if the Rust build runs out of memory. 30 GB gp3 disk. |
| P3 | **DNS control for `skycot.com`** | You'll add one `A` record `try.skycot.com` → the instance's Elastic IP. (If `try.` is currently a Cloudflare CNAME from the older Fargate plan, you'll repoint it — see Step 2 note.) |
| P4 | **The 3 gateway secrets** (paste into `deploy/.env.deploy` in Step 5) | • `ANTHROPIC_API_KEY` — the pitch/agent LLM (REQUIRED — gateway refuses to boot with no provider key). <br>• `OPENAI_API_KEY` — voice (speech-to-text / text-to-speech). <br>• `SKYCOT_DB_KEY_BASE64` — generate on the instance with `openssl rand -base64 32`. |
| P5 | **A gate username + password** for the basic-auth door (Step 4) | This is what you hand out to the people you point at the preview. Pick something you can share over a call. |
| P6 | **Stripe keys are NOT needed.** | Billing is not exercised by a gated *view*-only preview. Skip Stripe entirely for this. |

> **Cert note:** the nginx config expects the TLS cert at
> `/etc/letsencrypt/live/try.skycot.com/`. You do NOT provide this in advance —
> certbot creates it in Step 3. But nginx **will not start until that file exists**,
> so Step 3 must finish before Step 6. (This is the one real sequencing trap; the
> runbook handles it.)

---

## Step 0 — Validate locally before touching AWS (optional, ~2 min, on your Mac)

These are no-op checks that catch a typo before it costs you a server round-trip.
They need Docker Desktop running on your Mac.

```bash
cd ~/code/argo
# compose needs the env file to LITERALLY exist (the gateway service has env_file: .env.deploy).
# Make a throwaway one just for the syntax check, then delete it:
printf 'ANTHROPIC_API_KEY=x\nOPENAI_API_KEY=x\nSKYCOT_DB_KEY_BASE64=x\nARGO_HOME=/data\n' > deploy/.env.deploy
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.deploy config >/dev/null && echo "COMPOSE OK"
rm -f deploy/.env.deploy
```
✅ **Success looks like:** prints `COMPOSE OK`. (If it errors about a missing
`.env.deploy`, the file wasn't created — re-run the `printf` line.)

> Note: `nginx -t` is NOT useful to run locally — it tries to load the real TLS cert
> and fails because the cert only exists on the server after Step 3. That failure is
> expected locally and is not a config bug.

---

## Step 1 — Launch the instance (AWS console, ap-southeast-2)

- **EC2 → Launch instance** → Amazon Linux 2023, **`t4g.small`** (ARM).
- **Storage:** 30 GB gp3.
- **Security group (the firewall) — inbound rules, exactly these:**
  - `443` (HTTPS) from `0.0.0.0/0` — the public door.
  - `80` (HTTP) from `0.0.0.0/0` — needed ONLY so certbot can prove the domain; nginx also uses it to redirect to 443.
  - `22` (SSH) from **your IP only** — so you can log in.
  - **Do NOT open `3000` or `7400`.** Those are loopback-only and must never be public.
- **Elastic IP:** allocate one and associate it with the instance.

✅ **Success looks like:** instance state `running`; you have a fixed public IP (the Elastic IP).

---

## Step 2 — Point DNS at the instance

- In your DNS (Cloudflare, since `skycot.com` is on Cloudflare): add/repoint
  `try.skycot.com` → an **`A` record** to the **Elastic IP** from Step 1.
- **If `try.skycot.com` is currently a proxied CNAME** (orange cloud, from the
  earlier Fargate/ALB plan in `deploy/dns/`): change it to an **`A` record → Elastic IP**,
  and set the proxy to **DNS-only (grey cloud)** for now. certbot's HTTP challenge
  in Step 3 needs to reach your instance directly; Cloudflare's proxy can interfere
  with the first issuance. You can re-enable the orange cloud after the cert is issued.

✅ **Success looks like:**
```bash
dig +short try.skycot.com    # returns your Elastic IP
```
Give DNS a few minutes to propagate before Step 3.

---

## Step 3 — SSH in, install Docker, get the code, issue the TLS cert

```bash
ssh ec2-user@<elastic-ip>

# Docker + git + the basic-auth + cert tools:
sudo dnf install -y docker git httpd-tools
sudo systemctl enable --now docker
sudo usermod -aG docker ec2-user        # then log out + back in so docker works without sudo
exit
ssh ec2-user@<elastic-ip>               # reconnect to pick up the docker group

# Get the repo:
git clone <your repo URL> skycot && cd skycot
```

**Now issue the cert (do this BEFORE starting the stack — nginx needs it to start).**
Use certbot in standalone mode (it binds port 80 briefly to prove the domain):

```bash
sudo dnf install -y certbot
sudo certbot certonly --standalone -d try.skycot.com --agree-tos -m <your-email> --non-interactive
```
✅ **Success looks like:** `Successfully received certificate` and the files exist:
```bash
sudo ls -l /etc/letsencrypt/live/try.skycot.com/fullchain.pem   # must exist before Step 6
```
> If this fails with a connection/timeout error, DNS hasn't propagated or port 80
> isn't open / Cloudflare proxy is still on — fix that, then re-run. **Do not skip.**

---

## Step 4 — Create the basic-auth gate file

```bash
cd ~/skycot
htpasswd -c deploy/.htpasswd <gate-username>     # prompts twice for the gate PASSWORD (P5)
```
✅ **Success looks like:** `deploy/.htpasswd` exists with one line:
```bash
cat deploy/.htpasswd        # shows: <gate-username>:$apr1$....hashed....
```
(The compose file mounts this read-only into nginx as `/etc/nginx/.htpasswd`.)

---

## Step 5 — Fill in the secrets file

```bash
cd ~/skycot
cp deploy/.env.deploy.example deploy/.env.deploy
chmod 600 deploy/.env.deploy
# Generate the DB key right here and note it:
openssl rand -base64 32
# Now edit deploy/.env.deploy and paste:
#   ANTHROPIC_API_KEY=sk-ant-...        (REQUIRED — boot fails without a provider key)
#   OPENAI_API_KEY=sk-...               (voice)
#   SKYCOT_DB_KEY_BASE64=<the openssl output above>
#   ARGO_HOME=/data                     (already set in the template — leave it)
nano deploy/.env.deploy
```
✅ **Success looks like:**
```bash
ls -l deploy/.env.deploy        # -rw------- (chmod 600)
grep -c '=' deploy/.env.deploy  # 4 (the four set lines)
```
> **Why no DB-key drama:** the gateway binds `127.0.0.1` (loopback), so it runs in
> *demo-viewable* mode, NOT production mode — the SQLCipher at-rest-encryption boot
> gate does NOT fire here. The DB key is read and used but the stack does **not**
> require a SQLCipher-special build. (That requirement only applies to the
> scale-later non-loopback "real production" path, not this gated preview.)

---

## Step 6 — Build, then start the stack (only after Steps 3–5 all pass)

**Build first (separate from starting) so a build error can't half-start the stack:**
```bash
cd ~/skycot
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.deploy build
```
✅ **Success looks like:** both images build with no error (`gateway` Rust build is the
slow one — several minutes on first run; `web` builds the Next.js SPA).

> If the Rust build is killed for memory on `t4g.small` (2 GB), either stop here and
> resize the instance to `t4g.medium`, then re-run `build`; or build the gateway image
> on your Mac and push it to ECR. Don't fight a thrashing build.

**Then start (no `--build` here, so no surprise rebuild runs unreviewed):**
```bash
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.deploy up -d
docker compose -f deploy/docker-compose.yml ps          # expect gateway + web + nginx all "Up"
docker compose -f deploy/docker-compose.yml logs -f gateway   # watch for "listening", then Ctrl-C
```
✅ **Success looks like:** `ps` shows all three `Up`; the gateway log reaches a
`listening` line with no error. If **nginx** is the one that's not Up, the cert from
Step 3 is missing — check `sudo ls /etc/letsencrypt/live/try.skycot.com/`.

---

## Step 7 — Seed the demo data (so the URL shows the polished demo, not an empty workspace)

```bash
# 7a. Create the demo tenant; capture its "id" from the JSON output:
docker compose -f deploy/docker-compose.yml exec gateway \
  argo tenant create "SKYCOT Demo" --json
#     → copy the "id" value (a UUID).

# 7b. Seed that tenant with the canonical v1.0 demo data.
#     --skip-pass-a-turns seeds the demo with NO extra LLM calls (Pass B dressing +
#     scaffolding). Drop the flag (and set ARGO_DEMO_SEED_PROVIDER_KEY) only if you
#     want the full real-LLM Pass A turns seeded too.
docker compose -f deploy/docker-compose.yml exec gateway \
  argo demo seed --tenant <ID-FROM-7a> --skip-pass-a-turns
```
✅ **Success looks like:** the seed command finishes without error and reports the rows
it created. (These commands and flags are confirmed against the binary — see
"VERIFIED-BY-AGENT" below.)

> **Known refine-at-deploy (DEPLOYMENT.md #1):** the SPA's demo-mode must resolve to
> the SAME tenant id you just seeded. If the demo pages render but show empty data,
> the seed targeted a different tenant than the SPA resolves — a one-row workspace
> re-pin fixes it (same fix used locally). Check the gateway log for the tenant id the
> SPA's requests carry vs the id from 7a.

---

## Step 8 — Smoke-test the gate + that the demo loads (truth-bounded — actually run these)

```bash
# (a) The gate is closed to anonymous requests: a request with NO credentials = 401.
curl -skI https://try.skycot.com/ | head -1
#     ✅ expect:  HTTP/2 401   (the basic-auth gate is working)

# (b) WITH the gate credentials, the gateway health endpoint answers:
curl -sk -u <gate-user>:<gate-pass> https://try.skycot.com/health
#     ✅ expect:  {"status":"ok", ...}

# (c) WITH credentials, the seeded approvals data is present:
curl -sk -u <gate-user>:<gate-pass> https://try.skycot.com/api/v1/approvals | jq '.items | length'
#     ✅ expect:  a non-zero count (the seeded approval rows; ~14 in the canonical seed)
```

Then open it like a real visitor:
- Browse to **`https://try.skycot.com`** → the browser shows the **username/password
  prompt** → enter the gate user/pass → the **seeded SKYCOT demo loads** (workers,
  approvals, skills, the workflow, audit). The padlock is valid (no cert warning).
- **Click into a worker / the approvals inbox / a skill** and confirm pages render the
  seeded data (not "No activity yet" / not a mock overlay).
- If the live-approvals panel uses a websocket, confirm in the browser Network tab it
  connects to `wss://try.skycot.com/inbox/...` (same-origin), **not** an absolute
  `127.0.0.1` URL (DEPLOYMENT.md refine-item #5).

✅ **You are live (gated)** when (a) returns 401, (b)/(c) return data WITH the
password, and the browser shows the seeded demo behind the prompt with a valid padlock.

## Step 9 — Hand out access

Give the gate username + password (P5) to the investors / waitlist people you're
pointing at it. That shared credential is the gate. To revoke later, re-run Step 4 with
a new password and `docker compose ... restart nginx`.

---

## Teardown / rollback (if anything is wrong)

```bash
docker compose -f deploy/docker-compose.yml down        # stops all three; data volume survives
# To wipe seeded data too:  docker compose -f deploy/docker-compose.yml down -v
```
A bad cert? `sudo certbot delete --cert-name try.skycot.com` then redo Step 3. The
SQLite data lives on the EBS disk at `/data` (the `argo-data` volume) and survives
container restarts — snapshot the EBS volume before any risky change.

---

## VERIFIED-BY-AGENT vs UNVERIFIED (NO THEATER — exactly what was/wasn't run)

**Prepared + checked on Mark's Mac, 2026-06-09, against the repo at `~/code/argo`:**

VERIFIED (actually run / read this session):
- ✅ **`docker compose -f deploy/docker-compose.yml config` PASSES** (exit 0) once a
  `deploy/.env.deploy` file exists — confirmed the `gateway`/`web`/`nginx` services,
  `network_mode: host`, the `argo-data` volume, the `.htpasswd` + `/etc/letsencrypt`
  mounts, and the `NEXT_PUBLIC_SKYCOT_DEMO_MODE/_LIVE=1` build-args all render. **Gotcha
  proven:** `config` FAILS if `deploy/.env.deploy` does not literally exist (the
  service-level `env_file:` directive) — `--env-file` alone is not enough. Step 0 + Step 5 handle this.
- ✅ **The gateway Dockerfile bundles the runtime deps.** All COPY sources exist and are
  non-empty: `manifests/` (24 files), `policies/` (2) incl. `policies/truth-boundary/`,
  `skill-library/` incl. the pitch corpus at
  `skill-library/canonical-qa/corpora/skycot/` (5 files). The `ARGO_MANIFEST_DIR` /
  `ARGO_PITCH_CORPUS_DIR` / `ARGO_PITCH_POLICY_DIR` env targets all resolve to real dirs.
- ✅ **Loopback bind = NOT production mode = no SQLCipher boot gate.** Read the source:
  `boot.rs:200` `enforce_enterprise_encryption_at_boot` returns early when
  `is_production_mode_for_bind("127.0.0.1") == false`; the JWT-secret boot-guard
  (`startup_check.rs`) is likewise production-only. So the canonical loopback stack does
  **not** need the `bundled-sqlcipher` build that `deploy/secrets/README.md` warns about
  — that warning is for the scale-later non-loopback path. **No deploy-blocker here.**
- ✅ **A provider key IS required to boot.** `startup_check.rs` `provider_key_present()`
  is unconditional — the gateway refuses to bind with no `ANTHROPIC_API_KEY`/`OPENAI_API_KEY`.
  That's why P4's `ANTHROPIC_API_KEY` is marked REQUIRED, not optional.
- ✅ **The seed commands are real and the flags are correct.** Confirmed against the CLI
  source: `argo tenant create <NAME> --json` (`TenantCmd::Create`) and
  `argo demo seed --tenant <ID> [--skip-pass-a-turns]` (`DemoCmd::Seed`, `--tenant` +
  `--skip-pass-a-turns` flags exist; profile defaults to `v1-demo-canonical`).
- ✅ **Legacy stack steer-away confirmed.** `deploy/docker/` does NOT bundle
  policies/skill-library; the canonical `deploy/Dockerfile.gateway` does. Use the canonical one.
- ✅ **Build arch matches the target.** This Mac is `arm64`; the runbook's `t4g` instance
  is ARM — same arch. (If you ever switch to an x86 instance, build on it or pass `--platform`.)
- ❌ **BOTH image builds were RUN this session and BOTH FAILED** (Docker daemon brought
  up; `docker build -f deploy/Dockerfile.gateway .` and `... Dockerfile.web .`). Real,
  reproduced errors — see the "⛔ READ FIRST" section at the top:
  - **web:** prebuild `check-canonical-gateway-port.mjs` rejected the build —
    `NEXT_PUBLIC_GATEWAY_URL` unset. Needs the build-arg added.
  - **gateway:** Rust compile failed at `libdbus-sys v0.2.7` — `dbus-1` system library
    not found in the builder stage. Needs `libdbus-1-dev` in the builder `apt-get`.
  Both are one-line Dockerfile/compose fixes (this agent did NOT apply them — out of
  scope; flagged for the overnight fix-brief).

UNVERIFIED (cannot be checked without the live AWS instance — by design):
- ❌ The actual EC2 launch, the Elastic IP, the security-group rules.
- ❌ DNS propagation of `try.skycot.com` → the Elastic IP.
- ❌ The certbot issuance + the live `/etc/letsencrypt/live/try.skycot.com/` cert.
- ❌ `nginx -t` / nginx start **with the real cert present** (it correctly FAILS without
  the cert — that's the Step-3-before-Step-6 sequencing trap, not a config bug).
- ❌ The end-to-end live smoke test (Step 8) against `https://try.skycot.com`.
- ❌ Demo-tenant ↔ SPA-demo-mode id agreement on the server path (DEPLOYMENT.md refine #1)
  and the same-origin `wss://` approvals socket (refine #5) — verify live in Step 7/8.

**The one sequencing trap to not trip:** certbot (Step 3) MUST finish before
`docker compose up` (Step 6), because nginx refuses to start without the cert file at
`/etc/letsencrypt/live/try.skycot.com/fullchain.pem`. Everything else is linear.
