# SKYCOT — Production runbook (paying-customer shape, AWS ECS)

> **Phase:** Paying-ready launch (DEC-034) — the step AFTER the gated demo.
> **Goal:** the **public, self-serve** deployment shape: real users sign up,
> verify by magic-link email, pay via Stripe, and run real workers — on AWS ECS
> Fargate behind an ALB, fronted by Cloudflare.
> **Division of labour:** Mark (operator) owns the AWS account, billing, DNS,
> secret VALUES, and every `terraform apply` / `aws` mutation. The agent owns the
> repo artifacts this runbook points at (Dockerfile, IaC, env templates, smoke).
> **Truth-bounded ([DEC-029](../decisions/DEC-029-truth-bounded-status-reporting.md)):**
> every artifact below is *prepped + locally sound*; **the live AWS deploy is
> UNVERIFIED until the instance exists.** Where a step has never been run against
> real AWS, it says so — do not read "prepped" as "proven in production."

---

## How this differs from `DEPLOYMENT.md`

[`DEPLOYMENT.md`](DEPLOYMENT.md) is the **GATED-DEMO** runbook: a single EC2 box,
the gateway on **loopback** behind **nginx basic-auth**, demo-bypass on, a seeded
demo tenant. It is the "point an investor at a URL" shape, NOT a product.

This runbook is the **PAYING-CUSTOMER** shape and inverts the load-bearing
decisions:

| | `DEPLOYMENT.md` (gated demo) | this runbook (paying customers) |
|---|---|---|
| Gateway bind | loopback `127.0.0.1:7400` | **non-loopback** (ALB → container `:8080`) |
| Auth | nginx basic-auth gate, demo-bypass on | **production-auth mode** (bearer + JWT), magic-link signup |
| Compute | one EC2 + docker-compose | **ECS Fargate** via `infra/terraform` |
| Email | none (demo) | **SES/SMTP** for magic-link delivery |
| Payments | none | **Stripe** checkout + webhook |
| Store | SQLite on EBS | SQLCipher on **EFS** (RDS Postgres still deferred) |
| Perimeter | nginx on the box | **ALB + ACM TLS + Cloudflare** |

The load-bearing flip: **the gateway enters production-auth mode the moment it
binds a non-loopback address** (`crates/argo-cli/src/handlers/gateway/routes_auth_production_mode.rs`).
On ECS the container CMD binds `0.0.0.0:8080`, so production mode is ON and the
demo-bypass paths are unreachable. That is the whole point.

---

## Architecture

```
   Internet (HTTPS, api.skycot.com)
        │
   Cloudflare (proxy + WAF + DNS)         ← operator-managed, out of band
        │  (ALB SG accepts 80/443 only from Cloudflare ranges when
        │   front_with_cloudflare=true; gateway gets ARGO_TRUST_XFF=1)
   ┌────▼─────┐  ACM TLS
   │   ALB    │  (modules/alb)
   └────┬─────┘
   ┌────▼───────────────┐  ECS Fargate service, 1 task (modules/ecs)
   │  argo gateway      │  container :8080, production-auth mode
   │  (root Dockerfile) │  reads policies/ + skill-library/ from the IMAGE
   └────┬───────────────┘  secrets from Secrets Manager (modules/secrets)
        │  SQLCipher store @ /data  ← EFS (modules/efs), transit-encrypted
   CloudWatch Logs + alarms (modules/observability)
```

The whole stack is one Terraform root: [`infra/terraform`](../../../infra/terraform)
(`main.tf` + `modules/{network,alb,ecr,ecs,efs,iam,kms,secrets,observability}`),
default region `ap-southeast-2` (Sydney), default host `api.skycot.com`.

---

## Prerequisites (operator-runs)

1. AWS account + billing alarm; CLI authed (`aws sts get-caller-identity`).
2. Terraform ≥ 1.6 (`infra/terraform/versions.tf` pins providers).
3. A domain on Cloudflare (DNS managed out of band; `front_with_cloudflare`
   starts `false` for bring-up, flip to `true` once DNS proxies through CF).
4. Docker (to build + push the production image to ECR).
5. The secret VALUES ready (generated fresh — never reused from the demo):
   bearer token, workspace-root/credentials master key, JWT secret, SQLCipher DB
   key, Stripe secret + webhook secret, and (for email) the SES SMTP password.

---

## Runbook (step-by-step)

Each step is labelled:
**[operator-runs]** the operator executes against AWS · **[agent-prepped]** the
repo artifact exists + is locally sound, operator runs it · **[agent-verified]**
the agent proved this on the real binary this build (cited).

### 1. Provision the secret VALUES — [operator-runs]

The IaC creates the Secrets Manager *containers* (`modules/secrets`, keys:
`bearer_token`, `workspace_root_key`, `auth_jwt_secret`, `db_key`,
`stripe_secret_key`, `stripe_webhook_secret`, and optionally `smtp_password`).
The operator puts the real VALUES in — IaC never holds a secret value.

```bash
# Generate fresh secrets (examples — keep these OUT of git/shell history)
openssl rand -base64 32   # db_key (SQLCipher) AND workspace_root_key (use two)
openssl rand -hex 32      # bearer_token, auth_jwt_secret
# Stripe secret/webhook come from the Stripe dashboard.
# Put each into its Secrets Manager secret created by `modules/secrets`.
```

> NO THEATER: this has never been run against a real Secrets Manager from this
> repo. The key→env wiring is verified in code (`modules/ecs/main.tf`
> `secret_env_bindings`); the live put is operator-first-run.

### 2. Build + push the production image — [agent-prepped]

The **root [`Dockerfile`](../../../Dockerfile)** is the production/ECS image (NOT
`deploy/Dockerfile.gateway`, which is the loopback demo image). It is kept at
PARITY with the gateway image: same binary, same on-disk runtime deps
(`manifests/ + verticals/ + policies/ + skill-library/`), same runtime libs
(`libssl3 + libdbus-1-3` on `debian:bookworm-slim`).

```bash
# (operator) authenticate to ECR, build, tag, push :latest (or a SHA tag)
aws ecr get-login-password --region ap-southeast-2 | docker login --username AWS --password-stdin <ecr-url>
docker build -t <ecr-url>:latest .          # the ROOT Dockerfile
docker push <ecr-url>:latest
```

> **Why parity matters (the failure class this runbook closes):** the gateway
> reads truth-boundary policies + the pitch corpus from disk at runtime. A deploy
> image has no `Cargo.toml`, so `workspace_root()` falls back to the container cwd
> — and if the image never COPYed `policies/` + `skill-library/`, a
> policy-dependent route returns `503 pitch_unavailable`. That is the
> "policy-less gateway" class WS5 #2763 deleted for the compose path; the root
> image had silently kept it until this brief.
>
> **[agent-verified] this build:** with the real `argo` binary, a container
> layout MISSING `policies/`+`skill-library/` returns **`503 pitch_unavailable`**
> ("load .../skycot-public-pitch.toml: No such file"); the parity layout returns
> **`200` with `source:"canonical"`** and a real truth-bounded answer. The boot
> smoke that asserts this on the built image is
> [`deploy/smoke/production-image-boot-smoke.sh`](../../../deploy/smoke/production-image-boot-smoke.sh).
> The full `docker build` of the production image was NOT run in the authoring
> session (a cold release build is ~30 min); the runtime-base + COPY changes
> mirror the already-green `deploy/Dockerfile.gateway` (its 2026-06-12
> deploy-smoke first-green).

### 3. Provision the stack — [operator-runs]

```bash
cd infra/terraform
terraform init
bash ../../scripts/validate-infra.sh   # [agent-prepped] fmt + validate, no apply
terraform plan -out tf.plan            # REVIEW every created resource
terraform apply tf.plan
```

Key inputs (`variables.tf`): `domain_name` (default `api.skycot.com`),
`image_tag` (default `latest`), `front_with_cloudflare` (false for bring-up),
`route53_zone_id` (empty when DNS is on Cloudflare → validate ACM manually),
`public_base_url`, `email_outbound`, `alert_email`.

> NO THEATER: no `terraform apply` has been run against a real AWS account from
> this repo. `validate-infra.sh` (fmt + validate) is the only verification that
> has run; the plan/apply is operator-first-run.

### 4. Production env shape — [agent-prepped]

The container env is set by `modules/ecs/main.tf` and is the canonical template;
the local-compose equivalents live in `deploy/.env.deploy`. Both are kept
consistent (see the **env reference** below). The load-bearing ones:

- `ARGO_GATEWAY_PROFILE=production` — forces `auth_bypass=false`.
- `SKYCOT_AUTH_JWT_SECRET`, `ARGO_GATEWAY_BEARER_TOKEN` — auth (from secrets).
- `SKYCOT_DB_KEY_BASE64` — SQLCipher at-rest key; the boot guard **fails closed**
  without it (the image is built `--features bundled-sqlcipher`).
- `ARGO_PITCH_CORPUS_DIR=/app/skill-library/canonical-qa/corpora/skycot` +
  `ARGO_PITCH_POLICY_DIR=/app/policies/truth-boundary` — pin the on-disk policy
  + corpus locations so the policy route never falls back to a missing path
  (set in BOTH the image ENV and the task def, belt-and-suspenders).

### 5. Email / magic-link delivery (SES) — [operator-runs]

Production signup is **magic-link**, which needs an outbound email sender. Set
`SKYCOT_EMAIL_PROVIDER=smtp`, `SKYCOT_EMAIL_FROM_ADDRESS`, `SKYCOT_SMTP_HOST`
(e.g. `email-smtp.ap-southeast-2.amazonaws.com`), `SKYCOT_SMTP_PORT=587`,
`SKYCOT_SMTP_USERNAME`, and the secret `SKYCOT_SMTP_PASSWORD` (Secrets Manager
`smtp_password`). Verify the from-domain in SES and move SES out of sandbox.

> Without these, magic-link emails are **log-only** with wrong-host links — the
> gateway keeps its in-binary defaults (`provider=noop`,
> `https://app.skycot.com`). Empty env values are dropped (the gateway keeps the
> default), never overridden to `""` (`modules/ecs/main.tf` `email_env`).
> NO THEATER: live SES send is unverified until a real sign-up runs.

### 6. Stripe — [operator-runs]

Set `STRIPE_SECRET_KEY` + `STRIPE_WEBHOOK_SECRET` (secrets). Point the Stripe
webhook at `https://api.skycot.com/...` (the billing webhook route). The Stripe
PRICE id is NOT a gateway env var — the SPA sources it from the build-time
`NEXT_PUBLIC_STRIPE_PRICE_*` and sends it in the checkout body
(`modules/ecs/main.tf` documents this; the `stripe_price_ids` secret is an
operator record only).

### 7. Public base URL + DNS/TLS — [operator-runs]

Set `SKYCOT_PUBLIC_BASE_URL` to the public origin (e.g. `https://api.skycot.com`)
so magic-link + email links point at the real host. Issue the ACM cert (DNS
validation), point Cloudflare DNS at the ALB, then flip
`front_with_cloudflare=true` so the ALB SG locks to Cloudflare ranges and the
gateway trusts `X-Forwarded-For` (`ARGO_TRUST_XFF=1`) for per-IP rate limiting.

### 8. Backups — [agent-verified, MERGED #2867]

Data durability is the sibling brief
[`launch-backup-cron-s3-offsite-and-restore-drill`](../../build-plan/streams/launch-backup-cron-s3-offsite-and-restore-drill.md)
(MERGED `4a31f1136`). On a VM: `deploy/systemd/argo-backup.timer` +
`scripts/backup-cron.sh`. Containerised: the `deploy/docker-compose.backup.yml`
sidecar. Restore drill: `scripts/restore-drill.sh --local` (verified end-to-end
on the real binary: schema v94, 108 tables, integrity ok). On ECS the store is on
EFS — snapshot EFS and/or run the same `argo backup` flow against `/data`.

### 9. Monitoring — [agent-prepped]

`modules/observability` creates the CloudWatch log group + alarms (SNS to
`alert_email`). The container ships logs via `awslogs`. Prometheus/Grafana/
Alertmanager assets exist under `deploy/{prometheus,grafana,alertmanager}` for a
self-hosted metrics path. The gateway exposes `/metrics` and the
`argo gateway probe live|ready` K8s-style probes.

### 10. Post-deploy verification — [operator-runs once the instance exists]

Run, in order, and record results (NO THEATER — none of these has run against
real AWS yet):

1. `argo gateway probe ready` (or `curl https://api.skycot.com/health/ready`).
2. **Policy route:** `POST /api/v1/pitch/ask/public` returns `200` with
   `source:"canonical"` (proves policies + corpus loaded in the deployed image).
   This is exactly what `deploy/smoke/production-image-boot-smoke.sh` asserts
   locally on the built image.
3. A real stranger sign-up → magic-link email arrives → verify → onboarding.
4. A Stripe test-mode checkout → webhook received → seat provisioned.
5. Backup cron fired + a restore drill passes.

---

## Env reference (consistent across image ENV · ECS task def · compose)

Secrets come from Secrets Manager on ECS (`secret` column); plain env is set
directly. The image bakes safe defaults for the on-disk dep dirs; the task def
restates them.

| Env var | Where set | Secret? | Read by |
|---|---|---|---|
| `ARGO_GATEWAY_PROFILE=production` | task def | no | forces auth_bypass=false |
| `ARGO_GATEWAY_BEARER_TOKEN` | task def | **yes** (`bearer_token`) | bearer auth |
| `SKYCOT_AUTH_JWT_SECRET` | task def | **yes** (`auth_jwt_secret`) | session JWT |
| `SKYCOT_DB_KEY_BASE64` | task def | **yes** (`db_key`) | SQLCipher boot guard |
| `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` / `ARGO_CREDENTIALS_MASTER_KEY` | task def | **yes** (`workspace_root_key`) | credentials master key |
| `STRIPE_SECRET_KEY` / `STRIPE_WEBHOOK_SECRET` | task def | **yes** | billing |
| `SKYCOT_PUBLIC_BASE_URL` | task def | no | email/link origin |
| `SKYCOT_EMAIL_PROVIDER` / `_FROM_ADDRESS` / `SKYCOT_SMTP_HOST` / `_PORT` / `_USERNAME` | task def | no | magic-link email |
| `SKYCOT_SMTP_PASSWORD` | task def | **yes** (`smtp_password`) | SMTP auth |
| `ARGO_TRUST_XFF` / `ARGO_IP_RATE_LIMIT` | task def (when CF) | no | per-IP rate limit |
| `ARGO_HOME=/data` | image + task def | no | store path (EFS) |
| `ARGO_VERTICALS_DIR=/app/verticals` | image | no | vertical discovery |
| `ARGO_PITCH_CORPUS_DIR=/app/skill-library/canonical-qa/corpora/skycot` | image + task def | no | pitch corpus |
| `ARGO_PITCH_POLICY_DIR=/app/policies/truth-boundary` | image + task def | no | truth-boundary policies |

---

## Status (truth-bounded)

- **Verified this build (real binary):** the policy-resolution fix — broken
  layout `503`, parity layout `200 source=canonical`. The Dockerfile parity
  changes + ECS env parity + this runbook + the boot smoke.
- **Prepped + locally sound, NOT run against AWS:** every `terraform apply`,
  Secrets Manager put, ECR push, SES send, Stripe webhook, and the live
  end-to-end sign-up→pay path. **The live production deploy is UNVERIFIED until
  the instance exists.**
