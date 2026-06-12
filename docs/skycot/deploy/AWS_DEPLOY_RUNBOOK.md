# SKYCOT — AWS deploy runbook (operator-run)

**Truth-bounded notice.** Nothing in this runbook has been executed against AWS.
Every step is labelled `(operator runs)` or `(operator verifies)`. The IaC under
[`infra/terraform/`](../../../infra/terraform/), the remote-state bootstrap under
[`infra/terraform-bootstrap/`](../../../infra/terraform-bootstrap/), the
Cloudflare DNS module under [`deploy/dns/`](../../../deploy/dns/), the standalone
WAF module under [`deploy/waf/`](../../../deploy/waf/), and the SPA/Vercel config
under [`web/`](../../../web/) have been **authored + locally validated**
(`terraform validate` + `terraform fmt -recursive -check` pass via
`bash scripts/validate-infra.sh`), but **no AWS resource has been provisioned**.
The operator runs every `terraform apply` step.

Target region: **ap-southeast-2 (Sydney)**.
Public hostname (gateway): **api.skycot.com** (`try.skycot.com` is the demo host).
SPA hostname: **app.skycot.com** + apex **skycot.com** + **www.skycot.com** (Vercel).
DNS: **Cloudflare** (managed via `deploy/dns/cloudflare-records.tf`).

This is **ONE ordered sequence**. Run the steps in order. Each step states its
exact command(s), the success signal, and the module/file it targets. Where a
step is operator-only, default-off, needs a manual confirmation click, or is NOT
created by this stack, that is called out inline.

The full operator order is also mirrored in
[`docs/skycot/deploy/README.md`](README.md).

---

## 0. Prereqs (operator verifies, one-off)

- AWS CLI v2 installed and authenticated for the target account/region:
  ```sh
  aws configure              # access key + secret + region=ap-southeast-2
  aws sts get-caller-identity # confirms the right account
  ```
- Terraform `>= 1.5` on PATH (AWS provider `hashicorp/aws ~> 5.50` is pinned):
  ```sh
  terraform version
  ```
- Docker installed (for building the image locally — Apple Silicon / arm64 hosts
  MUST build with `--platform linux/amd64`; the task def pins `cpu_architecture = X86_64`).
- A clone of this repo on the operator's box; `git pull --ff-only origin main`.
- Cloudflare API token with `Zone:DNS edit` on `skycot.com` (for the ACM
  validation CNAME and the `api`/`try` records), plus the Cloudflare `zone_id`.
- **Optional pre-flight gate** — run the infra-validate sweep before touching AWS:
  ```sh
  bash scripts/validate-infra.sh            # or --skip-docker if docker isn't on PATH
  ```
  This is a bash script (uses `mapfile`/`find`/`xargs`); on Windows run it via
  Bash/WSL, not PowerShell. It `terraform validate`s every module under `infra/`
  + `deploy/` and does a best-effort `Dockerfile --check`. **Success: every
  per-module line prints `PASS`, the `== summary ==` block reports `fail: 0`, and
  the exit code is 0** (exit code == number of failing checks). See also step 12.

## 1. Remote-state bootstrap (operator runs, once — before the main stack)

This is a classic chicken-and-egg bootstrap: `infra/terraform-bootstrap/` creates
the S3 bucket + DynamoDB lock table that the **main** stack will use as its remote
backend. It runs **once, with LOCAL state**, before anything else.
Targets: [`infra/terraform-bootstrap/main.tf`](../../../infra/terraform-bootstrap/main.tf).

```sh
cd infra/terraform-bootstrap
terraform init     # LOCAL state — this module bootstraps the remote backend; no backend block yet
terraform apply    # creates the S3 state bucket + DynamoDB lock table
terraform output   # capture state_bucket + lock_table for step 2
```

**Success signals:**
- `terraform init` → `Terraform has been successfully initialized!` (local backend).
- `terraform apply` → `Apply complete! Resources: N added` — creates
  `aws_s3_bucket.state` (**skycot-prod-tfstate**, versioning Enabled, AES256
  SSE-S3, all 4 public-access-block flags true) and `aws_dynamodb_table.locks`
  (**skycot-prod-tflock**, PAY_PER_REQUEST, hash_key `LockID` type `S`).
- `terraform output` → `state_bucket = "skycot-prod-tfstate"` and
  `lock_table = "skycot-prod-tflock"`. Note both — they go into the main stack's
  `backend.tf` (step 2).

**Truth-bounded notes:**
- Defaults: `var.project = "skycot"`, `var.env = "prod"`, `var.region = "ap-southeast-2"`.
  The bucket name is `var.state_bucket_name` (default `skycot-prod-tfstate`) and
  **must be globally unique** AWS-wide — override the variable if it's taken. The
  lock-table name is NOT a variable; it is computed `"${project}-${env}-tflock"`.
- This directory's `terraform.tfstate` is the **only local state in the entire
  deploy**. Back it up securely (1Password / encrypted USB) — it is small and
  rarely changes; never commit it.
- `prevent_destroy = true` is set on `aws_s3_bucket.state`, so `terraform destroy`
  here **refuses** (you can't accidentally nuke the state store). To intentionally
  tear it down, remove the lifecycle block first.
- Credential-free local validation (no AWS creds needed):
  ```sh
  cd infra/terraform-bootstrap && terraform init -backend=false && terraform validate && terraform fmt -check
  ```
  → `Success! The configuration is valid.` and fmt reports no changes.

## 2. Wire `backend.tf` + migrate state (operator runs, once)

Backend blocks can't use variables, so the bucket/table names are hardcoded —
which is why `infra/terraform/backend.tf` is **intentionally NOT committed**. The
operator hand-creates it from the step-1 outputs, then migrates the main stack's
state from local → S3.
Targets: [`infra/terraform/backend.tf`](../../../infra/terraform/) (operator-created)
+ [`infra/terraform-bootstrap/README.md`](../../../infra/terraform-bootstrap/README.md) (the exact block).

Create `infra/terraform/backend.tf` (paste, do not commit):

```hcl
terraform {
  backend "s3" {
    bucket         = "skycot-prod-tfstate"   # = output state_bucket from step 1
    key            = "skycot/prod/terraform.tfstate"
    region         = "ap-southeast-2"
    dynamodb_table = "skycot-prod-tflock"    # = output lock_table from step 1
    encrypt        = true
  }
}
```

Then migrate (this runs in `infra/terraform/`, the **MAIN** stack — NOT in
`terraform-bootstrap/`):

```sh
cd infra/terraform
terraform init -migrate-state    # prompts to copy existing local state -> S3; answer "yes"
```

**Success signal:** init reports `Successfully configured the backend "s3"` and
migrates local → S3. After this, the main stack reads/writes S3 with a DynamoDB
lock — safe across machines and concurrent runs.

While you're here, stage the tfvars (gitignored — never put real secrets in it):

```sh
cp terraform.tfvars.example terraform.tfvars
# Edit: region/domain_name already default correctly. Leave route53_zone_id ""
# (Cloudflare DNS path). Leave front_with_cloudflare = false. Set alert_email
# and image_tag as desired. image_tag stays "latest" until the first push (step 6).
#
# FRONT-DOOR (sign-in email) — set these or the very first user touch breaks:
#   public_base_url = "https://app.skycot.com"   # the SPA's public origin; magic-link
#                                                 # + invite emails mint links against it.
#                                                 # If unset, the gateway warns at boot and
#                                                 # falls back to https://app.skycot.com.
#   email_outbound = {                            # AWS SES path (provider=smtp):
#     provider      = "smtp"
#     from_address  = "noreply@skycot.com"
#     smtp_host     = "email-smtp.ap-southeast-2.amazonaws.com"
#     smtp_port     = "587"
#     smtp_username = "<SES SMTP username>"       # SES console -> SMTP settings
#   }
# Leave email_outbound default ({}) to ship log-only email: signup then HONESTLY
# returns status="not_configured" and the SPA shows an "email not set up" state.
# The SMTP PASSWORD is a secret — it goes through Secrets Manager (step 4),
# NEVER in this tfvars file.
```

## 3. `terraform plan` — review what gets created (operator runs)

Targets: [`infra/terraform/main.tf`](../../../infra/terraform/main.tf) (8 modules:
network, ecr, efs, kms, secrets, iam, alb, ecs, observability) +
[`infra/terraform/versions.tf`](../../../infra/terraform/versions.tf).

```sh
cd infra/terraform
terraform init                  # downloads hashicorp/aws ~> 5.50; requires Terraform >= 1.5
terraform plan -out plan.bin
# Review the plan before applying.
```

**Success signal:** `Plan: N to add, 0 to change, 0 to destroy`. Verify the shape
matches the **8 modules** below. NB there are **NO operator-required variables** —
every variable has a default (`domain_name = api.skycot.com`, `region = ap-southeast-2`);
the operator only *overrides* `front_with_cloudflare` / `route53_zone_id` /
`alert_email` / `image_tag` as posture dictates.

Expected resource shape:

- **network** — 1 VPC (`10.40.0.0/16`), IGW, single NAT GW + EIP, 2 public + 2
  private subnets across 2 AZs, route tables + associations.
- **ecr** — 1 ECR repository (`skycot-prod-argo`, MUTABLE, scan-on-push) + 1
  lifecycle policy (keep last 10 tagged, expire untagged after 7d).
- **kms** — 1 customer-managed CMK (rotation enabled, 7-day deletion window) + 1 alias.
- **secrets** — 7 Secrets Manager secrets + 7 placeholder versions
  (`REPLACE_ME`), KMS-encrypted, recovery window 7d.
- **iam** — execution role + task role + a scoped policy (`GetSecretValue` +
  `kms:Decrypt` on the CMK).
- **alb** — 1 ALB, 1 target group, 1 ACM cert, 2 listeners (443 + 80→301), ALB SG
  + service SG.
- **efs** — 1 EFS filesystem (encrypted), 2 mount targets, 1 access point (`/data`,
  uid/gid 65532), backup policy, EFS SG.
- **ecs** — 1 Fargate cluster, capacity provider (FARGATE only), 1 task definition
  (1024 CPU / 2048 MiB, X86_64/LINUX), 1 service (desired_count 1), 1 CloudWatch log group.
- **observability** — 1 SNS topic (`skycot-prod-alerts`) + optional email
  subscription + **6 CloudWatch metric alarms** (running-tasks-low, alb-unhealthy-hosts,
  alb-target-5xx, alb-elb-5xx, ecs-cpu-high, ecs-memory-high).

**If anything *else* appears in the plan, STOP — investigate before applying.**
(Module dependency order is implicit: network + ecr + kms → secrets → iam → alb →
efs → ecs → observability.)

## 4. Fill production secrets (operator runs)

The secrets module creates each secret with the placeholder string `REPLACE_ME`,
KMS-encrypted with the customer-managed CMK. **Terraform never sets real values** —
populating them is an operator-only, out-of-band step. `lifecycle.ignore_changes
= [secret_string]` means a later `terraform apply` will NOT clobber a populated
value back to `REPLACE_ME`, so you may fill them before OR after the apply (step 5).
Targets: [`infra/terraform/modules/secrets/main.tf`](../../../infra/terraform/modules/secrets/main.tf)
+ [`deploy/secrets/skycot-production.env.example`](../../../deploy/secrets/skycot-production.env.example).

All 7 full secret names (use the FULL `--secret-id`):

```sh
# Bearer token — openssl rand -hex 32. Gateway will NOT boot in production without it.
aws secretsmanager put-secret-value \
  --secret-id skycot-prod/argo-gateway-bearer-token \
  --secret-string "$(openssl rand -hex 32)"

# Workspace root key — openssl rand -base64 32. Bound to TWO env names
# (SKYCOT_WORKSPACE_ROOT_KEY_BASE64 + ARGO_CREDENTIALS_MASTER_KEY). Without it, provider-key save 503s.
aws secretsmanager put-secret-value \
  --secret-id skycot-prod/skycot-workspace-root-key-base64 \
  --secret-string "$(openssl rand -base64 32)"

# Auth JWT secret — openssl rand -base64 48. Without it, sessions die on restart (ephemeral key).
aws secretsmanager put-secret-value \
  --secret-id skycot-prod/skycot-auth-jwt-secret \
  --secret-string "$(openssl rand -base64 48)"

# DB at-rest key — openssl rand -base64 32. REQUIRED in prod — boot gate fails closed without it
# (and the image MUST be SQLCipher-linked; see step 6).
aws secretsmanager put-secret-value \
  --secret-id skycot-prod/skycot-db-key-base64 \
  --secret-string "$(openssl rand -base64 32)"

# Stripe secret key — from the Stripe dashboard (NOT generated). Double-check live vs test.
aws secretsmanager put-secret-value \
  --secret-id skycot-prod/stripe-secret-key \
  --secret-string 'sk_live_REAL_STRIPE_SECRET_KEY'

# Stripe webhook signing secret — from the Stripe endpoint config (NOT generated).
aws secretsmanager put-secret-value \
  --secret-id skycot-prod/stripe-webhook-secret \
  --secret-string 'whsec_REAL_WEBHOOK_SIGNING_SECRET'

# Stripe price IDs — JSON map. OPERATOR RECORD ONLY; NOT read by the gateway
# (the SPA carries the price id via NEXT_PUBLIC_STRIPE_PRICE_*; see step 11).
aws secretsmanager put-secret-value \
  --secret-id skycot-prod/stripe-price-ids \
  --secret-string '{"free":"price_...","pro":"price_...","enterprise":"price_..."}'
```

Verify a fill took (should NOT print the literal `REPLACE_ME`):

```sh
aws secretsmanager get-secret-value \
  --secret-id skycot-prod/argo-gateway-bearer-token \
  --query SecretString --output text
```

**Success signal (per put):** JSON with a new `VersionId` and
`VersionStages` including `AWSCURRENT` on the named secret.

**Truth-bounded notes:**
- **Operator-must-fill (gateway fails closed / degrades without them):**
  bearer token (no boot in production), auth JWT secret (sessions die on restart),
  workspace root key (provider-key save 503s), DB key (no boot in production),
  Stripe secret + webhook (required for paid tiers).
- `stripe-price-ids` is an **operator record only** — the secret exists but is
  intentionally NOT injected as a container env var.
- `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` (the platform's *own* provider keys) have
  **NO Secrets Manager secret in this module** — they are env-template entries
  only. Per-tenant keys are stored encrypted at runtime under the master key.
- Run these against the **same region** where the secrets + CMK were created
  (`ap-southeast-2`); the KMS ViaService condition is region-scoped.

## 5. `terraform apply` — provision the stack (operator runs)

Targets: [`infra/terraform/main.tf`](../../../infra/terraform/main.tf) +
[`infra/terraform/outputs.tf`](../../../infra/terraform/outputs.tf).

```sh
cd infra/terraform
terraform apply plan.bin
```

**Success signal:** `Apply complete! Resources: N added`, then the **11 outputs**
print. First apply takes ~10 min (ALB + NAT GW + cert). The ACM cert sits in
`PENDING_VALIDATION` — on the Cloudflare path (`route53_zone_id = ""`, the default)
the apply does **NOT** create the validation record and does **NOT** block on
issuance; you validate the cert out-of-band in step 7. (Only when `route53_zone_id`
is set does the IaC create the validation record and auto-validate.)

Record the outputs:

```sh
terraform output -json > skycot-outputs.json

terraform output -raw ecr_repository_url            # docker push target (step 6)
terraform output -json | jq '.acm_validation_records.value'   # CNAMEs for Cloudflare (step 7) — empty if route53_zone_id set
terraform output -json | jq -r '.alb_dns_name.value, .alb_zone_id.value'   # DNS target (step 8)
terraform output -json | jq '.secret_arns.value'    # map of 7 logical key -> Secrets Manager ARN
terraform output -raw ecs_cluster_name              # for force-new-deployment (step 6)
terraform output -raw ecs_service_name
terraform output -raw alerts_sns_topic_arn          # alarms SNS topic (step 10)
terraform output -raw acm_certificate_arn
terraform output -raw secrets_kms_key_arn
terraform output -raw efs_file_system_id
```

The 11 root outputs: `alb_dns_name`, `alb_zone_id`, `acm_certificate_arn`,
`acm_validation_records`, `ecr_repository_url`, `secret_arns`, `efs_file_system_id`,
`secrets_kms_key_arn`, `ecs_cluster_name`, `ecs_service_name`, `alerts_sns_topic_arn`.

**Truth-bounded notes:**
- The apply creates the **empty ECR repo** but does NOT build or push the image —
  that is step 6. The ECS service will keep retrying until the first image is pushed.
- The apply does NOT create any DNS record pointing `api.skycot.com` at the ALB —
  DNS is out-of-band (steps 7–8).
- Secrets remain `REPLACE_ME` unless you filled them in step 4.

## 6. Build & push the argo image to ECR (operator runs)

The argo container image is NOT built or pushed by terraform — the stack only
creates the empty ECR repo. The registry login host is derived from
`ecr_repository_url` (the segment before the first `/`), so this step runs
**after** step 5.
Targets: [`Dockerfile`](../../../Dockerfile) (ARG `CARGO_FEATURES=bundled-sqlcipher`
by default; `rust:1.82-bookworm` builder → `gcr.io/distroless/cc-debian12`) +
[`infra/terraform/modules/ecr/main.tf`](../../../infra/terraform/modules/ecr/main.tf)
+ [`infra/terraform/modules/ecs/main.tf`](../../../infra/terraform/modules/ecs/main.tf).

```sh
# From repo root. ECR login (host = ecr_repository_url before the first '/').
aws ecr get-login-password --region ap-southeast-2 \
  | docker login --username AWS --password-stdin \
      "$(terraform -chdir=infra/terraform output -raw ecr_repository_url | cut -d/ -f1)"

# Build for linux/amd64 (REQUIRED — task def pins cpu_architecture = X86_64;
# building arm64 without this flag yields an exec-format-error crash loop).
docker build --platform linux/amd64 \
  --build-arg CARGO_FEATURES=bundled-sqlcipher \
  -t "$(terraform -chdir=infra/terraform output -raw ecr_repository_url):latest" .

# Optional: also tag an immutable sha- tag for a pinnable rollout.
docker tag \
  "$(terraform -chdir=infra/terraform output -raw ecr_repository_url):latest" \
  "$(terraform -chdir=infra/terraform output -raw ecr_repository_url):sha-$(git rev-parse --short HEAD)"

# Push.
docker push "$(terraform -chdir=infra/terraform output -raw ecr_repository_url):latest"

# Roll the service onto the new image (operator-driven; terraform ignores task_definition/desired_count).
aws ecs update-service \
  --cluster "$(terraform -chdir=infra/terraform output -raw ecs_cluster_name)" \
  --service "$(terraform -chdir=infra/terraform output -raw ecs_service_name)" \
  --force-new-deployment --region ap-southeast-2

# Wait for the new task to be RUNNING + healthy in the ALB target group.
aws ecs wait services-stable \
  --cluster "$(terraform -chdir=infra/terraform output -raw ecs_cluster_name)" \
  --services "$(terraform -chdir=infra/terraform output -raw ecs_service_name)" \
  --region ap-southeast-2
```

**Success signals:**
- `docker login` → `Login Succeeded`.
- `docker build` completes; final stage is distroless with `/usr/local/bin/argo`.
- `docker push` prints the pushed digest; ECR scan-on-push runs.
- `update-service` → JSON with `rolloutState: IN_PROGRESS`. Cluster
  `skycot-prod-cluster`, service `skycot-prod-argo`.
- `wait services-stable` returns 0 once the task is RUNNING and registered
  **healthy** in the ALB target group (health = HTTP GET **`/health/ready`** on
  container port 8080, matcher 200-299).

**Truth-bounded notes:**
- The Dockerfile **defaults to `CARGO_FEATURES=bundled-sqlcipher`** — no manual
  feature flag is needed; the `--build-arg` above is explicit-but-redundant. Do
  NOT override to `""` for prod: the at-rest-encryption boot gate fails closed and
  the gateway refuses to boot (also requires `SKYCOT_DB_KEY_BASE64` from step 4).
- **Single-writer by design:** desired_count 1 + `minimum_healthy_percent 0` /
  `maximum_percent 100` means the old task is stopped *before* the new one starts —
  every rollout has a brief downtime window. This is intentional (one SQLite DB
  file on EFS `/data`; two writers would corrupt it). Do not raise desired_count.
- The container's own ECS healthcheck is a no-op (`exit 0`; distroless has no
  shell). **Trust the ALB target-group health, not the ECS container health.**
- If the task crash-loops, check CloudWatch Logs `/aws/ecs/skycot-prod-argo`
  (stream prefix `argo`) — most likely the SQLCipher boot gate (missing
  `SKYCOT_DB_KEY_BASE64` or a non-SQLCipher image).
- ECR is `image_tag_mutability = MUTABLE` — re-pushing `:latest` overwrites in
  place. For rollback-able rollouts, push + pin an immutable `sha-` tag
  (`image_tag = "sha-XXXXXXX"` in `terraform.tfvars`).

## 7. ACM cert DNS validation via Cloudflare (operator runs — Cloudflare path)

On the Cloudflare path (`route53_zone_id = ""`, the default), Terraform creates
neither the validation record nor a wait — the operator pastes the CNAME(s) into
Cloudflare by hand and waits for ACM to issue.
Targets: [`infra/terraform/modules/alb/main.tf`](../../../infra/terraform/modules/alb/main.tf)
+ [`infra/terraform/modules/alb/outputs.tf`](../../../infra/terraform/modules/alb/outputs.tf).

```sh
# Root output. (At the alb-module level this output is certificate_validation_records;
# the root re-exports it as acm_validation_records.)
terraform -chdir=infra/terraform output -json acm_validation_records
# -> JSON array of { name, type: "CNAME", value } records. Usually one.
```

In the Cloudflare dashboard for `skycot.com`:
1. **DNS → Records → Add record**.
2. Type = `CNAME`, Name = the `name` from the output (Cloudflare strips the
   `skycot.com.` suffix), Target = the `value`, **Proxy = OFF (grey cloud —
   proxied breaks ACM validation)**, TTL = `Auto`.
3. Save.

Wait 1–30 min, then confirm issuance:

```sh
aws acm describe-certificate \
  --certificate-arn "$(terraform -chdir=infra/terraform output -raw acm_certificate_arn)" \
  --region ap-southeast-2 \
  --query 'Certificate.Status' --output text
# PENDING_VALIDATION at first -> ISSUED once the CNAMEs propagate and ACM verifies.
```

**Success signal:** `Certificate.Status == ISSUED`. The HTTPS:443 listener cannot
serve a usable TLS endpoint until the cert is ISSUED. (If `route53_zone_id` was
set, this whole step is automatic during step 5 and `acm_validation_records` is empty.)

## 8. DNS → ALB cutover (operator runs)

Point `api.skycot.com` + `try.skycot.com` at the ALB. Use the Cloudflare module
in `deploy/dns/` (now ALB-aware), or edit the records by hand in the dashboard.
Targets: [`deploy/dns/cloudflare-records.tf`](../../../deploy/dns/cloudflare-records.tf)
+ [`deploy/dns/README.md`](../../../deploy/dns/README.md).

```sh
ALB=$(terraform -chdir=infra/terraform output -raw alb_dns_name)

cd deploy/dns
export CLOUDFLARE_API_TOKEN=...    # Zone:DNS edit on skycot.com
export CLOUDFLARE_ZONE_ID=...
terraform init                     # cloudflare provider ~> 4.0

terraform plan  -var "zone_id=$CLOUDFLARE_ZONE_ID" -var "alb_dns_name=$ALB"
terraform apply -var "zone_id=$CLOUDFLARE_ZONE_ID" -var "alb_dns_name=$ALB"
```

Then verify the cutover:

```sh
for host in api.skycot.com try.skycot.com; do
  echo "== $host =="; dig +short "$host" A; dig +short "$host" AAAA
done
# 104.x / 172.64-67.x answers = proxied through Cloudflare's edge (NOT raw ALB IPs).

curl -sI https://api.skycot.com/health | head -1
# HTTP/2 200 — request flows Cloudflare -> ALB -> Fargate gateway.
```

**Success signals:**
- `plan` shows `cloudflare_record.skycot["api"]` + `["try"]` as CNAME → `$ALB`
  (proxied=true); apex_a / apex_aaaa / www / app stay on Vercel.
- `apply` → `Apply complete` — `api`/`try` are CNAMEs to the ALB, orange-cloud proxied.
- `dig` returns Cloudflare edge IPs; `curl -sI .../health` → `HTTP/2 200`.

**Truth-bounded notes:**
- The ACM cert must already be **ISSUED** (step 7) before HTTPS works.
- `apex` / `www` / `app` records stay on Vercel (`76.76.21.21` / `cname.vercel-dns.com`)
  regardless of the `api`/`try` target. An **empty** `alb_dns_name` in `deploy/dns`
  falls back to legacy Fly.io A/AAAA records (the rollback path) — for the AWS
  deploy it MUST be set from `terraform output`.
- Before locking the origin (step 9), set Cloudflare **SSL/TLS → Overview →
  encryption mode = Full (Strict)** in the UI (NOT terraform) so the proxy
  validates the ALB's real ACM cert end-to-end.

## 9. Cloudflare-front lockdown — `front_with_cloudflare = true` (OPTIONAL / LATER, default-off)

**Default-off.** `front_with_cloudflare` defaults to `false` so the ALB is
directly reachable for the bring-up smoke test (step 13). Flip it to `true`
**only AFTER** `api`/`try` are confirmed proxied (step 8) and SSL/TLS = Full
(Strict) is set — otherwise you black-hole the origin (the SG would reject your
own non-Cloudflare requests). This is a **SECOND, separate `terraform apply` in
`infra/terraform/`**, NOT part of the `deploy/dns` apply.
Targets: [`infra/terraform/main.tf`](../../../infra/terraform/main.tf) (wires
`trust_xff = var.front_with_cloudflare`) +
[`infra/terraform/modules/alb/main.tf`](../../../infra/terraform/modules/alb/main.tf).

```sh
cd infra/terraform
echo 'front_with_cloudflare = true' >> terraform.tfvars
terraform apply

# Pick up ARGO_TRUST_XFF=1 + ARGO_IP_RATE_LIMIT=1 on the running task.
aws ecs update-service \
  --cluster "$(terraform output -raw ecs_cluster_name)" \
  --service "$(terraform output -raw ecs_service_name)" \
  --force-new-deployment --region ap-southeast-2
```

**Success signal:** the ALB security-group ingress on 80/443 narrows from
`0.0.0.0/0` + `::/0` to Cloudflare's published CIDRs (15 IPv4 + 7 IPv6), and the
ECS task gets BOTH `ARGO_TRUST_XFF=1` and `ARGO_IP_RATE_LIMIT=1` (the per-IP rate
limiter is enabled together with XFF trust — `modules/ecs/main.tf` `proxy_env`).
Direct-to-origin is now blocked, and the gateway's per-IP rate limiter keys on the
**real** client IP (the SG lockdown is what makes X-Forwarded-For non-spoofable —
the three are deliberately set together via the single root var).

**Truth-bounded notes:**
- The Cloudflare proxy/DNS itself is NOT created by this stack (it's step 8).
- The hardcoded Cloudflare CIDR lists in `modules/alb/variables.tf` are
  point-in-time snapshots of `cloudflare.com/ips`; if Cloudflare changes ranges,
  refresh them or legitimate edge traffic is dropped.
- **(Optional, standalone) Cloudflare WAF — `deploy/waf/`.** This is a **separate
  standalone Terraform module, NOT wired into the root stack and NOT run by CI**
  (root `main.tf` declares no `module "waf"`). It is OPTIONAL edge defense-in-depth
  that sits above the gateway's own per-IP/route limiter, and its rules only apply
  once `api.skycot.com` is Cloudflare-proxied (step 8). It is most naturally
  applied here, alongside the lockdown:
  ```sh
  cd deploy/waf
  export CLOUDFLARE_API_TOKEN=...
  terraform init
  terraform plan  -var "zone_id=<zone-id>"
  terraform apply -var "zone_id=<zone-id>"   # prints rate_limit_ruleset_id + firewall_ruleset_id
  ```
  Local-validate path (credential-free, the one CI uses):
  `cd deploy/waf && terraform init -backend=false && terraform validate && terraform fmt -recursive -check`.
  WAF only does request-rate caps (`http_ratelimit` + `http_request_firewall_custom`),
  NOT a cumulative dollar LLM-spend cap (that's a separate app-layer follow-up).

## 10. CloudWatch alarms + SNS (operator verifies)

The observability module created the SNS topic + 6 alarms in step 5. Confirm they
exist, settle to `OK`, and that an endpoint is subscribed.
Targets: [`infra/terraform/modules/observability/main.tf`](../../../infra/terraform/modules/observability/main.tf)
+ [`infra/terraform/outputs.tf`](../../../infra/terraform/outputs.tf) (`alerts_sns_topic_arn`).

```sh
# The 6 alarms.
aws cloudwatch describe-alarms \
  --alarm-name-prefix "$(terraform -chdir=infra/terraform output -raw name_prefix 2>/dev/null || echo skycot-prod)" \
  --region ap-southeast-2 \
  --query "MetricAlarms[].{Name:AlarmName,Metric:MetricName,State:StateValue}" \
  --output table

# The SNS topic ARN (root output is alerts_sns_topic_arn; the module's own is sns_topic_arn).
terraform -chdir=infra/terraform output -raw alerts_sns_topic_arn

# The subscription (if any).
aws sns list-subscriptions-by-topic \
  --topic-arn "$(terraform -chdir=infra/terraform output -raw alerts_sns_topic_arn)" \
  --region ap-southeast-2 \
  --query "Subscriptions[].{Protocol:Protocol,Endpoint:Endpoint,Arn:SubscriptionArn}" \
  --output table
```

**Success signals:**
- `describe-alarms` lists all **6** alarms: `skycot-prod-ecs-running-tasks-low`
  (RunningTaskCount, LessThanThreshold 1), `skycot-prod-alb-unhealthy-hosts`
  (HealthyHostCount, LessThanThreshold 1), `skycot-prod-alb-target-5xx`
  (HTTPCode_Target_5XX_Count, GreaterThanThreshold 10), `skycot-prod-alb-elb-5xx`
  (HTTPCode_ELB_5XX_Count, GreaterThanThreshold 5), `skycot-prod-ecs-cpu-high`
  (CPUUtilization, GreaterThanThreshold 85), `skycot-prod-ecs-memory-high`
  (MemoryUtilization, GreaterThanThreshold 85). State should be `OK` (or briefly
  `INSUFFICIENT_DATA` right after apply — not `ALARM`).
- `output -raw alerts_sns_topic_arn` prints a non-empty
  `arn:aws:sns:ap-southeast-2:<acct>:skycot-prod-alerts`.
- `list-subscriptions-by-topic`: if `alert_email` was set, one `email` row with a
  real `SubscriptionArn`.

**Truth-bounded notes:**
- **Manual confirm click required.** When `alert_email` is set, AWS emails a
  "Subscription Confirmation" link — the operator MUST click it. Until then the
  subscription shows `SubscriptionArn = PendingConfirmation` and **no alerts are
  delivered**. Terraform apply success is NOT evidence alerts will be received.
- **`alert_email = ""` (the default) = topic with NO subscription (default-off).**
  Alarms still fire and publish to the topic, but nothing receives them — wire
  Slack/PagerDuty/email to the topic ARN out-of-band.
- Only the `email` protocol is managed here; Slack/PagerDuty/SMS are
  operator-wired and won't appear as managed resources.
- `alarm_names` is a module-level output only (NOT re-exported at root) — discover
  names via `describe-alarms --alarm-name-prefix` as above.
- `service_down` + `unhealthy_hosts` use `treat_missing_data = breaching`, so a
  genuinely-missing metric (service never started) flips them to `ALARM` —
  intended, not a false positive.

## 11. SPA deploy on Vercel (operator runs — console-driven)

The SPA is a separate front-end project deployed via the Vercel dashboard — there
is no scripted `vercel deploy` in this runbook.
Targets: [`deploy/vercel/README.md`](../../../deploy/vercel/README.md) + `web/`.

1. In Vercel, import `github.com/mark-hallam/Argo-Agent`, set **Root Directory =
   `web`**, framework = Next.js, keep `web/vercel.json` defaults
   (`pnpm install --frozen-lockfile`, `pnpm build`, output `.next`).
2. **Domains** — add `app.skycot.com` AND apex `skycot.com` + `www.skycot.com` to
   the SAME project (single-app consolidation 2026-06-04; www redirects to apex).
   Do NOT add `api.skycot.com` or `try.skycot.com` (gateway/demo owned).
3. **Env vars** (build-time public config baked into the Next.js build — NOT in
   the gateway, NOT in Secrets Manager):
   - `NEXT_PUBLIC_ENV = production` (forces production gateway error behavior —
     suppresses mock fallbacks / hides demo controls). Already baked to
     `production` by `web/vercel.json`, so a build won't break if you omit it, but
     set it explicitly to match `deploy/vercel/README.md`.
   - `NEXT_PUBLIC_GATEWAY_URL = https://api.skycot.com` (the gateway pointer)
   - `NEXT_PUBLIC_API_URL = https://api.skycot.com` (compatibility alias)
   - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = 0` (auth-bypass kill switch; NOT a
     URL pointer — ignored by prod builds anyway)
   - `NEXT_PUBLIC_STRIPE_PRICE_PRO` + `NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE` (MUST
     be populated or the Pro/Enterprise CTAs fail)
   - `NEXTAUTH_URL` + `NEXTAUTH_SECRET`

**Success signals:** Vercel deployment summary shows root directory = `web` and
the build log shows `pnpm build`; Vercel issues managed TLS for `app.skycot.com`
/ `skycot.com` / `www.skycot.com`; opening the preview, `/` + `/team` + `/audit`
load, and Lighthouse Performance / Accessibility / Best Practices / SEO each ≥ 95.

**Truth-bounded notes:**
- There is no `NEXT_PUBLIC_ARGO_GATEWAY_URL` / `NEXT_PUBLIC_ARGO_GATEWAY_*` URL
  var in this repo — the gateway pointer is `NEXT_PUBLIC_GATEWAY_URL`. Do not
  invent a different name.
- `web/` is the ONLY front-end project (the old `marketing/` project was deleted
  2026-06-04; public marketing routes are auth-exempt in `web/middleware.ts`).
- The old `NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY` var was removed (not read anywhere).

## 12. Pre-deploy gate (operator runs)

Run the infra-validate sweep as the go/no-go gate before the final smoke test
(it can also serve as the step-0 pre-flight).
Targets: [`scripts/validate-infra.sh`](../../../scripts/validate-infra.sh).

```sh
bash scripts/validate-infra.sh             # or: bash scripts/validate-infra.sh --skip-docker
```

**Success signal:** every per-module line prints `PASS`, the `== summary ==` block
reports `fail: 0`, and the process exits 0 (exit code == number of failing checks).
`--skip-docker` omits the Dockerfile-syntax block (terraform-validate sweep only).

**Truth-bounded notes:**
- This is a bash script (uses `mapfile`/`find`/`xargs`) and requires `terraform`
  on PATH (exit 2 if missing). On Windows run it via Bash/WSL, not PowerShell.
- The Docker check is best-effort: it SKIPs (does not fail) if docker is absent or
  only supports the older `docker build --check`.
- It validates `terraform` config + Dockerfile syntax — it does NOT provision,
  does NOT hit AWS, and is NOT the cumulative $-spend ceiling (that's a separate
  app-layer follow-up).

## 13. Smoke test (operator verifies)

Confirm the gateway answers through the ALB / Cloudflare and that production-mode
gates are engaged.
Targets: [`infra/terraform/modules/alb/main.tf`](../../../infra/terraform/modules/alb/main.tf)
(health path `/health/ready`, matcher 200-299) + [`docs/skycot/deploy/README.md`](README.md) step 8.

```sh
# Liveness through the ALB — 200, JSON {"status":"ready","epoch_micros":N} (the
# legacy SQLite-ping probe; 503 {"status":"not_ready","reason":...} when the DB is
# unreachable). For the richer {"status":"ok","version":...,"schema_version":N,
# "uptime_seconds":S} body, hit /health or /readyz instead. (Pre-cutover, hit the
# raw ALB DNS: curl -i https://<alb_dns_name>/health/ready)
curl -i https://api.skycot.com/health/ready

# Production-mode confirmation — bearer-protected route returns 401 without auth.
curl -i https://api.skycot.com/api/v1/team
# Expect 401 — proves auth_bypass is OFF (ARGO_GATEWAY_PROFILE=production).

# Stripe webhook is mounted — bad signature returns 400.
curl -i -X POST https://api.skycot.com/api/v1/billing/webhook \
  -H 'Stripe-Signature: t=0,v1=invalid'
# Expect 400 with a signature-error body.

# SPA loads (step 11).
curl -sI https://app.skycot.com | head -1   # HTTP/2 200
```

**Success signals:** `/health/ready` → 200; `/api/v1/team` → 401; webhook → 400;
`app.skycot.com` → 200. For the full acceptance, run the cold
signup → hire → approve journey per [`docs/skycot/deploy/README.md`](README.md) step 8.

**Truth-bounded note:** a 503 at the ALB means no healthy targets are registered
yet — re-check the ECS rollout (step 6) and CloudWatch Logs `/aws/ecs/skycot-prod-argo`.

## 13b. Offsite SQLite backups + restore drill (operator runs)

EFS gives you filesystem-level durability, but a logical corruption (a bad
migration, an accidental wipe) replicates to every EFS snapshot. The
application-level backup below is the independent, point-in-time, *verified*
copy — and the drill proves it actually restores.

**The `.argo` convention.** `argo backup create` snapshots `<cwd>/.argo`, and
the gateway stores its db at `$ARGO_HOME/sessions.db`. They align only when
`ARGO_HOME` is the `.argo` subdir of the backup working dir. On EFS that means
mounting at e.g. `/data` and running the gateway with `ARGO_HOME=/data/.argo`.
The bundled scripts fail loud (never ship a silent empty backup) if the db
isn't where they expect.

**Scheduled backup.** Run `scripts/backup-cron.sh` daily — either as an
EventBridge-scheduled ECS `RunTask` mounting the same EFS access point, or via
the compose/systemd units in `deploy/` on any host that mounts the EFS volume:

```sh
# Env the task/timer needs:
#   S3_BUCKET=skycot-prod-backups  S3_PREFIX=argo-backups
#   ARGO_WORKSPACE=/data  RETENTION_DAYS=14
#   SKYCOT_DB_KEY_BASE64=<same at-rest key as the gateway>  (to drill)
# AWS auth: the task role / instance profile — no static keys on disk.
bash scripts/backup-cron.sh        # create → verify → s3 cp → prune; JSON on stdout
```

Add an S3 lifecycle rule on the bucket for remote retention (e.g. expire after
35 days, transition to Glacier after 7). The encrypted-at-rest SQLCipher db is
copied byte-for-byte; the same key opens the restored copy.

**Restore drill — schedule it; an un-restored backup is unverified.**

```sh
S3_BUCKET=skycot-prod-backups SKYCOT_DB_KEY_BASE64=<key> \
  bash scripts/restore-drill.sh                 # latest from S3 → restore → boot → /readyz
```

The drill restores into a throwaway dir, boots a real gateway against the
restored db, and asserts `/readyz` is ok plus a positive `schema_version` — it
exits non-zero on any failure. `scripts/backup-consistency-test.sh` separately
proves the snapshot is crash-consistent under concurrent writes.

## 14. Rollback + teardown (operator runs only if needed)

### Image rollout / rollback

```sh
# Build + push (rollback: re-tag a known-good sha- image as :latest), then roll.
docker push "$(terraform -chdir=infra/terraform output -raw ecr_repository_url):latest"
aws ecs update-service \
  --cluster "$(terraform -chdir=infra/terraform output -raw ecs_cluster_name)" \
  --service "$(terraform -chdir=infra/terraform output -raw ecs_service_name)" \
  --force-new-deployment --region ap-southeast-2

# Watch the rollout.
aws ecs describe-services \
  --cluster "$(terraform -chdir=infra/terraform output -raw ecs_cluster_name)" \
  --services "$(terraform -chdir=infra/terraform output -raw ecs_service_name)" \
  --query 'services[0].deployments' --region ap-southeast-2
```

For a controlled/pinnable rollback, push immutable `sha-` tags (step 6) and set
`image_tag = "sha-XXXXXXX"` in `terraform.tfvars`.

### Teardown (main stack)

```sh
# 1. Disable ALB deletion protection (enable_deletion_protection = true).
aws elbv2 modify-load-balancer-attributes \
  --load-balancer-arn "$(aws elbv2 describe-load-balancers \
      --names skycot-prod-alb --query 'LoadBalancers[0].LoadBalancerArn' --output text)" \
  --attributes Key=deletion_protection.enabled,Value=false \
  --region ap-southeast-2

# 2. Empty the EFS filesystem if needed (mount temporarily and rm -rf) so destroy
#    isn't blocked by in-flight tasks.

# 3. Destroy the main stack.
cd infra/terraform
terraform destroy
```

**Truth-bounded notes:**
- `terraform destroy` does NOT delete Secrets Manager secrets immediately — they
  enter the 7-day recovery window. Force-delete with
  `aws secretsmanager delete-secret --secret-id <id> --force-delete-without-recovery`
  to free names sooner. The CMK also has a 7-day deletion window.
- **The remote-state bucket survives.** `terraform destroy` of the MAIN stack does
  NOT and CANNOT remove the bootstrap S3 state bucket (`skycot-prod-tfstate`) — it
  has `lifecycle prevent_destroy = true` in `infra/terraform-bootstrap/main.tf`.
  Tearing down the state store is a separate, deliberate act (remove the lifecycle
  block in `infra/terraform-bootstrap/` first).
- `deploy/dns/` and `deploy/waf/` are separate operator-run modules with their own
  (local) state — destroy them separately if you want to revert DNS/WAF.

---

## What this runbook does NOT cover

- **DNS record creation by the root stack.** DNS is out-of-band (steps 7–8 via
  Cloudflare). The root apply provides `alb_dns_name` / `alb_zone_id` /
  `acm_validation_records` for whoever does it.
- **The Cloudflare WAF as part of the main apply.** `deploy/waf/` is a separate
  standalone module (not in `infra/terraform/main.tf`, not run by CI) — see the
  OPTIONAL block in step 9.
- **Secret values.** Terraform creates `REPLACE_ME` placeholders only; the
  operator populates real values (step 4).
- **Per-tenant provider keys / the platform's own `ANTHROPIC_API_KEY` /
  `OPENAI_API_KEY`.** No Secrets Manager secret in this module; per-tenant keys
  are stored encrypted at runtime under the master key.
- **A cumulative dollar LLM-spend ceiling.** WAF only does request-rate caps; the
  $-cap is a separate app-layer follow-up.
- **Secret-rotation automation** (Lambda rotation). Post-launch follow-up.
- **Backups beyond the EFS default policy.** Application-level, offsite,
  verified SQLite backups + a restore drill now ship (step 13b:
  `scripts/backup-cron.sh` + `scripts/restore-drill.sh`); the *Terraform*
  wiring of the scheduled task / S3 bucket lifecycle is still operator-run, not
  in this module.
- **Multi-instance / horizontal scale.** Single-task, single-writer SQLite on EFS
  is the v1.0 architecture (do not raise `desired_count` / `maximum_percent`
  without redesigning the store layer).
- **Postgres migration.** Out of scope until SQLite single-writer becomes a
  measured bottleneck.
