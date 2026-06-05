# infra-validate-all-terraform-modules

**Authored against:** origin/main fcf9b31ff on 2026-06-05.
**Validate before claim:** bash scripts/check-brief-freshness.sh infra-validate-all-terraform-modules

**Owner**: windows-infra (lane B). **WS5 deploy support. No live-deploy.**

## Why

The infra surface now spans 9 terraform modules under `infra/terraform/` +
`deploy/dns/` + (queued via open PRs) `deploy/waf/` + `infra/terraform-bootstrap/`.
An operator about to deploy has no one-command sanity check that every module
parses cleanly. PRs land green individually, but module-vs-module drift (a
variable removed in `iam` that `ecs` still references, an outputs.tf rename) only
shows up when `terraform validate` is run on each module root and the root stack
together.

## What

Add `scripts/validate-infra.sh` — iterates every dir containing a top-level
`*.tf`, runs `terraform init -backend=false -input=false && terraform validate`,
and reports a PASS/FAIL summary. Exit code = number of failing checks. Optional
`--skip-docker` flag; when docker IS available, also runs `docker build --check`
against the gateway `Dockerfile` (HCL parse + linter; does not actually build).

## Parallel-surface declaration

- `scripts/validate-brief.sh` exists but is for AUTHORING-time brief content
  validation, NOT terraform/Dockerfile sanity. Different surface.
- No existing `validate-infra.sh` or equivalent on origin/main (rg verified).

## Acceptance

- `bash scripts/validate-infra.sh --skip-docker` on origin/main → PASS for 9
  modules (alb, ecr, ecs, efs, iam, network, secrets, infra/terraform root,
  deploy/dns).
- Exit code 0 when all modules pass.
- After WS5 PRs land (#2685 KMS/secrets, #2688 WAF, #2692 cloudflare, #2693
  observability, #2695 bootstrap), the same command auto-discovers + validates
  the new modules.
- Script is `100755` (executable bit set via `git update-index --chmod=+x`).

## Out of scope

- terraform plan / apply (needs AWS creds; operator-only)
- ECR image build/push (separate runbook)
- AWS smoke after deploy (separate runbook)
