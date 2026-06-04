# infra/terraform — SKYCOT AWS IaC

Terraform that provisions the AWS Fargate deploy for the SKYCOT gateway. Single
binary, single task, single AZ for the SQLite store (single-writer), behind an
ALB with TLS, with secrets sourced from AWS Secrets Manager.

**This directory is author-only in CI.** The operator runs `terraform apply`
out of band — see [`docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md`](../../docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md)
for the exact command sequence.

## Modules

| Module | What it provisions |
|---|---|
| `modules/network` | VPC, 2 public + 2 private subnets across `az_count` AZs, IGW, NAT GW, route tables. |
| `modules/ecr` | Private ECR repo `<prefix>-argo` with lifecycle (keep last 10 images). |
| `modules/efs` | EFS filesystem + access point + mount targets in the private subnets; backup policy enabled. |
| `modules/secrets` | Placeholder Secrets Manager entries (bearer / master key / JWT / Stripe). Operator populates real values. |
| `modules/iam` | ECS task-execution role + task role. Least-privilege; only logs + Secrets Manager read. |
| `modules/alb` | ALB, target group, HTTPS listener with ACM cert, HTTP→HTTPS redirect, ALB SG, service SG. |
| `modules/ecs` | ECS cluster, task definition (env + secrets + EFS mount + log config), service, CloudWatch log group. |

## Layout

```
.
├── README.md                — this file
├── .gitignore               — *.tfstate, .terraform/, *.tfvars
├── versions.tf              — terraform + AWS provider pins
├── variables.tf             — root inputs (region, env, domain, image_tag, ...)
├── main.tf                  — module wiring
├── outputs.tf               — ALB DNS, ECR URL, secret ARNs, ...
├── terraform.tfvars.example — example values (no secrets)
└── modules/
    ├── network/
    ├── ecr/
    ├── efs/
    ├── secrets/
    ├── iam/
    ├── alb/
    └── ecs/
```

## Validate locally (CI does this; no AWS creds required)

```sh
cd infra/terraform
terraform init -backend=false
terraform validate
terraform fmt -recursive -check
```

`init -backend=false` skips backend / remote-state setup — the local validate is
schema-only.

## Operator deploy

Don't try to apply from this README. See
[`docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md`](../../docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md)
for the truth-bounded, step-by-step operator runbook.

## Design notes

- **Single-task service, Recreate-style.** Single-writer SQLite + a strategy=Recreate
  deployment matches the existing `deploy/k8s/base/argo-deployment.yaml` (`replicas: 1`,
  `strategy: { type: Recreate }`). Multi-instance horizontal scaling is a post-paying-ready
  architecture decision and would need a Postgres-or-Aurora migration first.
- **EFS, not EBS, for the SQLite store.** Fargate cannot mount an EBS volume directly;
  the supported persistent store for Fargate tasks is EFS (or third-party). EFS gives us
  graceful task replacement without manual volume detach/attach.
- **ACM DNS validation is conditionally automated.** If `route53_zone_id` is set, the IaC
  creates the validation CNAME automatically. If not (the Cloudflare DNS path), the
  validation records are surfaced as a Terraform output and the operator pastes them into
  Cloudflare per the runbook.
- **Secrets are placeholders.** `terraform apply` creates the secret resources with a
  sentinel value `REPLACE_ME`; the operator populates the real values via
  `aws secretsmanager put-secret-value`. `lifecycle.ignore_changes = [secret_string]` means
  a future `terraform apply` will NOT overwrite the real values.

## Cloud-agnostic reference

[`deploy/k8s/base/`](../../deploy/k8s/base/) ships the same workload in Kubernetes manifests
for EKS / GKE / on-prem deployments. The k8s manifests stay as a reference; `infra/terraform/`
is the operative path for the v1.0 AWS launch.
