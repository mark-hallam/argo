# infra-finalize-aws-deploy-runbook (Windows / infra lane — non-cargo, docs-only)

**Owner**: windows or claude (docs-only; no build).
**Depends on**: the 8 merged infra PRs (state/#2695, KMS/#2685, abuse/#2688, CloudWatch/#2693, DNS/#2691, SPA-verify/#2689, Docker/#2686, validate-infra/#2707).
**Authored against**: origin/main `da38d892a` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh infra-finalize-aws-deploy-runbook`.

## Why
The full AWS-deploy IaC stack is now merged, but it's spread across the modules. The operator needs **ONE complete, ordered runbook** to do the live deploy (`terraform apply` + secrets + DNS) without guessing the sequence. `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` exists but was touched by several PRs — finalize it into a single authoritative, step-by-step guide.

## Task (docs-only — NO live commands run)
Finalize `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` as an ordered sequence. Each step = the exact command + what success looks like + the relevant module. Cover, in order:
0. **Prereqs** — AWS credentials/profile, terraform version, region.
1. **Remote-state bootstrap** (#2695) — S3 bucket + DynamoDB lock, `terraform init` against the backend.
2. **`terraform plan`** — review; call out what gets created.
3. **Fill secrets** (#2685) — replace the `REPLACE_ME` sentinels in Secrets Manager (the KMS CMK + DB key + gateway secrets); note `lifecycle.ignore_changes` so re-applies don't clobber them.
4. **`terraform apply`** — ECS/Fargate gateway + ALB up (#2686 Docker image, ECS/ALB modules).
5. **DNS → ALB** (#2691) — point `api`/`try.skycot.com` at the ALB; verify resolution.
6. **Cloudflare front toggle** (#2692, once landed) — enable `front_with_cloudflare`, lock origin to Cloudflare IPs.
7. **CloudWatch alarms + SNS** (#2693) — confirm alarms armed + the SNS subscription.
8. **SPA Vercel deploy** (#2689) — deploy the web app, point it at the gateway, verify.
9. **Pre-deploy gate** — `bash scripts/validate-infra.sh` (#2707) as the go/no-go check.
Add a **rollback** note + a **smoke-test** (hit the gateway health endpoint through the ALB) at the end.

## Acceptance
A reader can follow the runbook top-to-bottom to stand up the stack; every step names its exact command + success signal + module; no step is missing or out of order. Truth-bounded — if a step isn't actually wired yet, say so, don't invent it.

## PR
Title: `infra(docs) — finalize the end-to-end AWS deploy runbook`. Docs-only.
