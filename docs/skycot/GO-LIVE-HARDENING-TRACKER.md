# SKYCOT go-live hardening tracker

**The single canonical checklist of everything outstanding before public launch.**
Created 2026-06-23 so the future items from DEC-061 (origin lockdown) and the
2026-06-23 verification sweep can't be missed. Update the status column in-band as
items land; this file is the source of truth for "what's left."

Legend: 🔴 do before *any* public traffic · 🟠 before public *launch* · 🟡 later / HA ·
✅ done · 👤 operator-gated (config/secret/registration the conductor can't run) ·
🤖 conductor-doable.

---

## 🔴 Origin security (DEC-061) — the live-verified critical gap

The AWS origin is reachable directly from the internet, so Cloudflare's WAF /
rate-limit / bot rules are bypassable. Staged three-rung ladder:

| # | Item | Owner | Status | Unblocks / notes |
|---|------|-------|--------|------------------|
| A | **Lock ALB SG to Cloudflare CIDRs** (network-layer IP allowlist) | 👤 done | ✅ **DONE + VERIFIED LIVE 2026-06-23** | SG now Cloudflare-only (15 IPv4 + 7 IPv6 on 80/443, no `0.0.0.0/0`). Verified: direct hit to the raw ALB DNS **times out** (bypass closed); `api.skycot.com` via Cloudflare = HTTP 200, `server: cloudflare`. WAF/rate-limit/bot now bypass-proof. |
| B0 | **SSL `Full (strict)`** (validate the origin cert) | 👤 done | ✅ **DONE + VERIFIED 2026-06-23** | Operator set Cloudflare SSL/TLS → Full (strict). Verified: api.skycot.com/health → HTTP 200 through Cloudflare (origin cert validates, no breakage). Origin leg now encrypted + cert-verified. |
| B1 | **Cloudflare secret-header origin gate** (operator chose this over full mTLS 2026-06-23) | 👤 done | ✅ **DONE + VERIFIED LIVE 2026-06-23** | LIVE: Cloudflare Transform Rule stamps `X-Skycot-Origin-Auth` (= secret) on all requests; ALB listener priority-10 rule forwards secret-bearing traffic, **default action = 403** for anything without it. Verified: self-test MATCH; api.skycot.com/health + / → 200, app.skycot.com → 307 (SPA fine), no legit traffic 403'd. Closes the "shared Cloudflare IP" gap the IP-lock can't. **Chose over mTLS because rung C (Tunnel) is the durable end-state.** Root cause of the long debug: the CF rule defaulted to "Custom filter expression" (matched nothing) instead of "All incoming requests". Scripts in `skycot-apply/scripts/origin-secret-*.sh`. |
| B1-r | **Rotate the origin secret** (it appeared in the debug transcript) | 👤+🤖 | ✅ **DONE + VERIFIED 2026-06-23** | Zero-downtime rotation complete (add new-secret rule → swap CF value → self-verify NEWMATCH → remove old rule). Live secret is now fresh (never printed); ALB has only the new forward rule + default 403; health 200. The transcript-exposed value is no longer live. |
| B2 | ~~Authenticated Origin Pulls / mutual-TLS~~ | — | ⏸️ deferred | Cryptographic gold standard, but discarded at the Tunnel (C) — not worth the cert/trust-store upkeep + outage risk now. Revisit only if the public ALB must live long-term before C. Full research in session notes. |
| C | **Cloudflare Tunnel** (no public origin at all) | 🤖+👤 at HA rebuild | 🟡 with the Postgres/multi-task move | Strongest posture; biggest change. Do when re-architecting for HA anyway. |
| — | **Turn on "Always Use HTTPS"** (Cloudflare → Edge Certificates) | 👤 | 🟠 | Cloudflare logged 279 plain-HTTP req/24h. |
| — | **DMARC enforce** (currently `p=none` monitor-only) | 👤 | 🟡 Low | Verified 2026-06-23: `_dmarc.skycot.com` = `v=DMARC1; p=none` (reporting only, NO anti-spoof enforcement). SPF present (`~all` soft-fail). Move `p=none` → `quarantine`/`reject` after confirming DKIM alignment (Google DKIM selector returned empty — verify SES/Google DKIM is published). |
| — | **Upgrade §12 gate to check LIVE AWS**, not just the Terraform plan | 🤖 | 🟠 | The gate missed the live divergence because it only reads the plan. |
| — | **Reconcile Terraform worktree drift** — COMMIT the em-dash→hyphen edit | 🤖 | 🟠 before any full apply | State + live already hold hyphens; committed HEAD's em-dash is a latent force-replace landmine. Disarm by committing, NOT reverting. |
| — | **Annotate tracked `terraform.tfvars.example`** with secure-default reminder | 🤖 | 🟡 | `*.tfvars` is gitignored; defaults already secure, so belt-and-suspenders. |
| — | *(optional)* Delete stale Route53 `api.skycot.com`→raw-ALB ALIAS | 👤 | 🟡 | Non-authoritative, harmless, but confusing. |
| **SPA** | **Lock the App Runner SPA origin** (`app.skycot.com`) | 🤖+👤 done | ✅ **DONE + VERIFIED LIVE 2026-06-23** | Closed via an AWS WAFv2 web ACL (default-Block + Allow only Cloudflare's 15 IPv4 + 7 IPv6 CIDRs) associated to App Runner `skycot-prod-web` (`scripts/lock-apprunner-spa-origin.sh`; CloudWatch metrics off for free-tier; no code change / no redeploy). Verified: raw `example.ap-southeast-2.awsapprunner.com` → **403** (bypass closed); `app.skycot.com` /, /landing, /pricing via Cloudflare → **200** (legit traffic unaffected); service still RUNNING. **Both tiers (gateway + SPA) now Cloudflare-only.** |

---

## 🟠 Other operator-gated config (verified-real gaps, 2026-06-23)

These are **config/secrets/registration**, not code — the code paths are built and
verified; they're inert until the operator wires the external service.

| Item | Owner | Status | Notes |
|------|-------|--------|-------|
| **OAuth provider apps** (Gmail/GCal/etc. → 5 connectors return `not_configured`) | 👤 | 🟠 | Register apps + client IDs/secrets. Gmail/GCal need Google verification (lead time). Code path `/connect` #3026 verified real. |
| **Stripe overage metered price + cron** (`STRIPE_PRICE_PRO_OVERAGE` unset) | 👤+🤖 | 🟠 | Base subscription already wired (keys+webhook+PRO price live). Only the per-unit overage is off until the price id is set and the overage cron is `terraform apply`-d. |
| **Voice (OpenAI key)** — STT/TTS are MOCK in prod | 👤 | 🟡 decision | No `OPENAI_API_KEY`/`ARGO_VOICE_PROVIDER`. Pitch *text* is real (Anthropic); speech is faked. Wire only if real voice is wanted for the demo. |
| **Named runtime autonomy floor** (money-out / irreversible) | 🤖 | ✅ **IMPLEMENTED + cargo-verified 2026-06-23** | The deterministic `is_irreversible_action` deny-list (money / destructive / egress / credential) is now wired as a **manifest-independent floor** into BOTH runtime gates: the duty autofire gate (a money-out duty never auto-fires) and the dispatch fence (a money-out tool call is held, cleared only by an explicit+consumed operator grant) — above the manifest risk resolver, so a mis-stamped "low-risk" wire is still held. Adversarial test passes (money-out held even when the manifest "approves" it). FOLLOW-UPS (not blocking): a distinct audit label for floor-holds (v1 reuses `EscalationRequired`), and the stateful **first-time external recipient** floor (needs recipient tracking). |

---

## 🟡 HA / scale (post-launch)

| Item | Owner | Status | Notes |
|------|-------|--------|-------|
| **DB → Postgres for `desiredCount > 1`** | 🤖 | 🟡 | Single-writer SQLite can't run multi-task (split-brain). Required before true HA + Cloudflare Tunnel (C above). |
| **Gateway right-size already applied** | — | ✅ | task-def `:18`, cpu 2048 / mem 4096, commit 7f9aef8a9. Mitigates the flap-under-load risk; not full HA. |

---

## ✅ Done this session (2026-06-23)

- Gateway right-sized + PR-4 (spend-read fail-close) deployed — task-def `:18`, /health DB-intact.
- SPA redeployed from main (App Runner, zero-downtime) — autonomy 4-rung selector + OAuth /connect surface live.
- DEC-061 authored; tfvars set to secure default; `lock-alb-origin.sh` written + adversarially reviewed SAFE.
- **✅ ORIGIN LOCKED + VERIFIED LIVE (DEC-061 Step A)** — ALB SG now Cloudflare-only; raw-ALB direct hit times out, Cloudflare path HTTP 200. The #1 verified-real critical security gap is CLOSED.
- **✅ RUNG B DONE + VERIFIED LIVE** — B0: Cloudflare SSL `Full (strict)` (origin cert validated). B1: secret-header origin gate live (CF stamps `X-Skycot-Origin-Auth`, ALB default→403). Origin now has 3 layers: IP-allowlist + Full-strict + per-zone secret. Follow-up: rotate the secret (shown in debug transcript).
- FALSE blockers corrected: SES magic-link email **is** wired in prod (SES Sydney SMTP); Stripe base subscription **is** wired. (Agents had read IaC template defaults, not the live task-def env.)
