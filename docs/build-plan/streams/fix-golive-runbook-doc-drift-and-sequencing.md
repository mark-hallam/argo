# fix-golive-runbook-doc-drift-and-sequencing

**Authored against:** origin/main `7f98b30cf` on 2026-06-15
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-golive-runbook-doc-drift-and-sequencing`
**Priority:** 🔴 P1 — contains a guaranteed copy-paste FAILURE for the operator (§2) + items
that would silently break a fresh operator's apply / cold-stranger journey. Land before the apply.

## Problem (VERIFIED 2026-06-15 AWS go-live audit — exact file:line)

Doc-drift + sequencing bugs in the go-live runbooks (all are docs, except the one
`validate-infra.sh` 1-liner). Fix each:

1. **(BLOCKER) Image build/push sequencing.** `GO-LIVE-APPLY-AND-COLD-SMOKE-2026-06-14.md` §2
   (~line 80) says run the docker login/build/push "after step 4's `terraform plan` so the
   registry URL exists" — but `terraform output -raw ecr_repository_url` is EMPTY until
   `terraform apply` (step 5) creates the ECR repo. As written, §2 yields `docker login ... ""`
   + `docker build -t ":latest"` failures. **Fix:** rewrite §2's lead-in to "Run AFTER step 5
   (apply), which creates the ECR repo + populates `ecr_repository_url`" (delete the "after
   step 4 plan" clause), or move §2 physically after §5.
2. **(HIGH) SES sandbox prominence.** The SES production-access (sandbox-exit) requirement is
   only in a GO-LIVE troubleshooting bullet (~line 235) and ABSENT from `AWS_DEPLOY_RUNBOOK.md`
   (grep sandbox = 0). **Fix:** add a prominent §0 OPERATOR-ONLY prerequisite in BOTH runbooks
   ("Confirm SES is OUT of the sandbox in your region + sending domain/DKIM verified — a fresh
   AWS account is sandboxed; a real stranger email will NOT deliver until production access is
   granted, ~24h"). Cross-link `LAUNCH-CHECKLIST-2026-06-12.md` (proves it done for skycot.com).
3. **(HIGH) check-alb-origin-locked.sh path.** `AWS_DEPLOY_RUNBOOK.md:538` calls
   `bash scripts/check-alb-origin-locked.sh` from inside `infra/terraform` where that path does
   NOT resolve. **Fix:** `bash ../../scripts/check-alb-origin-locked.sh` (the other two
   invocations — GO-LIVE:169, AWS_DEPLOY §12a:675 — are already correct).
4. **(MED) front_with_cloudflare stale instruction.** `AWS_DEPLOY_RUNBOOK.md:136` says "Leave
   `front_with_cloudflare = false`" — contradicts the #2928 secure default (true). **Fix:**
   "Leave at its secure default (true); only set false + `allow_open_origin_bringup=true` for
   the transient pre-DNS bring-up, then revert" (align with GO-LIVE:71/236).
5. **(MED) SKYCOT_PUBLIC_BASE_URL value.** `LAUNCH-CHECKLIST-2026-06-12.md:79` =
   `https://try.skycot.com`, but `/auth/verify` is an SPA route on `app.skycot.com`
   (try→ALB has no SPA). **Fix:** `https://app.skycot.com` + a note that the base URL must be
   the Vercel SPA origin, not the ALB gateway origin.
6. **(MED) distroless staleness.** `AWS_DEPLOY_RUNBOOK.md:338/377/394` call the prod image
   "distroless" — it's `debian:bookworm-slim` (non-root uid 10001, HAS a shell). **Fix:**
   correct the 3 lines + reword the "distroless has no shell" healthcheck rationale (the ECS
   CMD-SHELL no-op healthcheck works; the real gate is ALB target-group health on /health/ready).
7. **(MED) agent-preppable mis-tag.** GO-LIVE §0.5 (state-bucket bootstrap) + §2 (ECR push) are
   tagged 🟢 agent-preppable but both need live AWS creds + spend real money. **Fix:** re-tag
   🔴 OPERATOR-ONLY (or a 🟡 "needs AWS creds, ~cents" tier) — they violate the doc's own
   "no real money, no live secrets" definition of 🟢.
8. **(MED) validate-infra.sh tofu fallback.** `scripts/validate-infra.sh:33-36` hard-requires
   `terraform` (exits 2) — the runbooks say "Terraform or OpenTofu, either works", and a
   tofu-only host fails the §0/§12b pre-flight. **Fix:** `TF=$(command -v terraform || command -v tofu)`.
9. **(MED) backup "provisioned" caveat.** GO-LIVE:9 says "offsite backups ... provisioned by
   the apply" with no caveat. **Fix:** "the backup BUCKET is provisioned; automated daily
   backups need the image follow-up (`fix-backup-image-bake-scripts-awscli-and-argo-home`) —
   run `scripts/backup-cron.sh` manually (§13b) until then."

## Verifier
- Re-read each edited location; the command in #1 is sequenced after apply; #3 path resolves;
  #8 `validate-infra.sh` runs green on a tofu-only host. No remaining `front_with_cloudflare =
  false` / `distroless` / `try.skycot.com`-as-base-url instructions in the operator path.

**LOC budget:** doc edits across 3 files + a 1-line `validate-infra.sh` change.
