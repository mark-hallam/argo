# SKYCOT / Argo — AWS deploy-readiness audit (2026-06-08)

> **What this is:** an honest, read-only assessment of whether the code on
> `main` (HEAD `a7e702e58`) can be deployed to AWS today, and exactly what to
> do / decide / fix first. Written for a non-technical reader: every technical
> term is expanded the first time it appears.
>
> **Method:** I read the actual source files and infrastructure config. Each
> claim is tagged **VERIFIED** (I read the code/file that proves it) or
> **ASSUMED** (reasoned, not directly proven). Nothing here has been run
> against a live AWS account — the whole platform has never been deployed to
> AWS, so *every* "it will work" claim is "verified-from-code, not
> verified-in-the-cloud" until you stand up the first instance.

Glossary of the unavoidable jargon, once:
- **Gateway** = the Rust (a fast, safe programming language) server program (the
  `argo` binary) that does all the real work and holds the database.
- **SPA** (Single-Page Application) = the website front-end (built with Next.js,
  a popular website framework). What a user sees in their browser.
- **Container / Docker** = a sealed box that holds an app + everything it needs
  to run, so it runs identically on your laptop and on a server.
- **AWS** (Amazon Web Services) = Amazon's cloud — you rent servers, storage,
  databases, networking from them.
- **ECS Fargate** (Elastic Container Service, Fargate mode) = Amazon's way of
  running a container without you managing the underlying server.
- **EC2** (Elastic Compute Cloud) = a plain rented virtual server (you manage it
  yourself).
- **Terraform / IaC** (Infrastructure as Code) = text files that describe the
  cloud setup, so it's built reproducibly by a tool instead of by hand-clicking.
- **Env var** (environment variable) = a named setting passed to a program at
  startup, e.g. a secret key.
- **TLS / HTTPS** = the padlock — encrypted, trusted connection between browser
  and server.
- **JWT** (JSON Web Token) = a signed digital ID card the server issues; the
  browser presents it to prove who it is.
- **SQLite / SQLCipher** = the database is a single file on disk (SQLite);
  SQLCipher is the encrypted-at-rest version of it.
- **Tenant** = one customer's isolated workspace + data.

---

## 1. Bottom line — can we deploy, and to what?

**Yes — the platform is deployable to AWS today, and the deploy machinery is
unusually complete: a full, code-grounded, operator-runnable AWS setup already
exists in the repo.** The security perimeter that protects your core promise is
real and **fails closed** (VERIFIED from code): with the one dev switch left
off, the gateway refuses to boot in production without its security keys, an
anonymous request to a protected page is rejected (401 = "not authenticated"),
and a signed-in customer reads only their own tenant's data (already
browser-verified this session). The two dangerous flip-flags
(`ARGO_DUTY_AUTOFIRE`, `ARGO_EXECUTION_GRADUATION`) are **default-OFF in code**
(VERIFIED) — you don't have to do anything to keep them off; they only turn on
if someone explicitly sets them.

**The honest caveat:** "deployable" means *"to a private/invite-gated URL you
can confidently put in front of investors and design partners."* It is NOT yet
*"open the floodgates to anonymous strangers and walk away."* Two real
guardrails are still missing for a fully-public, anyone-can-sign-up launch: (a)
the **Free tier is only *softly* kept on the cheap mock AI** (no hard tier lock
yet), and (b) there is **no hard dollar ceiling on AI spend for anonymous
visitors** — today only the request *rate* is capped, not the cumulative cost.
Neither blocks a gated/invite launch; both should land before a no-limits public
launch. Details in §5.

One scope note you should know: there are **two deploy plans** in the repo
written at different times (§2). The **newer, recommended one is the ECS Fargate
plan** (`AWS_DEPLOY_RUNBOOK.md`, dated 2026-06-05) — it matches your current
"fully-public self-serve product" direction (decision DEC-034). The **older
single-EC2 "gated demo" plan** (`DEPLOYMENT.md`, dated 2026-06-01) is simpler
and cheaper but was designed to show *seeded demo data behind a password*, not
to onboard real paying users. Don't mix them up.

---

## 2. The two documented deploy paths

The repo contains a sprawling `deploy/` folder with many cloud targets
(Kubernetes, Fly.io, Vercel, a plain VM, Cloudflare WAF, etc.). Ignore the
noise — **two coherent, documented paths matter:**

### Path A — ECS Fargate, full production (RECOMMENDED) — VERIFIED present
**File:** [`docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md`](deploy/AWS_DEPLOY_RUNBOOK.md)
(the operative one) + the Terraform code under
[`infra/terraform/`](../../infra/terraform/).

**What it is, plainly:** Amazon runs your gateway container for you (Fargate),
behind an Amazon **load balancer** (ALB — Application Load Balancer, the front
door that terminates HTTPS and forwards traffic). The database file lives on
**EFS** (Elastic File System — a network disk that survives the container
restarting). Secrets live in **AWS Secrets Manager** (a vault). The website
(SPA) is hosted separately on **Vercel** (the company that makes Next.js;
purpose-built for hosting it). DNS (Domain Name System — the phone book that
maps `skycot.com` to a server) is on **Cloudflare**.

It is described as ONE ordered sequence of 14 steps. The Terraform builds **8
modules** (network, container registry, encrypted file system, key management,
secrets vault, permissions, load balancer, the container service itself, plus
monitoring alarms). **VERIFIED:** the runbook is exhaustive, cites the exact
file behind every step, and the IaC passes local validation
(`scripts/validate-infra.sh`).

**Why it's recommended:** it matches the "real product with real signups + Stripe
billing" direction you locked in DEC-034. It binds a public address → the
gateway automatically enters **production mode** (the locked-down, password-
required mode), which is what you want for real customers.

**The concrete step list** (condensed from the runbook — the runbook itself
walks each command with success signals):
1. **Prereqs (one-off):** install the AWS command-line tool + Terraform +
   Docker; log in; get a Cloudflare API token for `skycot.com`.
2. **Bootstrap remote state:** `cd infra/terraform-bootstrap && terraform
   apply` — creates the small storage that Terraform uses to remember what it
   built.
3. **Wire the backend + review the plan:** create `backend.tf` from step-2
   output, then `terraform plan` in `infra/terraform` and eyeball that exactly
   the 8 expected modules appear (stop if anything else does).
4. **Fill the 7 production secrets** into Secrets Manager (the keys in §3) —
   Terraform only creates `REPLACE_ME` placeholders; you paste the real values.
5. **`terraform apply`** — builds the whole cloud stack (~10 min).
6. **Build + push the gateway image, roll the service:** `docker build
   --platform linux/amd64 ...` then push to Amazon's registry and tell the
   service to use it. (Apple-Silicon Macs MUST pass `--platform linux/amd64` or
   the container won't run.)
7. **Validate the HTTPS certificate** by pasting a DNS record into Cloudflare.
8. **Point `api.skycot.com` at the load balancer** (DNS cutover).
9. *(Optional, later)* **Lock the origin behind Cloudflare** so nobody can
   bypass it — turns on the per-IP rate limiter at the same time.
10. **Confirm the monitoring alarms** + (if you set an alert email) click the
    confirmation link AWS sends you.
11. **Deploy the SPA on Vercel** (import the repo, root dir = `web`, set the
    public env vars in §3).
12. **Run the pre-deploy validation gate** (`scripts/validate-infra.sh`).
13. **Smoke-test:** hit `/health/ready` (expect 200 = healthy), hit
    `/api/v1/team` with no login (expect 401 = correctly locked), hit the
    billing webhook with a bad signature (expect 400 = correctly rejecting).
14. *(If needed)* rollback / teardown commands.

**Cost:** higher than Path B (load balancer + NAT gateway + Fargate are the main
line items, on the order of low-tens-of-dollars/month minimum, more under load)
— but comfortably inside your ~$800 of AWS credits for a long runway. (ASSUMED —
the runbook doesn't give a single monthly total the way Path B does.)

### Path B — single EC2 + nginx + Docker Compose, *gated demo* — VERIFIED present
**File:** [`docs/skycot/operator/DEPLOYMENT.md`](operator/DEPLOYMENT.md) + the
kit under [`deploy/`](../../deploy/) (`docker-compose.yml`, `nginx.conf`, etc.).

**What it is, plainly:** ONE small rented server runs three containers (the
gateway, the SPA, and **nginx** — a web server that sits in front, does HTTPS,
and adds a simple shared-password gate). The gateway deliberately stays on
"loopback" (`127.0.0.1`, meaning "only reachable from inside this one machine"),
so it stays in **demo mode** and shows pre-seeded demo data with no login wall;
nginx's password is the only gate. (**nginx** = a popular, lightweight web
server / reverse proxy.)

**What it's for:** pointing a handful of investors at polished *demo* data behind
one shared password. **~$16-18/month** (VERIFIED — the doc gives this figure),
so the AWS credits cover 3+ years.

**Recommendation:** Use **Path A** for the direction you've chosen (real signups
+ billing). Path B is a faster, cheaper fallback *only if* all you want this
week is a password-gated showcase of demo data, not real customer accounts.

> **Decision-record context (VERIFIED):** DEC-032 (2026-06-01) authored Path B
> (gated demo). DEC-034 (2026-06-04) then changed the goal to "fully-public
> self-serve SaaS + enterprise tier" and folded deployment into sprint
> workstream WS5. The newer ECS runbook (Path A, 2026-06-05) is the artifact
> that matches the *current* goal. So: **DEC-034 is the live intent; Path A is
> its deploy plan; Path B / DEC-032 are the earlier, simpler fallback.**

---

## 3. Required production config + secrets

The single source of truth for this is
[`deploy/secrets/skycot-production.env.example`](../../deploy/secrets/skycot-production.env.example)
— a fully code-cited list (VERIFIED: I cross-checked the load-bearing ones
against the crates that read them).

### Secrets the GATEWAY must have (set in AWS Secrets Manager for Path A)

| Setting | Plain-English what + why | Security weight | Source proof |
|---|---|---|---|
| `ARGO_GATEWAY_PROFILE=production` | A switch that forces the locked-down mode on. Not secret, but **load-bearing.** | Turns the whole perimeter on | VERIFIED `routes_auth_production_mode.rs` |
| `ARGO_GATEWAY_BEARER_TOKEN` | The admin password for the gateway when it's publicly reachable. **Without it the gateway refuses to boot in production.** Generate with `openssl rand -hex 32`. | HIGH — admin access | VERIFIED `boot.rs` bearer guard |
| `SKYCOT_AUTH_JWT_SECRET` | The secret that **signs the session login cookie** (the digital ID card). If unset/short, the gateway refuses to boot in production; if it were silently empty, login would be silently broken. Min 32 bytes. | **HIGH — this is what proves a user is who they say.** Whoever holds it could forge logins. | VERIFIED `startup_check.rs` `jwt_secret_boot_guard` |
| `SKYCOT_DB_KEY_BASE64` | The key that **encrypts the database file at rest** (SQLCipher). In production the gateway refuses to boot without it (and without a SQLCipher-built image). Generate with `openssl rand -base64 32`. | **HIGH — encrypts all stored data.** | VERIFIED `store_open.rs` `enforce_enterprise_encryption` |
| `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` (a.k.a. `ARGO_CREDENTIALS_MASTER_KEY`) | The key that **wraps each tenant's saved secrets** (their own AI keys, channel tokens). Without it, saving a provider key fails (503 error). | HIGH — protects per-customer secrets | VERIFIED (cited in secrets file; `argo-credentials/src/master_key.rs`) |
| `STRIPE_SECRET_KEY` + `STRIPE_WEBHOOK_SECRET` | Billing — your Stripe account's secret + the signature key that proves a payment webhook really came from Stripe. Required for paid tiers. | HIGH — money | VERIFIED (`argo-billing/src/stripe_client.rs`) |
| `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` (platform's OWN keys) | The platform's keys for Genesis (the AI that builds teammates), voice, and the pitch agent. **Optional** in the AWS module (no Secrets-Manager slot created) — per-customer keys are stored encrypted at runtime instead. | MEDIUM — these spend YOUR money on shared AI features | VERIFIED secrets file |

> **Good news on a subtle trap (VERIFIED):** the encryption gate checks a config
> field called `key_source`, which defaults to `"none"`. But the boot code
> (`boot.rs` `effective_key_source`) treats "`SKYCOT_DB_KEY_BASE64` is set" as
> equivalent to a real key source. **So just setting the env var is enough — you
> do NOT also need to edit an `argo.toml` config file.** The ECS task already
> injects `SKYCOT_DB_KEY_BASE64` (VERIFIED in `infra/terraform/modules/ecs/main.tf`).

### Public config the SPA needs (set in Vercel — these ship to the browser, NOT secret)
- `NEXT_PUBLIC_GATEWAY_URL = https://api.skycot.com` — where the website talks to
  the gateway. (VERIFIED `web/lib/config/gateway-url.ts`.)
- `NEXT_PUBLIC_STRIPE_PRICE_PRO` (+ `…_ENTERPRISE`) — the Stripe price IDs the
  pay buttons use. **Must be set or the Pro/Enterprise buttons fail.** (VERIFIED.)
- `NEXTAUTH_URL` + `NEXTAUTH_SECRET` — settings for the login library.

### Settings that MUST be off / unset for production safety — VERIFIED default-safe
| Flag | What it does if ON | Default | How prod stays safe |
|---|---|---|---|
| `ARGO_GATEWAY_AUTH_BYPASS` | **Opens the perimeter** — anyone can read anything, no login. The ONE dev/demo switch. | OFF (only a `=1` opens it) | Leave it unset. Production mode also fights it. VERIFIED `routes_auth.rs`, `routes_auth_production_mode.rs` |
| `ARGO_DEMO_MODE` | Mounts a "mint a fake login" route for recording demos. | OFF | The gateway **refuses to boot** if `ARGO_DEMO_MODE=1` without the bypass also on — so it can't be shipped to prod by accident. VERIFIED `startup_check.rs` `demo_mode_compatibility_check` |
| `ARGO_DUTY_AUTOFIRE` | Lets scheduled teammate tasks **auto-approve + fire themselves** without a human OK. | **OFF** | Default-closed in code; the scheduler boots with a "closed gate". VERIFIED `boot_spawns.rs` (`gate=closed` unless flag truthy) |
| `ARGO_EXECUTION_GRADUATION` | Lets a teammate **graduate to doing real actions** automatically. | **OFF** | Default-OFF; the whole step is dormant unless set. VERIFIED `routes_workers_hire_safe_tier_graduation.rs`, `worker_actions_dispatch.rs` |

**How secrets are loaded:** for Path A, **AWS Secrets Manager** (the IaC creates
the 7 secret slots; you paste real values once). For Path B / local dev, a
`chmod 600` (owner-only-readable) `.env.deploy` file on the box, or
`~/.argo/env.local`. (VERIFIED in the runbook + secrets README.)

---

## 4. Readiness checklist

### ✅ READY — VERIFIED in code/files this audit
- ✅ **The gateway builds as one self-contained container** with the encrypted
  database support compiled in by default (`CARGO_FEATURES=bundled-sqlcipher`).
  *Evidence: root `Dockerfile` line 20; multi-stage Rust build → tiny
  "distroless" runtime image.* (VERIFIED)
- ✅ **`.dockerignore` keeps junk out of the image** (no `target/`,
  `node_modules`, `.git`, `*.db`, `.env`). *Evidence: `.dockerignore`.* (VERIFIED)
- ✅ **The website builds for production** as a standalone Next.js output and
  reaches the gateway via a same-origin `/api/v1/*` rewrite — **so there is no
  CORS** (Cross-Origin Resource Sharing — the browser rule about talking to a
  different web address; not an issue here because browser→website and
  website→gateway are treated as one origin). *Evidence: `web/next.config.ts`
  (`output: "standalone"`, the `rewrites()` block, the comment "In production,
  SPA and gateway are colocated behind a single load balancer so this rewrite is
  a no-op").* (VERIFIED)
- ✅ **The perimeter fails CLOSED by identity.** With the bypass off, an
  anonymous hit on a protected route returns 401. *Evidence: `routes_auth.rs`
  `require_identity` + the `88fa18c34` WS5 commit.* (VERIFIED — and live-verified
  in browser this session per the brief.)
- ✅ **A signed-in customer reads only their own tenant.** The per-request tenant
  is taken from the login cookie's `tnt` claim, not the server's working
  directory. *Evidence: `resolve_tenant.rs` `resolve_tenant_for_request`.*
  (VERIFIED — and live-verified across team/workers/skills/audit this session.)
- ✅ **Production gateway won't boot misconfigured.** Three boot guards refuse to
  start without: the login-signing secret, a real DB encryption key + encrypted
  build, and (when publicly bound) the admin bearer token — and it refuses a
  bare public bind with no TLS-proxy contract. *Evidence: `startup_check.rs`,
  `store_open.rs`, `boot.rs`, `proxy_guard.rs`.* (VERIFIED)
- ✅ **The two dangerous flip-flags are default-OFF** (see §3 table). (VERIFIED)
- ✅ **Database migrations are additive + guarded against collisions; current
  schema version = 91.** *Evidence: `EXPECTED_SCHEMA_VERSION = 91` in
  `argo-store/src/lib.rs`; `scripts/check-migration-slots.sh` +
  `migrations/registry.rs` `validate_no_duplicate_slots()`.* (VERIFIED)
- ✅ **Single-writer database is respected in the IaC:** the container service is
  pinned to exactly one running copy (`desired_count 1`, old task stopped before
  new starts) because two writers would corrupt the one SQLite file. *Evidence:
  `AWS_DEPLOY_RUNBOOK.md` step 6 "Single-writer by design"; ECS module.*
  (VERIFIED)
- ✅ **Browser security headers + a Content-Security-Policy are set** (HTTPS-
  forcing, anti-clickjacking, restricted connections). *Evidence:
  `web/next.config.ts` `headers()`.* (VERIFIED)
- ✅ **Layered abuse protection partly LIVE:** a per-IP rate limiter is on by
  default (≈120 burst / 2-per-second per source IP) plus a tighter signup
  limiter; per-tenant + per-route limits are config-driven. *Evidence:
  `abuse-and-cost-controls.md` citing `boot_serve.rs:379`,
  `sections_rate_limit.rs`.* (VERIFIED)
- ✅ **A complete, operator-runnable AWS plan + Terraform exists and locally
  validates.** *Evidence: `AWS_DEPLOY_RUNBOOK.md` + `infra/terraform/` (8
  modules) + `scripts/validate-infra.sh`.* (VERIFIED authored & validated;
  never applied to AWS.)
- ✅ **The SPA-login path is wired correctly (point #6 — confirmed
  VERIFIED-FROM-CODE).** On magic-link verify the gateway sets a `__Host-session`
  cookie carrying the user's tenant, with `Secure; HttpOnly; Path=/` and **no
  Domain** (the exact attributes the `__Host-` prefix demands), and first-time
  users auto-get their own tenant. The Next.js proxy forwards it. *Evidence:
  `auth_magic_link_verify.rs:140`; the Vercel/HTTPS domain satisfies `Secure`.*
  (VERIFIED — see the note in §6.)

### ☐ MUST-DO before you deploy (operator actions)
- ☐ **Generate + store the 6-7 production secrets** in AWS Secrets Manager (§3).
  Without the bearer token + DB key + JWT secret, the gateway will not boot.
- ☐ **Build the image on a `linux/amd64` platform** (`--platform linux/amd64` on
  your Apple-Silicon Mac) or the container crashes on Amazon's x86 servers.
- ☐ **Create the Stripe products/prices** and put the price IDs in Vercel
  (`NEXT_PUBLIC_STRIPE_PRICE_PRO` / `…_ENTERPRISE`), or pay buttons fail.
- ☐ **Validate the HTTPS certificate** (paste the DNS record into Cloudflare,
  step 7) and **point `api.skycot.com` at the load balancer** (step 8).
- ☐ **Set the alert email and click AWS's confirmation link**, or monitoring
  alarms fire into the void. (VERIFIED this is a manual click in step 10.)
- ☐ **Run the smoke test** (step 13): `/health/ready`→200, `/api/v1/team`→401,
  bad-signature webhook→400. This is your proof the perimeter is actually on.
- ☐ **Do a live cold-signup test** of point #6 end-to-end on the real HTTPS
  domain (a stranger signs up → verifies → lands in their OWN empty workspace →
  reads no demo/other-tenant data). The code is verified; the live HTTPS round-
  trip of the `__Host-` cookie through Vercel + Cloudflare is the one thing only
  a live test can confirm (see §6).

### ❓ DECIDE before you deploy
- ❓ **Which path — A (ECS Fargate, real signups) or B (single EC2, gated demo)?**
  Recommendation: **A**, to match DEC-034. (B only if you want a cheap password-
  gated demo this week.)
- ❓ **Gated/invite launch vs fully-public launch.** If gated (recommended
  first), you can ship now and accept the two soft gaps in §5. If fully-public
  with anonymous strangers, land the two §5 items first (cost protection).
- ❓ **Backup policy for the database.** The IaC turns on the EFS default backup
  policy, but **application-level SQLite snapshots are "TBD"** (the runbook says
  so). Decide: rely on EFS backup for v1, or add a periodic DB-file snapshot.
- ❓ **Turn on the Cloudflare origin lockdown + WAF now or later** (step 9 +
  `deploy/waf/`). It's authored but default-off and optional.

---

## 5. Gaps — with severity

### Blockers for a FULLY-PUBLIC (anonymous-stranger) launch — but NOT for a gated/invite launch
- **🟥 No hard dollar ceiling on AI spend for anonymous visitors.** Today only
  the request *rate* is capped (so a bot can't hammer you fast), but there is **no
  cumulative dollar cap** — the Genesis "build a teammate" flow calls *your* AI
  keys with no per-visitor spend limit. A determined abuser within the rate
  limit could still run up a bill. *Evidence (VERIFIED): `abuse-and-cost-
  controls.md` §6 — `argo-genesis/src/author.rs` has zero budget/cost
  references; tracked as the unbuilt brief `sprint-ws5-4b-anon-llm-spend-
  ceiling-backend`.* **For a gated/invite audience this is acceptable** (you know
  who's in); for open public signup it should land first.
- **🟥 Free tier is only *softly* kept on the cheap mock AI.** Requests default to
  the free "mock" provider, but there is **no hard tier lock** — a Free user who
  saves a real AI key and explicitly asks for it is **not** blocked by tier
  logic. *Evidence (VERIFIED): `abuse-and-cost-controls.md` §5 — the
  `provider_real_keys` tier flag "is never checked at runtime"; the hard gate is
  the unbuilt brief `sprint-ws4-enforce-tier-capability-gates`.* So **a trial
  user is NOT stuck on mock AI if they bring their own key** — which is fine
  (it's their spend), but it means "Free = mock" is a default, not a guarantee.

### Important (do soon, not a hard blocker)
- **🟧 Never actually run on AWS.** Every "it works" is verified-from-code, not
  verified-in-the-cloud. The first deploy is where real-world surprises (the
  HTTPS cookie round-trip, the SQLCipher boot gate, a missing on-disk file the
  gateway reads at boot) would surface. The runbook's own "known refine-during-
  deploy" items acknowledge this. *(VERIFIED — explicit truth-bounded notices in
  both runbooks.)*
- **🟧 App-level database backups are "TBD".** EFS filesystem backup is on, but a
  clean point-in-time SQLite snapshot story isn't built. Low risk at low volume,
  but decide before real customer data lands. *(VERIFIED — `AWS_DEPLOY_RUNBOOK.md`
  "What this runbook does NOT cover".)*
- **🟧 Cloudflare origin-lockdown + WAF default-OFF.** Until step 9 is run, the
  load balancer is directly reachable (not hidden behind Cloudflare). Fine for
  bring-up; tighten before wide exposure. *(VERIFIED — step 9 is default-off.)*

### Nice-to-have (later)
- **🟩 No Postgres / horizontal scale.** It's single-server SQLite by design for
  v1; the runbook says Postgres is out of scope "until SQLite becomes a measured
  bottleneck." Not a launch blocker — just a known ceiling. *(VERIFIED — no
  Postgres in the gateway binary; the Kubernetes Postgres YAML in `deploy/k8s/`
  is unwired aspiration.)*
- **🟩 Secret-rotation automation** and a few **partial older env templates**
  (`deploy/.env.deploy.example`, the Fly.io template with a wrong key name) are
  superseded by the canonical `skycot-production.env.example`. Use the canonical
  one. *(VERIFIED.)*

---

## 6. The SPA-login gap (#159) — resolved as VERIFIED-FROM-CODE (with one live test to run)

**The concern:** the website sends BOTH a login cookie AND an `Authorization:
Bearer` header. The bearer header does **not** carry a per-user identity the
gateway trusts — the gateway only accepts a bearer that matches the *static* admin
token. So does a real signed-in user actually authenticate in production?

**Answer (VERIFIED from code): YES — via the cookie, not the bearer.**
1. On magic-link verify, the gateway issues `Set-Cookie:
   __Host-session=<JWT>; Max-Age=…; Path=/; HttpOnly; Secure; SameSite=Lax`,
   and that JWT carries the user's tenant (`tnt`). First-time users get their own
   tenant auto-provisioned right here. *(VERIFIED `auth_magic_link_verify.rs`.)*
2. The browser automatically attaches that cookie to same-origin `/api/v1/*`
   requests; the Next.js proxy forwards the request (cookie included) to the
   gateway. *(VERIFIED `web/next.config.ts` rewrite + standard same-origin cookie
   behavior.)*
3. The gateway's `require_identity` accepts a valid `__Host-session` cookie as
   proof of identity (it doesn't need the bearer). *(VERIFIED `routes_auth.rs`.)*
4. The per-request resolver then reads the tenant from that same cookie, so the
   user reads only their own data. *(VERIFIED `resolve_tenant.rs`.)*

So the bearer header is a harmless belt-and-braces extra (the website actually
puts the cookie's own JWT into it too, via `readMagicLinkClaims`), and the
**cookie is the real, working auth path.**

**The `__Host-` prefix concern (VERIFIED safe):** browsers only accept a
`__Host-`-prefixed cookie if it is `Secure`, `Path=/`, and has **no `Domain`
attribute** — exactly how the gateway sets it (no `Domain` is present). On a real
**HTTPS** production domain (`app.skycot.com`), all three hold, so nothing blocks
the cookie. *(It would NOT work over plain `http://` — but production is HTTPS,
so this is fine.)*

**The one thing only a live test can confirm (NEEDS-LIVE-TEST):** that the cookie
survives the full real-world round-trip — browser → Vercel (the website host) →
the Next proxy → Cloudflare → the load balancer → the gateway — on the actual
HTTPS domains, and comes back correctly on the next request. Every individual
link is verified in code; the *assembled* chain on live infrastructure is the
last unproven inch. **This is exactly the "cold stranger signup" smoke test in
§4** — run it as the first acceptance check after deploy.

---

### Appendix — key files I read (all paths absolute-relative to repo root)
- Build: `Dockerfile`, `.dockerignore`, `web/next.config.ts`
- Deploy plans: `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` (Path A, operative),
  `docs/skycot/operator/DEPLOYMENT.md` (Path B), `infra/terraform/` (8 modules),
  `docs/skycot/deploy/abuse-and-cost-controls.md`,
  `docs/skycot/deploy/consolidated-single-app-domain-attach.md`
- Secrets: `deploy/secrets/skycot-production.env.example`
- Perimeter/boot: `crates/argo-cli/src/handlers/gateway/{startup_check,boot,
  proxy_guard,routes_auth,routes_auth_production_mode,resolve_tenant,
  auth_magic_link_verify,boot_spawns}.rs`, `crates/argo-store/src/store_open.rs`
- SPA auth: `web/lib/api/client.ts`, `web/middleware.ts`, `web/auth.ts`,
  `web/lib/auth/session-cookie.ts`, `web/app/api/auth/verify/route.ts`
- Decisions: `docs/skycot/decisions/DEC-032`, `DEC-034`
