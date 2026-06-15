# SKYCOT go-live readiness — prerequisites tracker (2026-06-15)

Live tracker for the **one-time full-production AWS apply + cold-stranger test**
(the DEC-034 ship gate). Updated as each prerequisite is confirmed. Deploy path =
**ECS Fargate + ALB** (full production state). AWS account **123456789012**,
region **ap-southeast-2 (Sydney)**.

| # | Prerequisite | Status | Evidence / notes |
|---|---|---|---|
| 1 | **SES out of sandbox + DKIM** | ✅ **CONFIRMED 2026-06-15** | See SES record below. A cold stranger's signup email will deliver. |
| 2 | **Stripe** (keys + Pro price + overage price + webhook) | 🟡 in progress | Operator has the account + is creating test-mode keys/products. Spec: `docs/skycot/deploy/` Stripe section. |
| 3 | **Cloudflare API token** (DNS + WAF/Ruleset edit on skycot.com) | ⬜ pending | Needed for the DNS + WAF apply (which now also provisions the rate-limit — PR #2965). |
| 4 | **Vercel SPA** (`app.skycot.com` deployed) | ⬜ pending | The stranger signs up on the SPA (Vercel), separate from the AWS gateway. |
| 5 | **AWS credits** applied to `...0000` | ⬜ pending | Throwaway-test budget; the test itself costs a few dollars. |

## Prerequisite 1 — SES (CONFIRMED 2026-06-15)

Verified two independent ways: the operator's SES console dashboard **and** a
read-only `aws sesv2 get-account --region ap-southeast-2` (no email sent):

- **Production access:** ENABLED (out of the sandbox — can email arbitrary recipients, not just verified ones)
- **Sending enabled:** true · **EnforcementStatus:** HEALTHY
- **Daily sending quota:** 50,000 emails / 24h (sandbox would be 200)
- **Max send rate:** 14 emails / second
- **Region:** Asia Pacific (Sydney) — ap-southeast-2
- **Domain:** `skycot.com` — verified identity, **DKIM setup SUCCESS** (AWS Health event `AWS_SES_DKIM_PENDING_TO_VERIFIED`, 2026-06-12)

→ The signup → magic-link email path is production-ready for skycot.com. This was
the prerequisite with the ~24h approval lead, so it is the most important one to
have cleared before the apply.

## What's left before the apply
Prerequisites 2–5 above. The apply itself is operator-driven (it spends real money
+ enters real credentials); the conductor prepares config/secrets/image/plan and
co-pilots the run live. Cost: a few dollars for a few-hours test (fully covered by
the credits); ~$105–130/mo only if left running 24/7.
