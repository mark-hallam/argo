# AWS deploy-readiness review — 2026-06-14

**Driver:** conductor workflow `wf_932d337a-652` (5 parallel area audits -> synthesis). All findings code-verified against origin/main; NONE cloud-verified (stack never applied). NO-THEATER lens.

**Verdict:** NOT go-live ready · 2 BLOCKER · 10 HIGH · 39 total findings.

> NOT go-live ready for paying users today — but much closer than the prompt feared, and the gap is wiring/sequencing, not a rotten architecture. The IaC is genuinely production-grade (ECS Fargate, scoped IAM, CMK-encrypted Secrets Manager, TLS 1.3, fail-closed boot gates, clean remote state) and SpendGate + tier-quota + verified backups have all shipped since the last readiness doc. The stop-ships are three concrete defects on the documented path: signup email is unwirable via the runbook (root Terraform never declares public_base_url/email_outbound — verified in code), the ALB ships open to 0.0.0.0/0 by default so every Cloudflare/WAF protection is bypassable at launch (verified: front_with_cloudflare defaults false, no AWS WAF anywhere), and the entire stack has never touched AWS so the assembled cloud chain is unproven. Fix those plus the offsite-backup and root-user gaps and a gated/invite paying launch is reachable in days, not weeks.

## Go-live blockers

- **BLOCKER (Terraform IaC): Magic-link signup email is unwirable on the documented path. The runbook (AWS_DEPLOY_RUNBOOK.md:139-154) tells the operator to set public_base_url + email_outbound in terraform.tfvars, but neither variable is declared in infra/terraform/variables.tf and the root module "ecs" block (infra/terraform/main.tf:88-108) never passes them (CODE-VERIFIED this turn: grep returns 0 in variables.tf; the ecs block only threads secret_arns/trust_xff). A verbatim paste errors with 'undeclared variable'; omitting them (the only thing that plans cleanly) boots the task with email provider=noop, so the first stranger's magic-link is written log-only (auth_magic_link_delivery.rs:53-57 NotConfigured) and NO real user can complete signup. Compounded by no smtp_password slot in the secrets module (modules/secrets/main.tf:18-47), so the SES password half is also unwired. Fix: declare both root vars + an smtp_password secret and thread all three into module "ecs"; re-run terraform validate. Introduced by commit f0c124a83 (#2856).**

- **BLOCKER (Edge security): The ALB origin is internet-open by default and every edge protection is therefore bypassable at go-live. front_with_cloudflare defaults to false (CODE-VERIFIED this turn: infra/terraform/variables.tf:72-76), which sets the ALB SG to 0.0.0.0/0 + ::/0 (modules/alb/main.tf:4-7). All Cloudflare rate-limits/firewall rules (deploy/waf/) only apply to traffic that traverses Cloudflare, so an attacker who resolves the public ALB DNS name hits the origin directly and bypasses every edge control — including the genesis LLM-cost block. The lockdown is documented as OPTIONAL/LATER (runbook:477-484) with nothing forcing it before users arrive. Until the SG is Cloudflare-CIDR-only, the gateway's own per-IP limiter is also spoofable via X-Forwarded-For. Fix: default front_with_cloudflare=true for prod (or gate apply on it), and sequence the runbook so cutover→Full-Strict→origin-lockdown all happen in the SAME launch window, not a later second apply. Make 'origin open to 0.0.0.0/0' a hard pre-deploy gate (§12).**

- **BLOCKER (Cloud verification): The entire AWS stack has never been applied — every cloud claim is verified-from-code only, never verified-in-cloud. Local terraform validate passes and the docker-compose smoke ran green on a Mac (deploy/smoke/LAST-PASS.md, 2026-06-12, 4/4), but the assembled chain is unproven: ACM DNS-validation via Cloudflare, the __Host-session cookie surviving browser→Vercel→Cloudflare→ALB→Fargate, the SQLCipher boot gate on a fresh EFS volume, ECR pull through NAT, Secrets Manager injection. The known fresh-volume exit-78 was root-caused/fixed (#2866) but the 10-cycle fresh-boot acceptance was NOT run in-band. This is honestly disclosed (not theater), but it is a hard readiness gap. Fix: stand the stack up once on the AWS credits as a throwaway, run runbook step-13 smoke (/health/ready→200, /api/v1/team→401, bad-sig webhook→400) PLUS the cold-stranger signup→verify→isolated-workspace journey PLUS deploy/smoke/fresh-boot-loop.sh 10. Highest-value single action before any paying user.**

## Prioritised punch-list

- [BLOCKER] Declare root Terraform vars public_base_url + email_outbound + add an smtp_password secret to modules/secrets, and thread all three into module "ecs" (infra/terraform/main.tf:88-108); re-run terraform validate. Without this, signup email cannot be configured on the documented path. (Terraform IaC)
- [BLOCKER] Default front_with_cloudflare=true for prod (or gate apply), and re-sequence the runbook so cutover→Full-Strict→origin-lockdown happen in one launch window; add a §12 pre-deploy hard-gate that fails if the ALB SG is not Cloudflare-CIDR-only. Verify with curl direct-to-ALB-DNS from a non-Cloudflare IP returning timeout. (Edge security)
- [BLOCKER] Apply the stack once on the AWS credits as a throwaway and run the full smoke + cold-stranger-signup journey + fresh-boot-loop.sh 10 to convert every verified-from-code claim into verified-in-cloud. (Cloud verification)
- [HIGH] Add the smtp_password entry to modules/secrets/main.tf secret_specs (REPLACE_ME + ignore_changes) so its ARN flows into the IAM read grant and the ECS lookup automatically — otherwise the SES email path has no Secrets Manager slot even after the email-wiring fix. (Terraform IaC) [closely related to BLOCKER #1]
- [HIGH] Add a non-root USER to all three runtime Dockerfiles (gateway, web, root) + k8s securityContext (runAsNonRoot/runAsUser 10001, allowPrivilegeEscalation:false); confirm the gateway can still write /data and bind the port via both smoke scripts. All images currently run as root (UID 0). (Containers)
- [HIGH] Restore a minimal CI gate before/at go-live: re-enable ci.yml + audit.yml + deny.yml (or a self-hosted runner, or a mandatory documented pre-merge ritual running conductor-pre-merge-gate.sh + cargo-audit + cargo-deny). Today zero automated test/lint/RUSTSEC gate runs on merges to main. Also delete/relabel deploy-prod.yml.disabled — it targets legacy Fly.io, not AWS ECS. (CI/CD)
- [HIGH] Complete runbook §13b offsite backup before paying users: create the S3 backup bucket (versioned + lifecycle: Glacier 7d / expire 35d + public-access-blocked), wire an EventBridge-scheduled ECS RunTask running backup-cron.sh daily, run restore-drill.sh once against the live bucket. Strongly recommend promoting this into a Terraform module so 'apply' yields offsite backups by default. A fresh apply currently has EFS-snapshot durability ONLY. (Backups) [BLOCKER-adjacent for data safety]
- [HIGH] Decide edge-WAF posture consciously: the 'WAF' is rate-limiting only (deploy/waf/main.tf — no OWASP managed ruleset, bot control, or IP allow/deny; Free-plan scoped). At minimum enable Bot Fight Mode + IP-reputation challenge + OWASP signature rules on API paths, or enable the Cloudflare Managed Ruleset. Document residual risk in the sign-off. (Edge security)
- [HIGH] Either fold deploy/waf into the root stack as module "waf" or add a mandatory §12 step that applies it and verifies rate_limit_ruleset_id + firewall_ruleset_id are live — the genesis LLM-cost rate-limit must be treated launch-blocking given direct dollar exposure. Today the WAF is a third, separate, optional terraform apply nothing enforces. (Edge security)
- [HIGH] Make the docker-compose deployment boundary non-foot-gunnable: the compose/.env.deploy path is missing 3 of 7 required secrets (JWT, credential master key, Stripe) and runs in NON-production demo auth. Label docker-compose.yml 'NOT for paying users' at the top and route operators to infra/terraform + secrets/skycot-production.env.example. (Containers)
- [MEDIUM] Add subject_alternative_names=["try.skycot.com"] to the ACM cert (or issue a wildcard) — the cert is single-domain api.skycot.com but DNS CNAMEs both api and try at the same ALB, so try.skycot.com fails the Full(Strict) origin handshake (526). (Edge security)
- [MEDIUM] Recreate the app-level alerts (TenantSpendBurnRateHigh >$100/hr, AuditChainBroken) on the AWS Fargate path — they currently run ONLY in the compose/VM Prometheus stack. Add CloudWatch metric/log alarms (or an ADOT sidecar). And set alert_email + confirm the SNS subscription (verify via aws sns list-subscriptions-by-topic showing a real ARN, not PendingConfirmation) before launch. (Monitoring)
- [MEDIUM] Fix the runbook: it calls the prod image 'distroless' (lines 368, 385-386) but the root Dockerfile is debian:bookworm-slim (has a shell). Update the text and either drop the no-op-healthcheck claim or set the ECS container healthcheck to 'argo gateway probe live'. Confirm modules/alb/variables.tf health_check_path default == /health/ready (it is now the sole readiness gate). (Containers/Docs)
- [MEDIUM] Add ALB access logging (access_logs block → encrypted/lifecycle'd S3 bucket) — currently no edge request log feeds the abuse/cost-controls doc. (Terraform IaC)
- [MEDIUM] Source Cloudflare CIDRs dynamically (http data source or cloudflare_ip_ranges) and/or surface them as root vars — hardcoded snapshots in module defaults will silently black-hole new Cloudflare ranges with no alarm. (Edge security)
- [MEDIUM] Align/clean the k8s base manifests (reference-only): add a prominent 'NOT production-complete' banner, fix the secret set (add SKYCOT_DB_KEY_BASE64/JWT/master-key/Stripe; it would crash-loop or run UNENCRYPTED as-is), drop the unused postgres scaffold, pin the image, add securityContext. (Containers)
- [MEDIUM] Bake awscli into the backup sidecar image (it apt-get installs at every cold start, fails without debian-mirror egress) and document that backups default to EXCLUDE credentials (INCLUDE_CREDENTIALS=0) so a DR drill recovers sessions but NOT per-tenant provider keys. (Backups)
- [LOW] Set drop_invalid_header_fields=true + desync_mitigation_mode="strictest" on the ALB (defense-in-depth given XFF trust). (Terraform IaC)
- [LOW] Set ECR image_tag_mutability=IMMUTABLE and make the go-live image_tag SHA-pinned, not :latest. (Terraform IaC)
- [LOW] Pin both gateway Dockerfiles to the same Rust tag (1.83 vs 1.82 skew between the 'parity' images) and run BOTH the compose smoke and production-image-boot-smoke.sh in CI so the actual ECS image is exercised. (Containers)
- [LOW] Add healthchecks + depends_on condition:service_healthy to the canonical compose web/nginx services (mirror the smoke compose) to avoid 502s on restart. (Containers)
- [LOW] Consider managed_challenge instead of hard block for the global /api ceiling, or key on session/tenant signal, so shared-NAT enterprise offices aren't collectively blocked. (Edge security)
- [LOW] Reconcile /metrics auth: compose Prometheus scrapes /metrics with no bearer; fine for the loopback demo but will 401 against a production gateway. Document the assumption or add a bearer to the scrape_config. (Monitoring)
- [LOW/POST-LAUNCH] Secret rotation is deferred (acceptable). Post-launch prioritize Stripe key + bearer-token rotation (no session/data impact); treat JWT-secret + DB-key rotation as documented deliberate-migration events. (Secrets)
- [POST-LAUNCH] Track multi-AZ NAT + per-AZ HA once the service goes multi-instance (requires the SQLite→Postgres/Aurora migration off single-writer); optionally move EFS at-rest to the CMK for unified key governance. Accepted MVP trade-off today. (Terraform IaC)
- [DOC HYGIENE] Refresh DEPLOY-READINESS-2026-06-08.md (its two §5 'blockers' — spend ceiling + tier lock — both shipped via #2865/#2871) and mark launch-deploy-monitoring-stack-wire.md MERGED so stale problem-statements aren't mistaken for current state. Verify the SpendGate prod-default caps ($1/$10/$100) match intent and that overage Stripe round-trip is tested with a live card. (Docs)

---

## Full report

# SKYCOT AWS Deploy-Readiness Audit — Consolidated Review

**Reviewer:** Lead consolidation pass over 5 area audits
**Date:** 2026-06-14
**Scope:** Single-binary Rust gateway + Next.js SPA, going live for paying users on AWS (ECS Fargate path)
**Verdict:** **NOT go-live ready today.** 3 BLOCKERS, all fixable in days. Architecture is sound; the gaps are wiring, sequencing, and the fact that nothing has touched AWS yet.

---

## Blunt bottom line

The prompt feared an EC2/EBS/RDS jumble. That is not what is here. The Terraform provisions a **proper ECS Fargate single-task service** behind an ALB with ACM TLS 1.3, EFS for the single-writer SQLite store, CMK-encrypted Secrets Manager injection, least-privilege IAM, and a textbook remote-state bootstrap. Boot gates fail **closed** (no DB key → refuse boot; no bearer token → refuse boot; loopback → non-production auth). Secrets are **not** committed. SpendGate + tier-quota + verified backups have all shipped since the last readiness doc, which now *understates* readiness.

So the honest framing is: **this is close.** A *gated/invite* paying launch is reachable in days, not weeks. What stops "go live today" is three concrete defects on the documented operator path plus a never-applied stack:

1. **Signup email is unwirable via the runbook** (root Terraform never declares the variables the runbook tells you to set — I verified this in code this turn).
2. **The origin ships open to the whole internet by default** (`front_with_cloudflare=false`), making every Cloudflare/WAF protection bypassable at launch — also verified this turn.
3. **The stack has never been applied to AWS** — every cloud claim is verified-from-code, never verified-in-cloud.

Two more are BLOCKER-adjacent for a paying-customer bar: **all containers run as root**, and **offsite backups are 100% operator-manual** (a fresh apply has EFS-snapshot durability only).

---

## Go-live BLOCKERS (severity-sorted, must fix before paying users)

### B1 — Magic-link signup email is unwirable on the documented path `[Terraform IaC]`
The ECS module fully supports `SKYCOT_PUBLIC_BASE_URL` + SES/SMTP email env (`modules/ecs/variables.tf:93-109`, `modules/ecs/main.tf:60-78`), and the runbook (`AWS_DEPLOY_RUNBOOK.md:139-154`) instructs the operator to set `public_base_url` + `email_outbound` in `terraform.tfvars` under *"set these or the very first user touch breaks."* **But the root never wires them.**

**Verified this turn:**
- `grep -nE 'public_base_url|email_outbound' infra/terraform/variables.tf` → **0 hits** (neither variable declared)
- `infra/terraform/main.tf:88-108` `module "ecs"` block passes only `secret_arns` + `trust_xff` (and standard infra args) — **never** `public_base_url`, `email_outbound`, or an `smtp_password` ARN.

Consequence: pasting the step-2 tfvars verbatim produces a Terraform *"Value for undeclared variable"* error; omitting them (the only thing that plans cleanly) boots the task with `public_base_url=''` and email provider defaulting to `noop` → magic-link returns `status=not_configured` and writes the link to the trace log only (`crates/argo-cli/src/handlers/gateway/auth_magic_link_delivery.rs:53-57,98-101`). **No real self-serve user can complete signup.** Introduced by commit `f0c124a83` (#2856), which added the module support + runbook text but never the root wiring.

> **Note — dedupe:** the Terraform audit and the Documented-state audit reported this independently (same defect). Merged into one blocker.

**Fix:** declare root vars `public_base_url` (string, default "") + `email_outbound` (object matching `modules/ecs/variables.tf:99-109`), add an `smtp_password` secret, thread all three into `module "ecs"`, re-run `terraform validate`. Until then, fix the runbook to stop instructing operators to set non-existent variables, and treat "working signup email on AWS" as **UNVERIFIED** until proven in the first cold-signup smoke.

### B2 — ALB origin is internet-open by default; Cloudflare WAF + rate-limits are bypassable `[Edge security]`
**Verified this turn:** `front_with_cloudflare` defaults to **false** (`infra/terraform/variables.tf:72-76`), which sets `alb_ingress_v4=["0.0.0.0/0"]` / v6=`["::/0"]` on the ALB SG (`modules/alb/main.tf:4-7`). The ALB is internet-facing (`internal=false`) and its DNS name is a non-secret Terraform output.

Because **every** Cloudflare protection (the `deploy/waf/` rate-limits incl. the genesis LLM-cost block, the empty-UA challenge) applies only to traffic that traverses Cloudflare's proxy, an attacker who resolves the ALB DNS name hits the origin directly and bypasses all of it. The runbook makes origin-lockdown an **OPTIONAL / LATER / separate second apply** (`AWS_DEPLOY_RUNBOOK.md:477-484`) — nothing forces it before anonymous/paying users arrive. Compounding: until the SG is Cloudflare-CIDR-only, the gateway's own per-IP limiter keys on the **spoofable** X-Forwarded-For header, so the app-layer fallback is also defeated.

**Fix:** default `front_with_cloudflare=true` for prod (or gate `apply` on it), and re-sequence the runbook so **cutover → Full-Strict → origin-lockdown happen in the same launch window**, not "LATER." Add a §12 pre-deploy hard-gate that fails if the ALB SG is not Cloudflare-CIDR-only; verify with `curl` direct-to-ALB-DNS from a non-Cloudflare IP returning timeout. (The chicken-and-egg the runbook flags is real — the correct order is proxied first, *then* lock.)

### B3 — Entire AWS stack has never been applied; every cloud claim is code-verified only `[Cloud verification]`
Both readiness docs and the runbook state plainly (and correctly — this is honest disclosure, not theater) that **no `terraform apply` has run against AWS.** Local `terraform validate` passes and the docker-compose smoke came up green on a Mac (`deploy/smoke/LAST-PASS.md`, 2026-06-12, 4/4), but the *assembled cloud chain* is unproven: ACM DNS-validation via Cloudflare grey-cloud, the `__Host-session` cookie surviving browser→Vercel→Cloudflare→ALB→Fargate, the SQLCipher boot gate on a fresh EFS volume, ECS pulling from ECR through NAT, Secrets Manager injection. The known fresh-volume `exit-78` was root-caused/fixed (#2866 bounded store-probe retry) but the **10-cycle fresh-boot acceptance was not run in-band** (`BUILD_STATUS.md:270`).

**Fix (highest-value single action):** stand the stack up once on the AWS credits as a throwaway, run runbook step-13 smoke (`/health/ready`→200, `/api/v1/team`→401, bad-sig webhook→400) **plus** the cold-stranger signup→verify→isolated-workspace journey **plus** `deploy/smoke/fresh-boot-loop.sh 10` for 10/10 evidence.

---

## HIGH (fix before paying users; two are BLOCKER-adjacent)

| Area | Finding | Evidence |
|---|---|---|
| Terraform IaC | **No `smtp_password` secret** in the secrets module — the SES path has no Secrets Manager slot or IAM read grant even after B1 is fixed. | `modules/ecs/main.tf:75-78` (lookup); `modules/secrets/main.tf:18-47` (no slot); `modules/iam/main.tf:29-31` (grant scoped to existing ARNs) |
| Containers | **All images run as root** — no `USER` directive in any Dockerfile; k8s has no `securityContext`. Gateway parses untrusted webhooks + runs computer-use as UID 0; gated-demo host uses `network_mode: host`. | `deploy/Dockerfile.gateway:39-60`; `deploy/Dockerfile.web:42-52`; `Dockerfile:56-86`; `deploy/k8s/base/argo-deployment.yaml:21-44` |
| Containers | **Compose/.env.deploy path missing 3 of 7 required secrets** (JWT, credential master key, Stripe) and runs in non-production demo auth — anyone treating compose as the paying-user deploy ships a billing-broken, session-fragile, single-tenant gateway. | `deploy/.env.deploy.example:7-31`; `deploy/secrets/skycot-production.env.example:14-17,38-69`; `deploy/docker-compose.yml:1-5` |
| Edge security | **No managed OWASP ruleset / bot control / IP allow-deny** — the "WAF" is rate-limiting only (Free-plan scoped). Zero edge signature defense against SQLi/XSS/path-traversal/credential-stuffing; entire OWASP-class defense rests on app code. | `deploy/waf/main.tf:66-81`; `deploy/waf/README.md:18-19`; no AWS WAFv2/Shield anywhere (grep confirmed empty this turn) |
| Edge security | **WAF module is never applied by the deploy path** — standalone module, not in the root stack, not in CI, labeled OPTIONAL. The genesis rate-limit (the "strongest edge mitigation for runaway LLM cost") simply won't exist unless the operator runs a third separate `apply`. | `infra/terraform/main.tf` (no `module "waf"`); `AWS_DEPLOY_RUNBOOK.md:514-526` |
| CI/CD | **Zero CI runs** — every workflow is `.yml.disabled` and Actions disabled at repo level; no automated fmt/clippy/test/loc-budget/cargo-audit/cargo-deny gate on merges to main. A regression or known-vulnerable dep can reach main unseen. The one deploy workflow targets **legacy Fly.io**, not AWS ECS. | `.github/workflows/DISABLED_WORKFLOWS_POLICY.md:4,45-51`; `deploy-prod.yml.disabled:1-32` |
| Backups (data safety) | **Offsite S3 backup bucket, lifecycle, and schedule are 100% operator-manual** — not in Terraform (only the EFS backup policy exists). A fresh `apply` yields EFS-snapshot durability ONLY; a logical corruption replicates into every snapshot with no independent offsite copy until hand-wired. App-level backup scripts themselves are good and verified (#2867). **BLOCKER-adjacent.** | `AWS_DEPLOY_RUNBOOK.md:685-726,799-803`; `scripts/backup-cron.sh:50-111`; `modules/efs/main.tf:42-45` (no `aws_scheduler`/`cloudwatch_event` RunTask in infra/terraform) |
| Cloud verification | **(B3, also HIGH-rated by the Documented-state audit)** — stack never applied. Consolidated into BLOCKER B3 above. | — |

---

## MEDIUM

| Area | Finding | Evidence |
|---|---|---|
| Edge security | ACM cert is single-domain `api.skycot.com` with no SAN for `try.skycot.com`, which CNAMEs the same ALB → fails Full(Strict) origin handshake (526). Add SAN or wildcard. | `modules/alb/main.tf:107-118`; `deploy/dns/cloudflare-records.tf:62-64`; `AWS_DEPLOY_RUNBOOK.md:430-432,473-475` |
| Monitoring | App-level alerts (`TenantSpendBurnRateHigh`, `AuditChainBroken`) run **only** in the compose/VM Prometheus plane, NOT on Fargate. AWS path has 6 infra CloudWatch alarms but no spend/audit-integrity paging. Plus `alert_email` defaults empty + SNS needs a manual confirm-click → alarms can fire into a void. | `deploy/alertmanager/rules.yml`; `modules/observability/main.tf:20-149`; `deploy/prometheus/prometheus.yml`; `AWS_DEPLOY_RUNBOOK.md:572-583` |
| Containers / Docs | Runbook calls the prod image **'distroless'** (lines 368, 385-386) but the root Dockerfile is **debian:bookworm-slim** (has a shell — explicitly not distroless, rationale at `Dockerfile:48-58`). Drives a wrong healthcheck assumption. Also confirm `health_check_path` default == `/health/ready` (now the sole readiness gate). | `Dockerfile:48-58`; `AWS_DEPLOY_RUNBOOK.md:368,385-386`; `modules/alb/variables.tf:33` |
| Terraform IaC | **No ALB access logging** (`access_logs` block absent) — removes the primary edge forensic/abuse trail; even feeds the existing `abuse-and-cost-controls.md`. | `modules/alb/main.tf:68-81` |
| Edge security | Hardcoded Cloudflare CIDR snapshots in module defaults, not surfaced as root vars and not refreshed dynamically → a new Cloudflare range silently black-holes some users with no alarm. | `modules/alb/variables.tf:45-79`; `infra/terraform/main.tf:77-86`; `AWS_DEPLOY_RUNBOOK.md:511-513` |
| Containers | k8s base manifests are an incomplete reference: secret omits `SKYCOT_DB_KEY_BASE64` (crash-loop in prod OR unencrypted in dev) + JWT/master-key/Stripe; ships an unused postgres StatefulSet; unpinned `argo:dev` image; no securityContext. Would be dangerous if mistaken for production. | `deploy/k8s/base/argo-secret.example.yaml:28-37`; `argo-deployment.yaml:24,33-37`; `postgres-statefulset.yaml` |
| Containers | Compose gateway runs in **non-production auth mode** (loopback bind) with the only access control a single shared nginx basic-auth password + demo-mode SPA. Fine as a gated demo; a hole if mistaken for a real multi-user product. | `deploy/Dockerfile.gateway:9-11,59-60`; `routes_auth_production_mode.rs:47-49`; `deploy/docker-compose.yml:28-29`; `deploy/nginx.conf:26-27` |
| Containers | Backup sidecar depends on a fragile `ARGO_HOME`↔`.argo` symlink, installs awscli via apt at every cold start (fails without egress), and excludes credentials by default (`INCLUDE_CREDENTIALS=0`) — a DR drill recovers sessions but NOT per-tenant provider keys. | `deploy/docker-compose.backup.yml:33-34,46-50`; `scripts/backup-cron.sh:19-27,42,56,69-74` |
| Terraform IaC | **Default posture ships ALB open to 0.0.0.0/0** — this is the *procedural* face of B2 (transient bring-up state acceptable; steady-state go-live state is not). Folded into B2; flagged here for the IaC reader. Note: correctly NO SSH/22 open anywhere. | `infra/terraform/variables.tf:72-76`; `modules/alb/main.tf:4-7`; `AWS_DEPLOY_RUNBOOK.md:136` |

---

## LOW

- **ALB missing desync/header hardening** — set `drop_invalid_header_fields=true` + `desync_mitigation_mode="strictest"` (defense-in-depth given XFF trust). `modules/alb/main.tf:68-81`
- **ECR image tags MUTABLE** + default `:latest` — set `IMMUTABLE` + SHA-pinned tag for reproducible/auditable deploys. `modules/ecr/main.tf:3`; `variables.tf:42-46`
- **Single NAT GW** SPOF across both AZs — accepted MVP trade-off (service is single-task anyway). Track post-launch with the SQLite→Postgres migration. Also: EFS at-rest is on the AWS-managed key, not the CMK. `modules/network/main.tf:68-104`; `modules/efs/main.tf:27-40`
- **Rust version skew** between the two "parity" gateway images (1.83 vs 1.82); compose smoke doesn't exercise the actual ECS root image (covered separately by `production-image-boot-smoke.sh` — run both in CI). `deploy/Dockerfile.gateway:14`; `Dockerfile:19`
- **Canonical compose web/nginx have no healthcheck + bare depends_on** → 502s on restart. Mirror the smoke compose. `deploy/docker-compose.yml:32-45`
- **Genesis/global rate-limit rules hard-block keyed purely on ip.src** — shared-NAT enterprise offices (the target customer) can be collectively blocked; consider `managed_challenge` for the /api ceiling. `deploy/waf/main.tf:36-63`
- **`/metrics` auth mismatch** — compose Prometheus scrapes `/metrics` with no bearer; will 401 against a production gateway. Document the loopback assumption or add a bearer. `routes_auth.rs:158-167`; `deploy/prometheus/prometheus.yml`
- **Secret rotation deferred** to post-launch — acceptable for v1; prioritize Stripe key + bearer-token rotation later (no session/data impact). `AWS_DEPLOY_RUNBOOK.md:798`

---

## INFO — genuinely solid, do NOT churn on these (recording so they aren't "fixed" away)

- **Remote-state bootstrap, scoped IAM, secrets handling, TLS** are production-grade. Versioned/encrypted/public-access-blocked S3 + DynamoDB lock + `prevent_destroy`; IAM grants `GetSecretValue` only on exact ARNs + `kms:Decrypt` on one CMK (no wildcards), empty task role; secrets are placeholders with `ignore_changes`, CMK-encrypted; TLS 1.3 policy + 80→443 redirect + ACM DNS validation. `infra/terraform-bootstrap/main.tf:60-102`; `modules/iam/main.tf:25-61`; `modules/secrets/main.tf:58-76`; `modules/alb/main.tf:146-173`
- **Fail-closed boot gates are real and verified** — production refuses to boot without bundled-sqlcipher + `SKYCOT_DB_KEY_BASE64` (`store_open.rs:247-256`); `ARGO_GATEWAY_PROFILE=production` forces `auth_bypass=false` (`routes_auth_production_mode.rs`); bind-aware check fails closed to production when unset.
- **Build hygiene is good** — genuine multi-stage builds, pinned base images (no `:latest`), careful `.dockerignore`, separate `production-image-boot-smoke.sh` that builds the REAL root image, verified backup→S3 path with empty-DB guard.
- **No secrets committed** — repo-wide scan for `sk_live_`/`whsec_`/`AKIA`/private keys found ZERO real credentials; only `.example` templates and clearly-labeled CI throwaways. `.gitignore` correctly excludes `.env.deploy`/`.htpasswd`/`secrets/*.env`.
- **Secrets Manager injection via `valueFrom`** (never plaintext env), KMS-CMK encrypted, 7-day recovery, `stripe_price_ids` deliberately not injected.
- **ECS container healthcheck is a no-op (exit 0)** — correct and disclosed; the real gate is the ALB target-group health on `/health/ready` (matcher 200-299).
- **SpendGate (#2865) + tier-quota (#2871) + app-level backups/restore-drill (#2867) all shipped** — the DEPLOY-READINESS-2026-06-08 doc's two §5 "blockers" are closed; that doc now *understates* readiness.
- **Gateway port split (7400 demo / 8080 AWS)** is internally consistent and documented — keep runbook curls pinned to the public host so no port is hand-typed.
- **Stripe webhook signature verification** with 5-min tolerance is implemented (`crates/argo-billing`).

---

## What changed since the docs were written (favourable)

The most recent dedicated readiness doc (`DEPLOY-READINESS-2026-06-08.md`) is **stale in the operator's favour**: its two §5 "fully-public blockers" — no hard $ spend ceiling, no hard tier lock — have both shipped (SpendGate #2865 with hand-verified gating of all `build_transport` sites incl. pitch/genesis/ghostwriter/orchestration/distillation; tier-quota #2871 Free 100/mo + Pro 5000/mo LIVE). App-level offsite backups + restore drill also now ship. **Refresh that doc** before the go/no-go so the operator isn't reading a more-pessimistic picture than reality. Residual honest caveats to verify: SpendGate prod-default caps ($1/$10/$100) match intent; the tier-quota seam resolves tenant from gateway cwd not request session (correct for single-tenant, lumps buckets for multi-tenant — flagged follow-up); overage Stripe round-trip is UNVERIFIED-LIVE (needs an operator test-card).

## Recommended path to "go live"
A **gated/invite paying launch** is the right first step (code/IaC supports it). Sequence: fix B1 (email wiring) + B2 (origin lockdown default) → add `smtp_password` secret + non-root USER → complete §13b offsite backups → restore a minimal CI/security gate → **apply once on credits and run the full cold-stranger smoke + fresh-boot-loop 10× (B3)** → then open the gate. Fully-public adds the edge-WAF hardening + multi-tenant quota-seam follow-up on top.