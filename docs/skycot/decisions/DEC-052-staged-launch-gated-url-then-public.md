# DEC-052 — Staged launch: gated live URL first, then public paying customers

**Date**: 2026-06-12 · **Status**: active · **Operator-locked**: yes ("i will go with your recommendations here on these items")

## Decision

1. **Staged launch path.** The first AWS deployment targets a **gated live
   URL** (single EC2, compose stack, 5–15 invited design partners /
   waitlist users on simple beta terms) — NOT public self-serve. The
   public gates open only after: a green deploy smoke, one full staging
   bring-up, a live adversarial pass against the deployed stack, a real
   Stripe test-card run, legal sign-off on Terms/Privacy, and the pricing
   model locked. Rationale: every public-launch blocker requires a deployed
   stack to even test; staging de-risks without re-work (DEC-029 applies —
   "authored" is not "works").
2. **ARGO_DUTY_AUTOFIRE armed for the gated cohort.** Scheduled autonomous
   teammate runs are enabled on the gated instance's environment (not on
   dev defaults), watched via the audit trail, before staying on for
   public. (The flag's default-off in code is unchanged; this is a
   deploy-env setting for the gated instance.)
3. **Pricing model NOT yet locked.** Operator constraint recorded: the
   model must be margin-safe ("we cannot lose money"); usage metering and
   BYOK (bring-your-own-keys) are on the table. The usage-metering +
   spend-ceiling foundations (model-agnostic) are queued now; the
   Stripe-model wiring follows the operator's pick (options doc presented
   2026-06-12).

## Launch work queued (T6 batch, same turn)

10 briefs: spendgate-ceiling, tier-quota+metering, cloud file ingress,
email honesty+base-url, grant-card wire, backup-cron+restore-drill,
production-runbook+dockerfile-parity, monitoring-stack wire, cred-file
lock, deploy-smoke first green run. Operator checklist:
`docs/skycot/operator/LAUNCH-CHECKLIST-2026-06-12.md`.
