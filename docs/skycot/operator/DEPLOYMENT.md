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

**5. Build + start**
```bash
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.deploy up -d --build
# the gateway build is heavy (~10-15 min first time). Watch: docker compose logs -f gateway
```

**6. Seed the demo data** (so the URL shows the polished demo, not an empty workspace)
```bash
# one-off, idempotent — seeds the demo tenant's workers/approvals/notifications/Portia
docker compose -f deploy/docker-compose.yml exec gateway argo demo seed   # exact subcommand: see "Known refine items"
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
2. **Exact seed subcommand.** Confirm the `argo` seed CLI invocation (`argo
   demo seed` / the `argo-demo-seed` entry) against the built binary's `--help`.
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
