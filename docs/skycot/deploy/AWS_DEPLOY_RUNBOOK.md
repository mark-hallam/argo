# SKYCOT — AWS deploy runbook (operator-run)

**Truth-bounded notice.** Nothing in this runbook has been executed against AWS.
Every step is labelled `(operator runs)` or `(operator verifies)`. The IaC
under [`infra/terraform/`](../../../infra/terraform/) has been **authored +
locally validated** (`terraform validate` + `terraform fmt -recursive -check`
pass on the branch that introduced it), but no AWS resource has been
provisioned. The operator runs the `terraform apply` step.

Target region: **ap-southeast-2 (Sydney)** per sprint playbook.
Account: the operator's $800-credit pool.
Public hostname: **api.skycot.com**.
DNS: **Cloudflare** (already managed via `deploy/dns/cloudflare-records.tf`).

---

## 0. Prereqs (operator verifies, one-off)

- AWS CLI v2 installed and configured for the target account:
  ```sh
  aws configure              # access key + secret + region=ap-southeast-2
  aws sts get-caller-identity # confirms the right account
  ```
- Terraform `>= 1.5` on PATH:
  ```sh
  terraform version
  ```
- Docker installed (for building the image locally — Apple Silicon needs
  `--platform linux/amd64`).
- A clone of this repo on the operator's box; `git pull --ff-only origin main`
  to be sure WS5-1 is included.
- Cloudflare API token with `Zone:DNS edit` on `skycot.com` (for the ACM
  validation CNAME and the final `api.skycot.com` record).

## 1. Configure terraform state (operator runs, once)

For the v1.0 launch we use **local state** on the operator's box. Operator
keeps `terraform.tfstate` in a secure backup (1Password Vault / encrypted USB)
because it contains resource ids — never commit it.

```sh
cd infra/terraform
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars — region, env, project, domain_name. Leave
# route53_zone_id empty (Cloudflare DNS path). image_tag stays "latest"
# until the first image is pushed.
```

Post-launch we migrate to an S3 backend with a DynamoDB lock table; tracked
as a follow-up brief, not WS5-1.

## 2. `terraform init` and `plan` (operator runs)

```sh
cd infra/terraform
terraform init
terraform plan -out plan.bin
# Review the plan. Expected on first apply: ~50 resources across 7 modules.
```

The operator verifies the plan matches the resource shape in this runbook:

- 1 VPC, 1 IGW, 1 NAT GW, 1 EIP, 2 public + 2 private subnets, 2 route tables, 4 RT associations.
- 1 ECR repository + 1 lifecycle policy.
- 1 EFS filesystem, 2 mount targets, 1 access point, 1 backup policy, 1 EFS SG.
- 1 KMS CMK + 1 alias (encrypts the secrets — WS5-3).
- 7 Secrets Manager secrets + 7 placeholder versions (bearer, master key, JWT, **DB key**, Stripe secret, Stripe webhook, Stripe price-ids record).
- 2 IAM roles (exec + task), 1 customer policy (GetSecretValue + kms:Decrypt), 2 attachments.
- 1 ALB, 1 target group, 1 ACM cert, 2 listeners, 1 ALB SG, 1 service SG.
- 1 ECS cluster, 1 cluster capacity provider, 1 task definition, 1 service, 1 log group.

**If anything else appears in the plan, STOP — investigate before applying.**

## 3. `terraform apply` (operator runs)

```sh
terraform apply plan.bin
```

First apply takes ~10 min (mostly the ALB + ACM cert + NAT GW). Most resources
provision in parallel; the cert sits in `PENDING_VALIDATION` until step 5.

Outputs to record:

```sh
terraform output -json | jq '{alb_dns_name, ecr_repository_url, acm_validation_records, secret_arns}'
```

The operator writes these into the 1Password vault.

## 4. Build and push the argo image (operator runs)

> ⚠️ **SQLCipher build prerequisite — the deploy will NOT boot without it.**
> In production mode (which this deploy is — both `ARGO_GATEWAY_PROFILE=production`
> and the `0.0.0.0` bind trigger it) the gateway runs a fail-closed at-rest-encryption
> gate (`crates/argo-store/src/store_open.rs:238`). It **refuses to boot** unless the
> binary was built with the `bundled-sqlcipher` Cargo feature AND `SKYCOT_DB_KEY_BASE64`
> is set. The **stock root `Dockerfile` does NOT enable the feature** — building it as-is
> produces an image that crash-loops at boot with *"enterprise mode requires SQLCipher"*.
>
> Until the Dockerfile carries the feature (tracked as a follow-up —
> `infra-dockerfile-bundled-sqlcipher-feature`), build the production image with the
> feature explicitly. Either pass it as a build-arg the Dockerfile threads into the
> `cargo build` line, or build the binary locally with
> `cargo build --locked --release -p argo-cli --bin argo --features bundled-sqlcipher`
> and `COPY` it in. Confirm the resulting image is SQLCipher-linked before pushing.

```sh
# From repo root.
aws ecr get-login-password --region ap-southeast-2 \
  | docker login --username AWS --password-stdin <ECR_REPO_URL_WITHOUT_/skycot-prod-argo>

# Build for linux/amd64 (Fargate is x86_64 in our task def).
# NB: the image MUST be SQLCipher-linked (see the prerequisite box above).
docker build --platform linux/amd64 -t skycot-argo:bootstrap .

# Tag + push to the ECR repo from terraform output.
docker tag skycot-argo:bootstrap <ECR_REPOSITORY_URL>:latest
docker push <ECR_REPOSITORY_URL>:latest

# Optional immutable pin.
docker tag skycot-argo:bootstrap <ECR_REPOSITORY_URL>:sha-$(git rev-parse --short HEAD)
docker push <ECR_REPOSITORY_URL>:sha-$(git rev-parse --short HEAD)
```

The task definition references `:latest` by default. Once the image is in ECR
the service can pull it; ECS keeps retrying until the first pull succeeds.

## 5. ACM cert DNS validation (operator runs — Cloudflare path)

The Cloudflare DNS path requires the operator to paste the validation CNAME
into Cloudflare manually.

```sh
terraform output -json acm_validation_records
# Output is a list of { name, type, value } objects. Usually one CNAME.
```

In the Cloudflare dashboard for `skycot.com`:
1. **DNS → Records → Add record**.
2. Type = `CNAME`, Name = the `name` from the output (Cloudflare will strip
   the `skycot.com.` suffix automatically), Target = the `value` from the
   output, Proxy = **OFF** (grey cloud — proxied breaks ACM validation),
   TTL = `Auto`.
3. Save.

Wait 1–5 min, then watch ACM mark the cert `ISSUED`:

```sh
aws acm describe-certificate \
  --certificate-arn $(terraform output -raw acm_certificate_arn) \
  --region ap-southeast-2 \
  --query 'Certificate.Status'
# Expect: "ISSUED"
```

(If `route53_zone_id` was set in `terraform.tfvars`, this step happens
automatically during `terraform apply` — no Cloudflare touching needed.)

## 6. Populate the secret values (operator runs)

The secrets module creates each secret with the placeholder string
`REPLACE_ME`, encrypted with the customer-managed CMK (WS5-3). The operator
populates the real values via `aws secretsmanager put-secret-value`.
`lifecycle.ignore_changes` means future `terraform apply` runs will NOT
overwrite these.

The canonical list of every secret + what reads it is
[`deploy/secrets/skycot-production.env.example`](../../../deploy/secrets/skycot-production.env.example).

```sh
PREFIX=skycot-prod    # matches var.project + var.env in terraform.tfvars

# Bearer token (hex32). KEEP THIS SECRET.
aws secretsmanager put-secret-value \
  --secret-id "${PREFIX}/argo-gateway-bearer-token" \
  --secret-string "$(openssl rand -hex 32)"

# Workspace root key (base64 of 32 raw bytes). KEEP THIS SECRET.
aws secretsmanager put-secret-value \
  --secret-id "${PREFIX}/skycot-workspace-root-key-base64" \
  --secret-string "$(openssl rand -base64 32)"

# Auth JWT secret (>=32 random bytes; the gateway HMACs __Host-session JWTs with this).
aws secretsmanager put-secret-value \
  --secret-id "${PREFIX}/skycot-auth-jwt-secret" \
  --secret-string "$(openssl rand -base64 48)"

# DB at-rest encryption key (base64 of 32 raw bytes). REQUIRED — the production
# boot gate refuses to start without it (and a SQLCipher-linked image; see §4).
aws secretsmanager put-secret-value \
  --secret-id "${PREFIX}/skycot-db-key-base64" \
  --secret-string "$(openssl rand -base64 32)"

# Stripe live keys (from the Stripe dashboard). DOUBLE-CHECK live vs test.
aws secretsmanager put-secret-value \
  --secret-id "${PREFIX}/stripe-secret-key" \
  --secret-string "sk_live_..."

aws secretsmanager put-secret-value \
  --secret-id "${PREFIX}/stripe-webhook-secret" \
  --secret-string "whsec_..."

# Stripe price IDs — JSON map keyed by tier slug from
# docs/skycot/pricing/v1-tiers.toml. Operator creates the Stripe products
# matching those slugs first.
aws secretsmanager put-secret-value \
  --secret-id "${PREFIX}/stripe-price-ids" \
  --secret-string '{"free":"price_free","pro":"price_xxx","enterprise":"price_yyy"}'
```

After populating, the operator forces the ECS service to pick up the fresh
secret values (ECS resolves secrets at task-start, not live):

```sh
aws ecs update-service \
  --cluster $(terraform output -raw ecs_cluster_name) \
  --service $(terraform output -raw ecs_service_name) \
  --force-new-deployment
```

## 7. Point Cloudflare at the ALB (operator runs)

```sh
terraform output -raw alb_dns_name
# Example: skycot-prod-alb-1234567890.ap-southeast-2.elb.amazonaws.com
```

In Cloudflare, edit the existing `api.skycot.com` record (currently pointed at
the Fly.io v4/v6 per `deploy/dns/cloudflare-records.tf`):
1. **DNS → Records → Edit `api.skycot.com`**.
2. Change to **CNAME** → the ALB DNS name above. Proxy = **ON** (orange cloud —
   gives DDoS + bot-management + WAF; WS5-4 hardens further). TTL = Auto.
3. Save.

Wait a minute for the Cloudflare cache.

## 8. Smoke test (operator verifies)

```sh
# Liveness — should be 200, JSON: {"ok":true,"checks":...}.
curl -i https://api.skycot.com/health/ready

# Production-mode confirmation — bearer-token-protected route returns 401 without auth.
curl -i https://api.skycot.com/api/v1/team
# Expect HTTP/2 401 — proves auth_bypass is OFF and the JWT/bearer gate is engaged.

# Stripe webhook (operator can defer this until after a real signup).
curl -i -X POST https://api.skycot.com/api/v1/billing/webhook \
  -H 'Stripe-Signature: t=0,v1=invalid'
# Expect 400 with a signature-error body — proves the webhook handler is mounted.
```

## 9. Image rollouts (operator runs, every deploy)

```sh
# 1. Build + push fresh image.
docker build --platform linux/amd64 -t <ECR_REPOSITORY_URL>:latest .
docker push <ECR_REPOSITORY_URL>:latest

# 2. Roll the service.
aws ecs update-service \
  --cluster $(terraform output -raw ecs_cluster_name) \
  --service $(terraform output -raw ecs_service_name) \
  --force-new-deployment

# 3. Watch the rollout.
aws ecs describe-services \
  --cluster $(terraform output -raw ecs_cluster_name) \
  --services $(terraform output -raw ecs_service_name) \
  --query 'services[0].deployments'
```

## 10. Tear-down (operator runs only if needed)

```sh
# Disable ALB deletion protection first.
aws elbv2 modify-load-balancer-attributes \
  --load-balancer-arn $(aws elbv2 describe-load-balancers \
      --names skycot-prod-alb --query 'LoadBalancers[0].LoadBalancerArn' --output text) \
  --attributes Key=deletion_protection.enabled,Value=false

# Empty the EFS filesystem first if needed (mount it temporarily and rm -rf).
# Otherwise terraform destroy can fail if there are in-flight tasks.

cd infra/terraform
terraform destroy
```

NB: `terraform destroy` does not delete Secrets Manager secrets immediately —
they enter the 7-day recovery window (WS5-3 set `recovery_window_in_days = 7`).
Force-delete with
`aws secretsmanager delete-secret --secret-id <id> --force-delete-without-recovery`
if you need the names freed sooner. The CMK has a 7-day deletion window too.

---

## What this runbook does NOT cover

- **Vercel SPA deploy.** WS5-2 (separate brief). `web/vercel.json` already
  points at `api.skycot.com`.
- **WAF / Cloudflare per-IP rate-limit / global LLM-spend cap.** WS5-4
  (separate brief).
- **Secret-rotation automation** (Lambda rotation). Post-launch follow-up.
  CMK encryption + recovery window + the DB-key secret landed in WS5-3.
- **The SQLCipher Dockerfile feature flip** (see the §4 prerequisite box) —
  tracked as `infra-dockerfile-bundled-sqlcipher-feature`. Until it lands, build
  the production image with `--features bundled-sqlcipher` manually.
- **Backups beyond the EFS default policy.** Filesystem snapshots only;
  application-level SQLite snapshots TBD.
- **Multi-instance / horizontal scale.** Single-task, single-writer SQLite is
  the v1.0 architecture; revisit after the paying-ready gate.
- **Postgres migration.** Out of scope until SQLite single-writer becomes a
  measured bottleneck.
