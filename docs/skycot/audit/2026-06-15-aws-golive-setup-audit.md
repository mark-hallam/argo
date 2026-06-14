# AWS go-live setup audit (e2e) — 2026-06-15

**Method:** 6 parallel agents, static + local audit of the deploy setup against
`origin/main` @ `903072258` (terraform/security · secrets+boot · cold-stranger journey ·
container images · networking/observability/backup · runbook). `tofu validate` run; every
finding cites file:line (NO-THEATER). Full machine output: workflow `wf_1a2f026a-40e`.
Supersedes/reconciles the [2026-06-14 deploy-readiness audit](2026-06-14-aws-deploy-readiness.md).

## Verdict: GO **pending the operator apply + a short punch-list** (raw tally: 69 PASS / 3 BLOCKER / 8 HIGH / 16 MED)

**⚠️ Correction (2026-06-15, conductor):** one of the 3 HIGH punch-list items — the JWT-fail-closed finding — was a **FALSE POSITIVE** (the guard already ships in `startup_check.rs::jwt_secret_boot_guard`, #2706). So **2** real HIGH gaps remain (backup-image DR + WAF rate-limit), not 3. See the punch-list row below.

**There are ZERO code/infra blockers.** The setup is sound: all six go-live fixes are
confirmed present + correct on current main (#2927 signup-email TF vars, #2928 ALB
Cloudflare-lockdown-by-default + double hard-gate, #2929 non-root uid 10001 in all 3
images, #2930 offsite-backup module, #2934 root-Dockerfile libdbus, #2935 web build
single-source); `tofu validate` passes on every module; security is production-grade
(deletion-protection, TLS1.3 + ACM, CMK-encrypted secrets w/ scoped decrypt, least-priv
IAM, no port-22, public-access-blocked TLS-only backup bucket). **Both prior code
blockers (B1 email-unwirable, B2 origin-open) are CLOSED; 4 of 7 prior HIGHs CLOSED.**

The "NO-GO (blockers)" tally decomposes as:
- **2 of the 3 "blockers" are the SAME thing — the stack has never been applied to AWS**
  (operator-only, spends money, honestly disclosed). This is THE go-live action, not a
  defect. It can only be closed by the operator running the apply + the §6 cold-smoke.
- **1 real blocker is a runbook copy-paste bug** (GO-LIVE §2 image build/push sequencing)
  — fixable in the doc (queued below).

## Punch-list (close before / at the apply)

### Real code/config gaps → queued as briefs
| Finding | Sev | Brief |
|---|---|---|
| ~~**JWT secret doesn't fail closed.**~~ **❌ FALSE POSITIVE — corrected 2026-06-15 (conductor verify-before-fix).** The guard DOES exist and IS fatal: `startup_check.rs::jwt_secret_boot_guard` (shipped #2706, present in this audit's base `903072258`) returns `Err` in production when `SKYCOT_AUTH_JWT_SECRET` is unset/empty/<32 bytes; `boot_serve.rs` exits (`refusing to bind`). Tested by `health_check_tests::prod_mode_missing_secret_refuses`. This audit grepped `boot.rs`/`boot_serve.rs` but the guard lives in `startup_check.rs` (invoked via `run_startup_checks`). No fix needed. | ~~HIGH~~ → **N/A** | `fix-gateway-jwt-secret-fail-closed-in-prod` (STALE-FIXED) |
| **Automated daily offsite backup never runs.** The runtime image bundles neither `scripts/` nor the AWS CLI, and `ARGO_HOME=/data` ≠ backup's `/data/.argo` → the scheduled RunTask fails daily (the module's own README/schedule.tf disclose it). Bucket/IAM live; no working automated DR. | HIGH | `fix-backup-image-bake-scripts-awscli-and-argo-home` |
| **WAF (genesis LLM-cost rate-limit) is standalone/optional** — only applies if the operator runs a 3rd apply → direct-$ LLM-cost exposure unprotected by default. | HIGH | `fix-deploy-waf-fold-into-root-or-require` |

### Runbook doc bugs that would trip the operator → queued as a doc brief
`fix-golive-runbook-doc-drift-and-sequencing` covers:
- **BLOCKER:** GO-LIVE §2 says build/push the image "after step 4's `terraform plan`", but
  `ecr_repository_url` is empty until **apply** (step 5) → `docker login ""` / `build -t ":latest"`
  fail. Move §2 after §5.
- **HIGH:** the SES sandbox-exit / production-access step is buried in a troubleshooting
  bullet, not a prominent §0 prerequisite (AWS approval ~24h). *(Mitigation: THIS operator's
  SES is already out-of-sandbox per LAUNCH-CHECKLIST-2026-06-12 — so not an active blocker
  for skycot.com, but the runbook a fresh reader follows is wrong.)*
- **HIGH:** `AWS_DEPLOY_RUNBOOK.md:538` calls `bash scripts/check-alb-origin-locked.sh` from
  inside `infra/terraform` where the path doesn't resolve (must be `../../scripts/`).
- **MED:** `AWS_DEPLOY_RUNBOOK.md:136` still says "Leave `front_with_cloudflare = false`" —
  contradicts the #2928 secure default (true); following it disables the origin lockdown
  (the §12 gate still catches it).
- **MED:** `LAUNCH-CHECKLIST-2026-06-12.md:79` sets `SKYCOT_PUBLIC_BASE_URL=https://try.skycot.com`
  but the SPA verify page is on `app.skycot.com` (try→ALB has no SPA) → a stranger's
  magic-link verify would 404. Should be `app.skycot.com`.
- **MED:** `AWS_DEPLOY_RUNBOOK.md:338/377/394` call the prod image "distroless" — it's
  `debian:bookworm-slim` (non-root uid 10001).
- **MED:** GO-LIVE §0.5 state-bucket bootstrap + §2 ECR push are tagged 🟢 agent-preppable
  but both need live AWS creds + spend real (cents) — re-tag 🔴/🟡.
- **MED:** `scripts/validate-infra.sh:33-36` hard-requires `terraform` on PATH (exits 2) — add
  a `tofu` fallback (the runbooks say "either works"; this Mac has tofu only).
- **MED:** GO-LIVE:9 says "offsite backups provisioned by the apply" with no caveat — add
  "the BUCKET is provisioned; automated daily backup needs the image follow-up
  (`fix-backup-image-...`); run `scripts/backup-cron.sh` manually meanwhile."

### Accepted-by-design (not actioned)
- **No CI gate** — DEC-055 (CI permanently off); compensating controls = the local pre-merge
  gate + the main-verify sweep + the pre-deploy gate scripts. Record in the go/no-go sign-off.
- **Stack-never-applied** — operator-only; closed by running the apply + §6 smoke.

### Polish (before a broad paying launch, not the throwaway apply)
ACM cert has no `try.skycot.com` SAN (would 526 on Full-Strict); ECR is mutable `:latest`
(set IMMUTABLE + SHA-pin the go-live image); no ALB access logs; app-level CloudWatch
alarms (spend-burn, audit-chain-broken) absent on the Fargate path; `alert_email` SNS
subscription needs a confirm-click; Cloudflare CIDRs hardcoded (refresh risk); EFS at-rest
on AWS-managed key not the CMK.

## The single highest-value action
**Operator: apply once on the $800 throwaway credits + run GO-LIVE §6** (6a machine checks
200/401/400/200 → 6b the cold-stranger human journey: stranger email → SES-delivered
magic-link → onboarding → Genesis hire → test-card pay + webhook → 6c `fresh-boot-loop.sh 10`).
That converts every "verified-in-code" claim to "verified-in-cloud" — the one gate static
analysis cannot close. Land the punch-list (esp. the JWT gate + the runbook §2 fix) first.
