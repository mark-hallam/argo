# SKYCOT GO-LIVE — Apply the cloud stack ONCE + run the cold-stranger smoke

**Audience: the founder (CEO, non-developer).** Every developer term is expanded the first time it appears, with a one-line plain-English note. The companion deep runbook (`AWS_DEPLOY_RUNBOOK.md`) has the every-step detail; **this file is the one remaining go-live action: stand the stack up once and prove a stranger can pay.**

**Verified against `origin/main` on 2026-06-14.** Today's fixes are merged and in code — the three deploy blockers the audit flagged are CLOSED:
- **#2927** — signup-email Terraform variables now wired (root declares `public_base_url` + `email_outbound`, threaded into the container, plus an `smtp_password` secret slot). *The runbook tfvars no longer error.*
- **#2928** — the front door is **locked by default** (only Cloudflare can reach the server; secure-by-default, no opt-in).
- **#2929** — all containers run as a **non-root** user (uid 10001), not as the all-powerful root.
- **#2930** — **offsite backups** (a second, independent copy of the database in S3) are provisioned by the apply.
- **#2934 / #2935** — both container images (the gateway/server image **and** the website image) now **build cleanly**. *These were the "won't even build" blockers; gone.*

> **Plain-English glossary (read once):**
> - **Terraform** (infrastructure-as-code — describes the cloud servers in text files so they can be created reproducibly, the same way every time).
> - **AWS** (Amazon Web Services — the cloud provider we rent servers from). **ECS Fargate** (runs our server as a managed container, no machine to babysit). **ALB** (Application Load Balancer — the public front door that receives web requests). **ECR** (the private registry we push our built container image to). **Secrets Manager** (AWS's encrypted vault for passwords/API keys). **SES** (Simple Email Service — AWS's outbound email). **Cloudflare** (the security/DNS layer in front of the server).
> - **Container image** (a frozen snapshot of our app + everything it needs to run). **Magic link** (the sign-in link emailed to a new user instead of a password). **Webhook** (a server-to-server callback — Stripe pings us when a payment happens). **Smoke test** (a quick "is it alive and behaving?" check). **tfvars** (the text file holding your chosen settings for the Terraform run).

---

## Legend

- 🟢 **AGENT-PREPPABLE** — an assistant can do/validate this for you (no real money, no live secrets).
- 🔴 **OPERATOR-ONLY** — *you* do this: it spends real money, enters real credentials, or makes a payment.

---

## 0. One-time prerequisites 🔴 OPERATOR-ONLY (account + tools)

1. **AWS account with the AWS credits applied, used as a throwaway.** This whole stand-up is disposable: prove it works, then tear it down (step 7). Nothing here is your permanent production account.
2. **Install Terraform (or OpenTofu, the open-source fork — either works).**
   ```sh
   terraform version    # need >= 1.5
   # or:  tofu version
   ```
3. **Install the AWS command-line tool + log in** (so commands can reach your account):
   ```sh
   aws configure              # paste access key + secret; region = ap-southeast-2
   aws sts get-caller-identity # prints your account number — confirm it's the right one
   ```
4. **Install Docker** (builds the container images). On an Apple-Silicon Mac, builds MUST target Intel — the commands below already pass `--platform linux/amd64`.
5. **Bootstrap the state bucket (one command set).** This creates the small S3 bucket Terraform uses to remember what it built. 🟢 An agent can run this for you; it costs cents.
   ```sh
   cd infra/terraform-bootstrap
   terraform init && terraform apply        # creates skycot-prod-tfstate + lock table
   terraform output                          # note state_bucket + lock_table
   ```
   Then create `infra/terraform/backend.tf` (do **not** commit it) from those outputs and migrate — exact block is in `AWS_DEPLOY_RUNBOOK.md` §2. 🟢 agent-preppable.

---

## 1. Fill `terraform.tfvars` 🟢 AGENT-PREPPABLE (your settings, no secrets)

```sh
cd infra/terraform
cp terraform.tfvars.example terraform.tfvars
```
Edit `terraform.tfvars` so it contains the **real front-door values** (these are NOT secrets — secrets go in step 3):
```hcl
region      = "ap-southeast-2"
domain_name = "api.skycot.com"          # the gateway's public host
alert_email = "ops@skycot.com"          # optional: who gets CloudWatch alarms

# THE FRONT DOOR — required for real signup email to work:
public_base_url = "https://app.skycot.com"   # where sign-in/magic-link URLs point
email_outbound = {
  provider      = "smtp"                                       # use AWS SES
  from_address  = "noreply@skycot.com"
  smtp_host     = "email-smtp.ap-southeast-2.amazonaws.com"
  smtp_port     = "587"
  smtp_username = "<your SES SMTP username>"   # SES console -> SMTP settings
}
# Leave front_with_cloudflare and enable_offsite_backups at their secure defaults
# (locked-down origin + offsite backups are ON by default — do not touch).
```
> The **SMTP password** is the only email secret — it does NOT go here. It goes in Secrets Manager in step 3.

---

## 2. Build + push the two container images to ECR 🟢 AGENT-PREPPABLE

Both images now build (today's #2934/#2935). The apply (step 5) creates the empty registry; this step fills it. *Run after step 4's `terraform plan` so the registry URL exists — or push right after apply, then force a redeploy.*

```sh
# Log Docker in to your private AWS registry:
aws ecr get-login-password --region ap-southeast-2 \
  | docker login --username AWS --password-stdin \
      "$(terraform -chdir=infra/terraform output -raw ecr_repository_url | cut -d/ -f1)"

# Build the GATEWAY/SERVER image (Intel arch; SQLCipher = encrypted-database support, ON by default):
docker build --platform linux/amd64 \
  -t "$(terraform -chdir=infra/terraform output -raw ecr_repository_url):latest" .

docker push "$(terraform -chdir=infra/terraform output -raw ecr_repository_url):latest"
```
> Note: the runtime image is `debian:bookworm-slim` running as **non-root uid 10001** (not "distroless" — that older runbook line is stale). The **website (SPA)** is deployed separately on Vercel (`AWS_DEPLOY_RUNBOOK.md` §11); its Docker image (#2935) is only for the all-in-AWS path.

**Success:** `docker login` → `Login Succeeded`; `docker push` prints a digest.

---

## 3. Enter the real secrets into Secrets Manager 🔴 OPERATOR-ONLY (live credentials)

Terraform creates every secret as the placeholder `REPLACE_ME`. **You** put the real values in. A later `terraform apply` will NOT overwrite them. Secret-id format is `skycot-prod/<name>`.

```sh
# Gateway bearer token — random; server REFUSES to boot in production without it:
aws secretsmanager put-secret-value --secret-id skycot-prod/argo-gateway-bearer-token \
  --secret-string "$(openssl rand -hex 32)"

# Database at-rest encryption key — random; server refuses to boot without it:
aws secretsmanager put-secret-value --secret-id skycot-prod/skycot-db-key-base64 \
  --secret-string "$(openssl rand -base64 32)"

# Sign-in session secret (JWT — the signed token that keeps a user logged in):
aws secretsmanager put-secret-value --secret-id skycot-prod/skycot-auth-jwt-secret \
  --secret-string "$(openssl rand -base64 48)"

# Workspace/credential master key — random; without it, saving provider keys fails:
aws secretsmanager put-secret-value --secret-id skycot-prod/skycot-workspace-root-key-base64 \
  --secret-string "$(openssl rand -base64 32)"

# Stripe LIVE/TEST secret key — copy from the Stripe dashboard (NOT generated):
aws secretsmanager put-secret-value --secret-id skycot-prod/stripe-secret-key \
  --secret-string 'sk_live_REAL_STRIPE_SECRET_KEY'

# Stripe webhook signing secret — from your Stripe webhook endpoint config:
aws secretsmanager put-secret-value --secret-id skycot-prod/stripe-webhook-secret \
  --secret-string 'whsec_REAL_WEBHOOK_SIGNING_SECRET'

# SES SMTP PASSWORD — the secret half of email (username/host are in tfvars):
#   SES console -> SMTP settings -> Create SMTP credentials.
aws secretsmanager put-secret-value --secret-id skycot-prod/skycot-smtp-password \
  --secret-string 'REAL_SES_SMTP_PASSWORD'
```
**Success (each):** JSON reply showing a new `VersionId` with stage `AWSCURRENT`.
> Use the **test card** path during the smoke (step 6). If you want the smoke to charge a real test card cleanly, use Stripe **test-mode** keys (`sk_test_…` / `whsec_…` from the test endpoint) for the throwaway stand-up; switch to live keys only when you open the real gate.

---

## 4. `terraform plan` — preview, no changes yet 🟢 AGENT-PREPPABLE

```sh
cd infra/terraform
terraform init
terraform plan -out plan.bin
```
**Success:** `Plan: N to add, 0 to change, 0 to destroy`, listing ~9 modules (network, ecr, efs, kms, secrets, iam, alb, ecs, observability, **backup**). If anything *else* appears, STOP and investigate before applying.

---

## 5. `terraform apply` — build the cloud stack 🔴 OPERATOR-ONLY (spends credits)

```sh
cd infra/terraform
terraform apply plan.bin
```
**Success:** `Apply complete! Resources: N added` then ~13 outputs. First apply takes ~10 minutes (load balancer + network + certificate). The TLS certificate sits in `PENDING_VALIDATION` — you finish it in `AWS_DEPLOY_RUNBOOK.md` §7 (paste one CNAME into Cloudflare, DNS-only/grey-cloud) and point `api.skycot.com` at the load balancer (§8). Then roll the service onto your pushed image:
```sh
aws ecs update-service \
  --cluster "$(terraform output -raw ecs_cluster_name)" \
  --service "$(terraform output -raw ecs_service_name)" \
  --force-new-deployment --region ap-southeast-2
aws ecs wait services-stable \
  --cluster "$(terraform output -raw ecs_cluster_name)" \
  --services "$(terraform output -raw ecs_service_name)" --region ap-southeast-2
```
**Before the cold-stranger smoke, confirm the origin is locked** (the secure-default hard-gate):
```sh
terraform plan -out tfplan
terraform show -json tfplan | bash ../../scripts/check-alb-origin-locked.sh   # must exit 0 / "PASS"
```

---

## 6. THE COLD-STRANGER CLOUD SMOKE — the real go / no-go 🔴 OPERATOR-ONLY

This is what converts every "verified-in-code" claim into "verified-in-cloud." Do all of it.

**6a. Machine checks (fast):**
```sh
# Liveness — server answers through Cloudflare -> load balancer -> Fargate:
curl -i https://api.skycot.com/health/ready          # expect: 200, {"status":"ready",...}

# Auth is really ON (a protected route refuses anonymous access):
curl -i https://api.skycot.com/api/v1/team           # expect: 401 (NOT 200)

# Stripe webhook rejects a forged signature:
curl -i -X POST https://api.skycot.com/api/v1/billing/webhook \
  -H 'Stripe-Signature: t=0,v1=invalid'              # expect: 400

# Website loads:
curl -sI https://app.skycot.com | head -1            # expect: HTTP/2 200
```
Pass = `200`, `401`, `400`, `200`. Any `503` at the load balancer means no healthy server yet — re-check the rollout and CloudWatch logs `/aws/ecs/skycot-prod-argo`.

**6b. The real human journey (the actual proof) — do this from a fresh browser, as if you were a stranger:**
1. Go to `https://app.skycot.com`, **sign up with a brand-new email address** you control.
2. **The magic-link email actually arrives from SES** (check the inbox + spam). *If no email arrives, signup email is misconfigured — see step 8.* Click the link.
3. Complete **onboarding** into a fresh, isolated workspace.
4. Do the **Genesis first-hire** flow — describe a teammate in plain English and watch it get created.
5. **Pay with a Stripe test card** (`4242 4242 4242 4242`, any future expiry, any CVC) to upgrade the tier. Confirm the tier flips and the Stripe webhook is received (server-to-server payment confirmation lands).

**6c. Fresh-boot reliability loop (proves the server boots cleanly on a brand-new disk 10 times):**
```sh
bash deploy/smoke/fresh-boot-loop.sh 10              # expect: 10/10 cycles pass
```

**GO** only if 6a + 6b + 6c all pass. This is the single highest-value action before any paying user.

---

## 7. Rollback / teardown 🔴 OPERATOR-ONLY (it's a throwaway)

```sh
# Roll back the app to a known-good image: re-tag a previous sha- image as :latest, push,
# then force-new-deployment (same update-service command as step 5).

# Tear the whole stack down:
aws elbv2 modify-load-balancer-attributes \
  --load-balancer-arn "$(aws elbv2 describe-load-balancers --names skycot-prod-alb \
      --query 'LoadBalancers[0].LoadBalancerArn' --output text)" \
  --attributes Key=deletion_protection.enabled,Value=false --region ap-southeast-2
cd infra/terraform && terraform destroy
```
**Notes:** secrets + the encryption key enter a 7-day recovery window (force-delete with `aws secretsmanager delete-secret --force-delete-without-recovery` to free names sooner). The **state bucket survives on purpose** (`prevent_destroy`). DNS and the optional Cloudflare WAF (`deploy/dns/`, `deploy/waf/`) are separate modules — destroy them separately if used.

---

## 8. What success looks like — and what to do if a step fails

**Success =** all of 6a (200/401/400/200) + the full 6b human journey (stranger email → magic link **delivers from SES** → onboarding → Genesis first-hire → **test-card payment + webhook**) + 6c (10/10 fresh boots). At that point every cloud claim is verified-in-cloud, and a gated/invite paying launch is reachable.

**If a step fails:**
- **`terraform plan` errors on an undeclared variable** → you're on stale code. `git pull --ff-only origin main` (today's #2927 declares `public_base_url`/`email_outbound`); re-plan.
- **Server won't start / `503` at the load balancer** → check `/aws/ecs/skycot-prod-argo` logs. Almost always a missing secret from step 3 (bearer token or DB key — the server fails closed on purpose).
- **No magic-link email arrives** → confirm `email_outbound` in tfvars (step 1) AND the `skycot-smtp-password` secret (step 3) are both set, the SES sending domain is verified, and you're **out of the SES sandbox** (sandbox only sends to pre-verified addresses — a true stranger email won't deliver until production access is granted).
- **Origin-locked gate (step 5) exits 1** → a bring-up override is still in tfvars; remove `front_with_cloudflare=false` + `allow_open_origin_bringup=true`, re-plan. NO-GO until it exits 0.
- **Payment doesn't complete / webhook not received** → verify the Stripe keys in step 3 match the mode you're testing (test vs live) and that the webhook endpoint in Stripe points at `https://api.skycot.com/api/v1/billing/webhook`.
- **Anything in 6b feels mocked** → it isn't supposed to be; stop and treat as a real bug (NO-THEATER). The whole point of this stand-up is that the camera-visible path is real.

---

*Authored 2026-06-14, grounded against origin/main (every cited fix verified merged). The one remaining go-live action is OPERATOR-ONLY: stand the stack up once and run the cold-stranger smoke. The deep step detail lives in `AWS_DEPLOY_RUNBOOK.md`.*
