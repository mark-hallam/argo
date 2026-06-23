# SKYCOT verification — 2026-06-23 (code + live + AWS)

Method: 5 parallel skeptic agents verified each capability domain against the **actual source** (main `1dc654dd3`), **live prod** (gateway `5b7d609e1`, SPA `web-624d30954`), and **live AWS** (read-only describe). NO THEATER — "live" = code path + live signal; gaps are evidence-backed. Operator-facing summary: `SKYCOT-VERIFIED-STATE-2026-06-23.html` (supersedes the 06-21/06-22 strategy/readiness HTMLs).

## 🔴 Two issues surfaced live this session (fix first)
1. **ALB origin wide open** — SG `sg-0example000000000` ingress = `0.0.0.0/0` + `::/0` on 80/443; ALB DNS directly reachable → **Cloudflare WAF bypassable**. Contradicts the terraform secure default + the §12 go-live gate (a bring-up escape hatch never reverted). Fix: lock SG to Cloudflare CIDRs + run `scripts/check-alb-origin-locked.sh`. **#1 before public traffic.**
2. **Gateway flaps under load** — single 2 GB / 1 vCPU task, `desiredCount 1`. The 5 audit agents + a Genesis LLM stream → "failed ELB health checks" 09:35 → ~90s 503 → restart → self-recovered. Real availability risk. Fix: bump cpu/mem; for `desiredCount >1` move DB off single-writer SQLite first (split-brain).

## ✅ Verified LIVE + real
Genesis SSE first-hire (real LLM, `ARGO_GENESIS_LLM_CLASSIFIER=1`+anthropic, 200 text/event-stream) · magic-link auth + **JWT-derived tenant** (cwd-vs-JWT bug fixed) · OAuth `/connect` route #3026 (302, discriminating, inert-until-configured) · 17 token connectors (binaries in image + per-tenant cred injection at spawn, real HTTP) · `file_list` #3031 (in builtin_tools count=17 + Genesis safe baseline) · audit hash-chain + verify/export/cert (real SHA-256, tenant-scoped, truth-bounded) · Stripe code (real HTTP, webhook on `customer.subscription.created`, real $-overage in cents) · capability fence (deny-by-default; terminal/CU/connectors denied at birth) · durable DB (fail-closed S3 restore, exit 70/71) · strong security headers + clean 401 JSON · multi-tenancy plumbed + cross-tenant negative tests.

## 💤 Built + deployed but DORMANT
**DEC-060 autonomy ladder** — live in the gateway, inert behind a triple-OFF lock (global `ARGO_DUTY_AUTOFIRE` unset → ClosedGate; per-tenant opt-in off; per-teammate default Always-Ask), all fail-closed. Nothing auto-fires for any customer. **Caveat:** the "permanent floor" (money-out / first-recipient / irreversible) is enforced INDIRECTLY via manifest risk-tiering — there is NO dedicated predicate; it holds only if manifest actions are correctly risk-stamped. Add a named floor backstop before flipping autonomy on.

## ⚠️ CORRECTION (2026-06-23, verified against the LIVE task-def :17 env/secrets)
Two of the agents' "operator-gated" findings were WRONG — they checked the IaC TEMPLATE default (`terraform.tfvars.example` / `email_outbound={}`), NOT the live deployed env. Corrected:
- **Magic-link email is WIRED in prod.** Live task-def has `SKYCOT_EMAIL_PROVIDER=smtp` + `SKYCOT_SMTP_HOST=email-smtp.ap-southeast-2.amazonaws.com` (AWS SES, Sydney) + port 587 + username + `SKYCOT_SMTP_PASSWORD` secret + `SKYCOT_EMAIL_FROM_ADDRESS=noreply@skycot.com`. Operator confirms links send. **NOT a blocker.**
- **Stripe base subscription is WIRED.** `STRIPE_SECRET_KEY` + `STRIPE_WEBHOOK_SECRET` are set secrets in the live task-def; the SPA bakes a real PRO price id. Only the OVERAGE metered price (`STRIPE_PRICE_PRO_OVERAGE`) + the overage cron are genuinely unset. (Stripe-side product/webhook-endpoint registration: operator-to-confirm, not AWS-verifiable.)

## 🟠 Operator-gated (config, not code)
- **OAuth connectors gated to zero** — all 5 return `not_configured` (no provider client IDs). VERIFIED real. (register apps + keys; Gmail/GCal need Google verification)
- **Overage metered billing** — base sub works; the per-unit overage is off until `STRIPE_PRICE_PRO_OVERAGE` is set + the cron terraform-applied. VERIFIED real.
- **Voice = MOCK in prod** — no `OPENAI_API_KEY`/`ARGO_VOICE_PROVIDER`; STT/TTS are fakes, realtime 503s. "Voice live on /pitch" FALSE; pitch *text* is real (Anthropic). VERIFIED real.

## 🟦 Deploy freshness
- **SPA 9 commits behind main** (`web-624d30954`) — missing the autonomy 4-rung selector UI (#3030) + OAuth /connect SPA surface (#3026). Audit-verify button (#3021) IS deployed. → SPA redeploy.
- **Gateway 1 commit behind** — PR-4 #3034 (spend-read fail-close) undeployed (moot while autonomy off).
- **Every gateway deploy = ~90s+ outage** (single-writer SQLite, replace-style).

## ⚪ Honest corrections
- Channels = approval-callback webhooks (real sig verify), NOT inbound chat — don't market as two-way messaging.
- Computer-use **dead-in-prod** (headless Fargate; macOS-only). Old preflight gate-bug IS fixed (stale memory note to correct). Path = downloadable local runner (v1.x).
- Image-gen deferred (stub backend, CLI-only) — confirmed.
- In-core tool count = **17** (CLAUDE.md invariant still says 15 — stale).

## Outstanding (prioritized — post-correction)
P0 OPERATOR: (1) **lock ALB → Cloudflare** (the ONE verified-real critical gap) · (2) register OAuth provider apps · (3) set `STRIPE_PRICE_PRO_OVERAGE` + apply the overage cron (base sub already wired).
P1 DONE/CODE: (4) ✅ SPA redeployed from main (App Runner, zero-downtime, web-4ba2ed98e) · (5) ✅ gateway right-sized + PR-4 deployed (task-def :18, cpu 2048/mem 4096, commit 7f9aef8a9 = main HEAD, /health verified DB-intact).
P2 CODE/DECISION: (6) named runtime autonomy floor (spawned task) before autofire · (7) wire voice (OpenAI) if real speech wanted · (8) for true HA (desiredCount>1) move DB → Postgres.
NOTE: SES email + Stripe base were FALSE blockers (agents read IaC defaults, not the live task-def) — both wired in prod.
