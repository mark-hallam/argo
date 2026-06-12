# SKYCOT — Deployment plan & runbook (gated, pointable, AWS)

> **Phase:** Deployment Readiness ([DEC-032](../decisions/DEC-032-deployment-readiness-phase.md)).
> **Goal:** a **private, gated URL** you can point investors / a waitlist at — the
> bridge from "demo on Mark's Mac" to "a thing someone can open." On the ~$800
> AWS credits.
> **Division of labour:** Mark does the AWS account, billing, domain/DNS. The
> conductor preps everything else (the `deploy/` kit + this runbook).
> **Truth-bounded ([DEC-029](../decisions/DEC-029-truth-bounded-status-reporting.md)):**
> everything here is *prepped + locally sound* (the SPA prod-build is verified
> green); the live deploy is **unverified until the instance exists.**

---

## TL;DR — the recommended shape

A **single small EC2 instance** runs three containers behind **nginx**:

```
        Internet (HTTPS, your domain e.g. try.skycot.com)
                         │
                   ┌─────▼─────┐   nginx  (0.0.0.0:443)
                   │   nginx   │   • TLS termination
                   │  (gate)   │   • basic-auth GATE  (private but pointable)
                   └─────┬─────┘   • routes by path
              ┌──────────┼───────────────┐
              │ /api/v1/*, /health,       │  everything else
              │ /inbox/ws  → gateway      │  → SPA
        ┌─────▼─────┐                ┌────▼─────┐
        │  gateway  │ 127.0.0.1:7400 │   SPA    │ 127.0.0.1:3000
        │ (Rust)    │  LOOPBACK      │ (Next)   │  Next standalone
        └─────┬─────┘                └──────────┘
              │ SQLite @ /data (EBS volume, persistent)
              │ secrets via env (SSM/Secrets Manager or chmod-600 file)
```

**Why loopback + nginx (the load-bearing decision):** the gateway flips into
**production-auth mode (bearer-token required)** the moment it binds a
*non-loopback* address (`crates/argo-cli/src/handlers/gateway/routes_auth_production_mode.rs`:
`is_production_mode = profile==production || bind-is-non-loopback`). For a
**gated demo we want the seeded data viewable** (no login wall), so the gateway
stays on `127.0.0.1` (NOT prod-mode → demo-bypass works), and **nginx** provides
the public exposure + TLS + the privacy gate (HTTP basic-auth). People you give
the username/password to see the full seeded demo; everyone else gets a 401.

This is the cheapest, simplest, most controllable shape for a pre-seed gated
demo. The "real production" shape (App Runner/ECS/Amplify/RDS + magic-link auth)
is documented at the bottom for when you onboard real users.

---

## ⚠️ Validate every line before you run it (the deploy discipline)

**This deploy is run line-by-line, validated first — nothing is blind-pasted.**
These commands touch real infrastructure + real secrets, so the rule is:
**read → understand → validate → only then run**, one step at a time, together.
The conductor explains each command, confirms it against the actual state, we
run it, we check the output matches what we predicted, and only then move to the
next line. **If any output is not what we predicted — stop, don't continue.**

**Dry-run / preview first (validate without side effects).** Before the
side-effectful steps, use the no-op validators that exist:

```bash
# 1. Render + validate the FULL compose (env substitution, syntax, image refs)
#    WITHOUT starting anything. Catches a bad .env.deploy or a typo'd service.
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.deploy config

# 2. Build the images FIRST (validates BOTH Dockerfiles + every COPY) —
#    separate from starting them, so a build error never half-starts the stack.
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.deploy build

# 3. Test the nginx config syntax before it ever serves traffic.
docker run --rm -v "$PWD/deploy/nginx.conf:/etc/nginx/nginx.conf:ro" \
  nginx:1.27-alpine nginx -t

# 4. Confirm the gateway's REAL flags + the exact seed subcommand (step 6 is a
#    documented unknown, refine-item #2) BEFORE relying on them — read the help.
docker compose -f deploy/docker-compose.yml run --rm gateway argo --help
docker compose -f deploy/docker-compose.yml run --rm gateway argo gateway --help

# 5. Eyeball the secrets file is filled + locked (this does NOT print values):
ls -l deploy/.env.deploy            # expect -rw------- (chmod 600)
grep -c '=' deploy/.env.deploy      # sanity: the expected count of set vars

# 6. PRE-DEPLOY GATE — bring the SAME canonical images up locally end-to-end and
#    assert /health + the SPA + the hire/approve/metrics routes actually serve.
#    This is the proof the stack RUNS (not just "builds"); the last green run is
#    recorded in deploy/smoke/LAST-PASS.md. Requires a local Docker daemon + host
#    ports 8080/3000 free (override with SMOKE_GATEWAY_PORT / SMOKE_SPA_PORT).
bash deploy/smoke/run.sh             # exit 0 = stack came up + all checks green
```

**Read each artifact before building from it.** `cat deploy/Dockerfile.gateway`,
`deploy/Dockerfile.web`, `deploy/docker-compose.yml`, `deploy/nginx.conf` — the
conductor walks each line with you so you know what every container does.

**Pre-flight checklist — tick every box before the `up` in step 5:**
- [ ] `docker compose … config` renders cleanly (no env/syntax errors).
- [ ] `docker compose … build` succeeds for **both** images.
- [ ] `bash deploy/smoke/run.sh` exits 0 (the pre-deploy gate — stack comes up + all checks green; see `deploy/smoke/LAST-PASS.md` for the last recorded PASS).
- [ ] `nginx -t` reports the config is OK.
- [x] CONFIRMED against the binary (2026-06-01): `argo gateway start --bind 127.0.0.1 --port 7400`; provision via `argo tenant create <NAME> --json` then `argo demo seed --tenant <ID> [--skip-pass-a-turns]` (resolves refine-item #2).
- [ ] Security group exposes **only** 443 + 22 (your IP); `3000`/`7400` are NOT public.
- [ ] `deploy/.env.deploy` is filled + `chmod 600`; `deploy/.htpasswd` exists.
- [ ] DNS `A` record + TLS cert are in place (step 2).
- [ ] You understand what each runbook line does (the conductor has walked it with you).

Only when every box is ticked do we run the `up`.

---

## Prerequisites (what Mark provides)

1. An **AWS account** with the AWS credits applied.
2. A **domain** (e.g. `skycot.com`) with DNS you can edit (Route 53 or your
   registrar). Pick a subdomain for the gated demo, e.g. `try.skycot.com`.
3. The **secrets** to drop into the instance (never committed):
   - `OPENAI_API_KEY` — voice (STT/TTS + realtime) + any OpenAI provider use.
   - `ANTHROPIC_API_KEY` — the pitch/agent LLM path (or `ARGO_PITCH_LLM_PROVIDER=claude-code-cli` if billing a subscription, but API key is simpler on a server).
   - `SKYCOT_DB_KEY_BASE64` — 32-byte base64 DB key (generate with `openssl rand -base64 32`).
   - A **gate username + password** for nginx basic-auth.

---

## The `deploy/` kit (authored, in the repo)

| File | What it is |
|---|---|
| `deploy/Dockerfile.gateway` | Multi-stage Rust build of `argo`, **bundles the gateway's on-disk runtime deps** (pitch corpus, truth-boundary policies, provider manifests) + sets the `ARGO_*_DIR` overrides. Binds `127.0.0.1:7400`. |
| `deploy/Dockerfile.web` | Next.js **standalone** runtime image for the SPA (binds `127.0.0.1:3000`). |
| `deploy/docker-compose.yml` | gateway + web + nginx, **host networking** (so the gateway is reachable on the host loopback without binding non-loopback), persistent `/data` volume. |
| `deploy/nginx.conf` | TLS + **basic-auth gate** + path routing (`/api`, `/health`, `/inbox/ws` → gateway; everything else → SPA). |
| `deploy/.env.deploy.example` | Every env var + secret to set (copy to `.env.deploy`, fill, keep `chmod 600`). |

---

## Runbook (step-by-step)

> First pass targets **one instance** (Amazon Linux 2023, ARM `t4g.small`). Do
> it once by hand; we automate later if it's worth it.

**1. Launch the instance**
- EC2 → Amazon Linux 2023, `t4g.small` (2 vCPU / 2 GB; bump to `t4g.medium` if the Rust build OOMs — or build the image elsewhere and pull from ECR).
- Storage: 30 GB gp3 (the SQLite DB + the build cache live here).
- Security group: inbound **443** (HTTPS) + **22** (SSH, your IP only). **Not** 80→ (or 80 only to redirect to 443). **Never expose 3000/7400.**
- Elastic IP → associate, so the public IP is stable for DNS.

**2. DNS + TLS**
- Route 53 (or your registrar): `A` record `try.skycot.com` → the Elastic IP.
- TLS: easiest is **Let's Encrypt via certbot** on the instance (free, auto-renew). Alternatively an ALB + ACM cert in front (more AWS-native, ~$16/mo more).

**3. Install Docker + the kit**
```bash
sudo dnf install -y docker git && sudo systemctl enable --now docker
sudo usermod -aG docker ec2-user   # re-login after this
git clone <your repo> skycot && cd skycot
cp deploy/.env.deploy.example deploy/.env.deploy && chmod 600 deploy/.env.deploy
# edit deploy/.env.deploy — paste the 4 secrets + the gate user/pass
```

**4. Create the basic-auth gate file**
```bash
sudo dnf install -y httpd-tools
htpasswd -c deploy/.htpasswd <gate-username>   # prompts for the gate password
```

**5. Build + start** — *only after every box in the pre-flight checklist above is ticked.*
```bash
# Build is already validated if you ran the dry-run `build` (step 2 of the
# checklist). `up` then just STARTS the validated images — no `--build` here,
# so a surprise rebuild can't run unreviewed:
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.deploy up -d
# Confirm all three came up before going further:
docker compose -f deploy/docker-compose.yml ps           # expect gateway + web + nginx "Up"
docker compose -f deploy/docker-compose.yml logs -f gateway   # watch for the "listening" line, Ctrl-C when healthy
```

**6. Seed the demo data** (so the URL shows the polished demo, not an empty workspace)
```bash
# 6a. Provision the demo tenant, capturing its id from the JSON:
docker compose -f deploy/docker-compose.yml exec gateway \
  argo tenant create "SKYCOT Demo" --json        # → note the "id" field
# 6b. Seed that tenant with the canonical v1.0 demo data (workers / approvals /
#     skills / the Scene-5 workflow / audit rows). --skip-pass-a-turns needs NO
#     provider key (Pass B dressing + scaffolding still seed); for full real-LLM
#     Pass A turns instead, set ARGO_DEMO_SEED_PROVIDER_KEY and drop the flag.
docker compose -f deploy/docker-compose.yml exec gateway \
  argo demo seed --tenant <ID-FROM-6a> --skip-pass-a-turns
# ⚠️ The SPA's demo-mode must resolve to THIS tenant id (see refine-item #1).
```

**7. Verify (truth-bounded — actually check)**
```bash
curl -sk https://try.skycot.com/health            # expect {"status":"ok", ...}  (basic-auth will 401 first — use -u)
curl -sk -u <user>:<pass> https://try.skycot.com/api/v1/approvals | jq '.items | length'  # expect 14
# open https://try.skycot.com in a browser → basic-auth prompt → the seeded demo loads
```

**8. Hand out access:** give the gate username/password to the people you're
pointing at it. That's the gate.

---

## Gating options (pick one)

- **nginx basic-auth** (this kit) — simplest; one shared user/pass; instant. Good for a small investor/waitlist list.
- **Cloudflare Access / Tunnel** — nicer (per-email allowlist, no instance exposure), but adds Cloudflare in front.
- **AWS WAF IP allowlist** — if your audience has fixed IPs.

For "point a handful of investors at it," basic-auth is the right amount of gate.

---

## Secrets & data

- **Secrets:** for v1, a `chmod 600` `.env.deploy` on the instance is fine.
  Harden to **SSM Parameter Store / Secrets Manager** (the compose `env_file`
  becomes a fetch-on-boot) when it matters. Never commit secrets.
- **Data persistence:** the gateway's SQLite (`$ARGO_HOME` → `/data`) lives on
  the EBS volume → survives container restarts + redeploys. Back it up with an
  EBS snapshot before any risky change. (Postgres/RDS is the scale-later path.)

---

## Cost estimate vs the AWS credits

| Item | ~Monthly |
|---|---|
| `t4g.small` EC2 (on-demand) | ~$12 |
| 30 GB gp3 EBS | ~$2.50 |
| Elastic IP (in use) | $0 |
| Route 53 hosted zone | $0.50 |
| Data transfer (low, gated) | ~$1–3 |
| **Total** | **~$16–18/mo** |

→ The AWS credits cover **~3+ years** of a single gated instance. (An ALB +
ACM would add ~$16/mo if you prefer it over certbot.) Plenty of runway.

---

## Known refine-during-deploy items (honest)

These are real and small, to nail when the instance exists (don't pretend
they're solved):

1. **Demo-tenant resolution on a server path.** The demo tenant is resolved
   from the workspace path; in a container the path differs, so the seed +
   the SPA's demo-mode must agree on the tenant. Verify the seed targets the
   tenant the SPA resolves (a one-row workspace re-pin may be needed — same
   fix used locally).
2. **Exact seed subcommand — ✅ RESOLVED 2026-06-01.** Confirmed against the
   built binary (`argo demo seed --help`): provision with `argo tenant create
   <NAME> --json` (capture the `id`), then `argo demo seed --tenant <ID>
   [--skip-pass-a-turns]`. Profile defaults to `v1-demo-canonical`;
   `--skip-pass-a-turns` seeds Pass B + scaffolding with **no** provider key.
   Gateway start confirmed: `argo gateway start --bind 127.0.0.1 --port 7400`.
3. **Gateway runtime deps completeness.** The Dockerfile bundles corpus +
   policies + manifests; if any other `workspace_root()` read is missing at
   boot, the gateway log will name the path — add it to the COPY + an
   `ARGO_*_DIR` override.
4. **sqlcipher vs plaintext SQLite.** If the release build links sqlcipher, the
   runtime image needs `libsqlcipher`; if not, plaintext + the DB key is a
   no-op. Confirm from the build features.
5. **WebSocket gating.** `/inbox/ws` is proxied with upgrade headers in
   `nginx.conf`; confirm the live approvals socket connects through the gate
   (the SPA's ws hook must build a *same-origin* `wss://<host>/inbox/ws`, not an
   absolute `127.0.0.1` URL — verify in the browser network tab).
6. **SPA shows the demo, not a login wall.** `isAuthBypassed()` is FALSE in a
   production build (`NODE_ENV!=="production"`-gated), so the deployed SPA shows
   the seeded demo via **demo-mode** (`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` +
   `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`, baked as compose build-args), which makes it
   fetch real gateway data. Verify the app pages render behind the nginx gate
   without requiring a magic-link session; if any page hard-gates on a session,
   that's the one thing to patch for the gated demo.

---

## Scale-later path (deferred — when onboarding real users)

When the gated demo becomes a real product with real users:

- **Gateway** → ECS Fargate or App Runner (bind non-loopback → **production-auth
  mode on**), with `ARGO_GATEWAY_BEARER_TOKEN` set and wired through the SPA's
  server-side proxy; real **magic-link auth** for users (needs an email sender —
  SES).
- **SPA** → AWS Amplify Hosting (native Next.js SSR) or keep the container.
- **DB** → migrate SQLite → **RDS Postgres** (horizontal scale; the architecture
  anticipates this — it's not built yet).
- **Perimeter** → ALB + ACM + AWS WAF + per-tenant rate limiting.
- Multi-AZ, autoscaling, the works — only when the load justifies it.

> Don't build any of this for the gated demo. The single instance is correct
> until you have real users.
