# sprint-ws5-1-aws-iac-single-binary-gateway

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: nothing (independent infra lane).
**Authored against**: origin/main `3e36d058d` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws5-1-aws-iac-single-binary-gateway`.

## Why

`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5) finds:

> No AWS-native IaC. Zero AWS/ECS/Fargate/App-Runner/EKS Terraform or CloudFormation. The k8s
> manifests are cloud-agnostic and could deploy to EKS, but there's no cluster IaC, no ALB
> ingress config, no RDS/EBS/ACM wiring.

The container is ready (root `Dockerfile` builds the single `argo` binary, ships it on
distroless, exposes `:8080` via `gateway start --bind 0.0.0.0 --bind-non-loopback --port 8080`),
k8s manifests exist, and a Cloudflare-DNS-as-code module exists at
`deploy/dns/cloudflare-records.tf`. What's missing is the **AWS provisioning** that gets the
container running behind a public TLS endpoint at `api.skycot.com`.

Sprint playbook (`docs/skycot/SPRINT-paying-ready-2026-06-04.md`) targets **AWS Sydney
(ap-southeast-2)** on the operator's $800 credit pool. **Single-binary + single-writer SQLite**
→ ECS Fargate (one task, single AZ) is the right primitive — simpler than EKS, no node fleet
to manage, fits the strategy=Recreate single-replica deploy shape already in the k8s manifests.

## What

Author cloud-deployable Terraform under `infra/terraform/` for the production AWS deploy of
the SKYCOT gateway. **Author + locally validate ONLY** — `terraform validate`, `terraform fmt`,
`docker build`. **Do NOT run a live `terraform apply`** (that's operator-gated; needs AWS creds).

The IaC produces:

1. **Networking** — a fresh VPC (or operator-provided VPC, via variable) with 2 public + 2
   private subnets across 2 AZs in `ap-southeast-2`, an Internet Gateway, NAT Gateway, route
   tables. (NAT is needed for Fargate tasks in private subnets to pull the ECR image and reach
   provider APIs; the ALB lives in public subnets.)
2. **ECR** — a private repository `skycot-argo` for the gateway image.
3. **Storage** — an **EFS** filesystem + access point + mount targets for the SQLite store
   (`/data`) so a task restart preserves state. Single AZ is acceptable (single-writer
   SQLite), but EFS gives us a sane Recreate-style swap on task replacement without the
   EBS-volume-attach-detach dance. Backup policy: ENABLED.
4. **Secrets** — `aws_secretsmanager_secret` placeholders for `ARGO_GATEWAY_BEARER_TOKEN`,
   `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` (a.k.a. `ARGO_CREDENTIALS_MASTER_KEY`),
   `SKYCOT_AUTH_JWT_SECRET`, `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and a
   `stripe_price_ids` JSON secret. The IaC creates the secret resources (so the task role can
   read them) — **operator populates the values out-of-band** via the runbook (the SecretString
   is set to a placeholder sentinel `"REPLACE_ME"` so a `terraform apply` doesn't overwrite
   real values on re-apply; `lifecycle.ignore_changes = [secret_string]`). WS5-3 expands this.
5. **ECS Fargate** — cluster + task definition + service:
   - Container: `<ecr-repo>:<image_tag>` (default `latest`, override via variable).
   - CPU/memory: 1 vCPU / 2 GB (overridable).
   - Env: `ARGO_GATEWAY_PROFILE=production` (forces `auth_bypass=false`),
     `ARGO_HOME=/data`, `RUST_LOG=info`.
   - Secrets: bearer / master / JWT / Stripe sourced from Secrets Manager via the task
     execution role.
   - Mount: EFS access point at `/data`.
   - Health check: `/health/ready` against container port 8080.
   - Logs: CloudWatch Logs group `/aws/ecs/skycot-argo` with 30-day retention.
   - Service: `desired_count = 1`, `deployment_minimum_healthy_percent = 0`,
     `deployment_maximum_percent = 100` (Recreate-style; matches k8s strategy).
6. **ALB + ACM** — Application Load Balancer in public subnets; target group on port 8080;
   ACM-issued TLS cert for `api.skycot.com` (DNS validation); HTTPS:443 listener forwarding
   to the target group, HTTP:80 listener redirecting to HTTPS. `aws_route53_record` for
   the DNS validation challenge is **only created if a `route53_zone_id` variable is set** —
   if the operator runs DNS on Cloudflare (`deploy/dns/cloudflare-records.tf` already
   manages `api.skycot.com` A/AAAA), validation is done manually per the runbook.
7. **IAM** — task execution role (ECR pull + CloudWatch logs + Secrets Manager read), task
   role (least-privilege; no AWS API access beyond logs by default — the binary doesn't make
   AWS API calls).
8. **Security groups** — ALB SG (ingress 80+443 from 0.0.0.0/0), service SG (ingress 8080
   from ALB SG only), EFS SG (NFS 2049 from service SG only).

Plus:

9. **`deploy/k8s/base/` reference** — the k8s manifests already exist + ARE cloud-agnostic.
   Add ONE comment to `deploy/k8s/README.md` explaining that they are the cloud-agnostic
   reference (EKS, GKE, on-prem) and `infra/terraform/` is the operative AWS path for v1.0
   public launch. Do NOT modify the manifests themselves.

10. **`docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md`** — operator runbook with the EXACT
    commands the operator runs (init / plan / apply, secret population via
    `aws secretsmanager put-secret-value`, image build + push to ECR, DNS validation, smoke
    test). Truth-bounded: the runbook never claims any deploy step has been executed; it
    documents what the operator will do.

## Layout

```
infra/
  terraform/
    README.md                    — IaC overview + module wiring
    .gitignore                   — *.tfstate, .terraform/, *.tfvars (NEVER commit secrets)
    versions.tf                  — terraform + provider version pins
    variables.tf                 — region, env, domain_name, image_tag, vpc_cidr, ...
    main.tf                      — top-level module wiring
    outputs.tf                   — alb_dns_name, ecr_repo_url, secret ARNs
    terraform.tfvars.example     — example values (no secrets)
    modules/
      network/                   — VPC, subnets, IGW, NAT, route tables
      ecr/                       — private repo + lifecycle policy
      efs/                       — filesystem, access point, mount targets, backup
      secrets/                   — Secrets Manager entries (placeholder SecretString)
      iam/                       — task-execution role + task role
      alb/                       — ALB, target group, listeners, ACM cert
      ecs/                       — cluster, task def, service, CloudWatch log group
docs/skycot/deploy/
  AWS_DEPLOY_RUNBOOK.md          — exact operator commands (init/plan/apply, image push, DNS, smoke)
```

## May-touch

- `infra/terraform/**` (NEW — all files under this tree)
- `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` (NEW)
- `deploy/k8s/README.md` (ADD a 3-line "cloud-agnostic reference" note pointing at
  `infra/terraform/` — do NOT modify any of the .yaml files)
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open).

Do NOT touch:
- Any Rust crate (no behavior changes — this PR is infra only).
- The root `Dockerfile` (it already produces the image we need; if anything is missing, that's
  a separate brief).
- `deploy/dns/cloudflare-records.tf` (already correct for Cloudflare DNS — the runbook just
  references it).
- The k8s manifests under `deploy/k8s/base/` (cloud-agnostic; not the AWS path).

## Acceptance criteria

- [ ] `cd infra/terraform && terraform init -backend=false && terraform validate` exits 0
      against the root + every module. Report the exact pass output.
- [ ] `terraform fmt -recursive -check infra/terraform` exits 0 (no fmt diffs).
- [ ] Every module has a `README.md` (one paragraph: what it provisions + the resources
      it creates) and a `variables.tf` declaring its inputs.
- [ ] `infra/terraform/.gitignore` contains `*.tfstate*`, `.terraform/`, `*.tfvars`
      (so accidental `terraform apply` artefacts + `secret.auto.tfvars` cannot land in git).
- [ ] No file > 500 LOC, no Rust file changed (this is infra IaC + docs only).
- [ ] `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` exists and documents the **exact** operator
      command sequence: `terraform init` → `terraform plan` → `terraform apply` → ECR
      `docker login` + `docker push` → `aws secretsmanager put-secret-value ...` for each
      of the 5 secrets → DNS A-record at Cloudflare → smoke test (`curl
      https://api.skycot.com/health/ready` expecting 200).
- [ ] Runbook is **truth-bounded**: every step is labelled "(operator runs)" / "(verifies)";
      no claim that the deploy has happened.
- [ ] The Stripe + master-key + JWT-secret + bearer-token secrets have `lifecycle {
      ignore_changes = [secret_string] }` so a re-apply after the operator populates them
      does NOT overwrite real values.
- [ ] Task definition env wires `ARGO_GATEWAY_PROFILE=production`, `ARGO_HOME=/data`,
      `RUST_LOG=info`. Container port = 8080. Health check path = `/health/ready`.
- [ ] PR body lists every Terraform module + the `terraform validate` exit-0 evidence
      (paste the success line) + the `terraform fmt -check` clean output.

## Out of scope

- A live AWS `terraform apply` (operator runs this; this PR ships the IaC + runbook only).
- Populating real secret values (`STRIPE_SECRET_KEY`, etc.) — operator's step, documented
  in WS5-3 and the runbook.
- The Next.js SPA deploy story (Vercel; tracked separately as WS5-2).
- Cloudflare/WAF rules + global LLM-spend cap (WS5-4, separate brief).
- AWS account / IAM bootstrap (the operator's $800-credit AWS account already exists per
  sprint playbook).
- Horizontal scaling. v1.0 is single-instance single-writer SQLite. Multi-instance is a
  post-paying-ready architecture decision.

## Parallel-surface check

- `rg 'infra/terraform' .` — zero matches today; this brief creates the tree.
- `rg 'ecs|fargate' deploy/` — only doc-level mentions in k8s README; no parallel AWS IaC.
- `deploy/dns/cloudflare-records.tf` — separate concern (DNS records); no overlap with
  AWS resource provisioning.
- `deploy/flyio/` — historical Fly.io path; not the AWS lane.

## Source

`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5 section, "Missing for AWS +
anonymous public traffic"). Sprint playbook
`docs/skycot/SPRINT-paying-ready-2026-06-04.md` (WS5; AWS Sydney target;
$800 credit pool). Backend/infra-lane Windows lane B sprint mandate.
