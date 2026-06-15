# SKYCOT pre-launch security audit — 2026-06-15

**Method:** adversarial multi-lens Workflow (`wf_1ef00024-337`): 10 security lenses + 3 deep-dives (auth/tenant/payment) + completeness critic over the go-live surface on origin/main `06e4563d5`; 39 candidate critical/high findings each adversarially re-tested (refute-unless-exploitable), load-bearing claims spot-verified. 75 agents. Gates the production go-live (Fargate gateway + App Runner SPA + Route 53, front_with_cloudflare=false test).

---

All key claims verified. The Genesis findings are confirmed nuanced exactly as the refuting verdicts say: the comment at boot_router.rs:163 falsely claims "Bearer + principal validation happen inside the handler" (no such validation exists in the handler — only a provider-key check at line 143), but the per-IP limiter (line 130) DOES run before Genesis dispatch, and the `TransportGate::Anonymous` spend gate is present on the prod path. So it's a real cost/DoS surface but bounded — a medium-to-high follow-up, not a clean go-live blocker.

I have enough to write the truth-bounded synthesis. The picture is clear and consistent across my spot-checks: the candidate over-rated a large cluster of auth-bypass/open-ALB findings that the production profile + ALB gate genuinely neutralise, while a smaller core of unmitigated criticals (hardcoded secrets, cross-tenant session writes, audit-chain breakage, support-token leak) survive on any path.

---

# SKYCOT Pre-Launch Security Audit — Decision-Ready Synthesis

**Audit scope:** 39 candidate CRITICAL/HIGH findings (each re-tested adversarially on `origin/main` HEAD `06e4563d5`) plus 24 candidate medium/low findings (not individually re-verified). I spot-verified the load-bearing claims myself this turn against the live code — results below match the refuting verdicts.

---

## (1) VERDICT: **GO-WITH-FIXES**

This is **not a GO**, and it is **not a NO-GO**. The platform's production deployment path is genuinely well-defended — the `ARGO_GATEWAY_PROFILE=production` switch and the ALB-lock gate (both verified live this turn) correctly neutralise the *largest* cluster of scary-looking findings (auth-bypass, open-origin). The candidate audit over-rated roughly a dozen findings as "critical" that the production path makes operationally unreachable. Credit where due: those defenses are real and load-bearing.

**But** a smaller core of findings survive on *every* path — including a correctly-configured production deploy — and three of them break tenant isolation or leak a full account credential. Those are real go-live blockers. They are a finite, fixable list (4 must-fix items, all small code changes), which is why this is GO-WITH-FIXES and not NO-GO.

Plain-English bottom line for you: *the front door is locked properly, but there are four side windows left open. Close those four and you can ship.*

---

## (2) MUST-FIX-BEFORE-LAUNCH — verified go-live blockers

These survive even with production mode on and the ALB locked. I personally confirmed each in the code this turn.

### MF-1 — Hardcoded "dev" secrets protect audit downloads in production
- **Plain English:** The links that let someone download audit records and screenshots are protected by a password that is *printed in the public source code* — and nothing in the AWS setup replaces it. Anyone who reads the repo can forge a valid download link for any tenant's audit data.
- **Technical:** `secret_bytes()` falls back to `b"argo-dev-export-secret-do-not-use-in-prod"` when `ARGO_AUDIT_EXPORT_SECRET` is unset — `crates/argo-cli/src/handlers/gateway/routes_audit_export_signing.rs:12,66-71`. Same pattern for screenshots: `b"argo-dev-screenshot-secret-do-not-use-in-prod"` at `routes_audit_screenshot.rs:31`. **Verified this turn:** `startup_check.rs` enforces the JWT secret and base-URL in production but does **NOT** check these two; `rg` found **zero** references to either env var anywhere under `infra/` or `deploy/`. Export URLs have effectively unlimited renewable lifetime.
- **Fix sketch:** (a) add `ARGO_AUDIT_EXPORT_SECRET` + `ARGO_AUDIT_SCREENSHOT_SECRET` to Secrets Manager + the ECS task env (mirror the JWT-secret wiring), and (b) make `secret_bytes()` fail-closed in production mode — refuse to boot if unset — exactly as the JWT secret already does at `startup_check.rs:92`.

### MF-2 — Any signed-up user can write to *another* tenant's session approvals
- **Plain English:** The approve / revoke-approval buttons don't check that the session you're touching is actually *yours*. A second user can flip the approval switches on someone else's running AI teammate — including granting a tool that moves money — or read their whole approval policy.
- **Technical:** `routes_session_approval.rs` calls the unscoped `store.get_session(sid)` at lines **54, 133, 189** with no tenant check, while the sibling `routes_session_skills_lifecycle.rs` correctly calls `assert_session_belongs_to_tenant()` at lines 74/114/155/210 (verified this turn — the safe pattern exists right next door but wasn't applied here). Covers grant-inject, grant-revoke (cross-tenant DoS), and grant-read (policy exfiltration).
- **Fix sketch:** Add `assert_session_belongs_to_tenant(store, sid, raw_sid, &request_tenant_id(req)).await?` at the top of `session_approve_response`, `session_revoke_approval_response`, and `session_grants_get_response` — copy the exact 4-line guard from the skills-lifecycle file.

### MF-3 — A tenant rename silently destroys the tamper-evident audit chain
- **Plain English:** The "tamper-evident audit log" is one of your four headline differentiators. The tenant-rename tool edits audit records directly in the database without re-stamping their security fingerprint — so the moment an operator ever renames a tenant, the audit chain fails verification and becomes forensically useless. Worse, a malicious operator can alter audit payloads (billing amounts, hire dates) and the chain just looks "broken" rather than "tampered."
- **Technical:** `crates/argo-store/src/tenant_query_inspect.rs:122-134` runs `UPDATE audit_record SET payload_json = replace(...)` without recomputing `self_hash` (which is `sha256(parent_hash || kind || 0x00 || payload_json)`). Verification at `crates/argo-audit/src/verify.rs:175-182` then fails on every touched row. Verified this turn — no guard, no recompute, no test coverage.
- **Fix sketch:** Make `rename_tenant` re-chain audit rows after rewriting payloads (recompute `self_hash` and relink), **or** refuse to touch `audit_record` and require a documented offline re-chain procedure. Given the differentiator is "audit you can trust," the principled fix is the re-chain, not a carve-out. Until fixed, gate `rename_tenant` behind an explicit operator flag and add a verify-after-rename check.

### MF-4 — Every support ticket leaks the submitter's full gateway token
- **Plain English:** When a user submits a support ticket, the message we send out includes that user's full login token. If the support webhook is ever misconfigured, hijacked, or compromised, the attacker gets a working key to every ticket-submitter's account.
- **Technical:** `web/app/api/support/ticket/route.ts:82` puts `gateway_token: session?.gatewayToken` into the outbound payload (verified this turn). This was already flagged as P2-2 in the 2026-05-20 audit and never fixed.
- **Fix sketch:** Remove `gateway_token` from the payload entirely — send an opaque `ticket_id` and resolve user context server-side. One-line deletion plus a server-side lookup. (Note: `SUPPORT_WEBHOOK_URL` is not configured in the current AWS deploy, so the *immediate* live risk is lower — but this is a dangerous default that must not ship in the codebase before any operator wires a webhook. Fix it now; it's trivial.)

---

## (3) CONFIRMED NON-BLOCKERS (verified real, but safe for this controlled test)

These are genuine findings the candidate correctly identified — but the production path neutralises them. **Verified this turn:** `ARGO_GATEWAY_PROFILE=production` IS set in the ECS task (`infra/terraform/modules/ecs/main.tf:92`), which forces `auth_bypass=false` and returns 401 on a missing bearer token (`routes_auth.rs:60-79`); and `scripts/check-alb-origin-locked.sh` inspects the *resolved* ALB ingress and hard-fails on `0.0.0.0/0`, ignoring the bring-up escape hatch.

- **No edge WAF / rate-limit on the open ALB origin (Route53 test).** Real, and **accepted for a controlled test** — exactly as you framed it. The secure default (`front_with_cloudflare=true`) plus the Terraform precondition plus the pre-deploy gate all prevent a forgotten open origin reaching real production. **Action: add Cloudflare in front before any public launch.** This is the one to keep on the radar.
- **SPA bearer-token validation calls the public `/readyz`** (`web/auth.config.ts:57`) — a real design smell (a junk token passes the SPA's check), but the gateway's real bearer gate rejects it on protected routes, and production mode forces it off. → MEDIUM, post-launch.
- **`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` inlined into client bundles** — only dangerous if someone runs a production build with the flag set AND deploys it; the runtime middleware is `NODE_ENV==="production"`-gated and the Dockerfile sets that. → Real defense-in-depth gap (LOW), but **add a CI guard that refuses a prod build with this var set** — it's the cheap insurance against one operator typo.
- **The whole "Genesis is unauthenticated" + "auth bypass on signup/onboarding" cluster** — production mode + bearer gate neutralise the auth-bypass half. (The Genesis *cost-abuse* half is genuinely unfixed — see below.)
- **7-day session JWT + no sliding refresh + bearer-token replay** — long TTL is real, but a stolen token is killable: the `revoked_tokens` table is checked on **every** request (`middleware_session_verify.rs`). The residual risk is "user must notice and log out." → Tune TTL down + add token-binding post-launch; not a blocker.
- **CSRF exemption on `/auth/verify`** — real, but the one-time, 15-min, cryptographically-random magic-link token is the actual security boundary. → Defense-in-depth, post-launch.
- **Stripe price-id not whitelisted + unknown-price defaults to Pro** — real architectural gap, but currently only the Pro price exists (no Enterprise to escalate to) and exploitation needs Stripe-dashboard access or operator misconfiguration. → **Add server-side price-id allowlist + set tier metadata at checkout before activating multi-tier pricing.** Important, not blocking *this* launch.

---

## (4) GENUINE FOLLOW-UPS — real but bounded (do soon, post-launch)

Verified real this turn, just not clean blockers:

- **Genesis LLM cost-abuse (unauth + bucket-splittable rate limit).** The handler has **no** auth check and the comment at `boot_router.rs:163` *falsely* claims it does — verified this turn (only a provider-key check at line 143). The per-IP limiter (line 130) and the `TransportGate::Anonymous` spend gate DO run, so it's bounded to ~240 req/min/IP and a daily anonymous spend cap on the prod path — but the per-tenant bucket is client-controllable (`?tenant=` splitting) and there's no per-request LLM timeout/concurrency cap. → **Fix the false comment, add a real auth/session check or tighten the anonymous spend cap, and wrap the LLM call in a `tokio::time::timeout`.** Watch the Anthropic bill in week one.
- **`X-Argo-Principal` header trusted on onboarding routes** without cross-checking the bearer token's claims (`routes_onboarding_helpers.rs:84-99`). A signed-up user who knows a victim's email can provision into the victim's workspace. The verdict pair split on severity; I'd treat it as a **fast-follow** (decode-and-compare the bearer claim before honoring the header) — bounded to onboarding provisioning, not arbitrary reads.
- **Webhook secret has no rotation/detection; tenant_id taken from webhook metadata without a Stripe API cross-check.** All gated behind compromise of `STRIPE_WEBHOOK_SECRET` (in Secrets Manager). → Add a subscription-existence check before applying tier changes; plan secret rotation.
- **WebSocket token in query string** (`web/lib/api/websocket.ts:9-11`) — real exposure-at-rest, but ALB access logging is off and tokens are revocable. → Move to a cookie/subprotocol auth post-launch.

---

## (5) WHAT WAS CHECKED — coverage lenses

The audit applied these adversarial lenses across the codebase, and I re-verified the load-bearing ones this turn:

1. **Auth & session security** — magic-link minting/verify, JWT TTL, revocation, token binding/replay, CSRF, bearer validation.
2. **Tenant isolation** — session mutation/read endpoints, principal-header trust, rate-limit bucket keying (`extract_tenant` is client-controlled).
3. **Payment & billing** — checkout price-id validation, webhook signature + tier resolution, replay/dedup, downgrade behavior, promo codes.
4. **Secrets management** — hardcoded fallbacks, env/Terraform provisioning, fail-closed startup checks.
5. **Audit-chain integrity** — hash-chain tamper-evidence under direct DB mutation (the headline differentiator).
6. **Perimeter / infra** — ALB open-origin vs Cloudflare, production-mode gating, the pre-deploy lock gate.
7. **Cost / DoS** — unmetered LLM endpoints, rate-limit coverage gaps, concurrency limits.
8. **Web/SPA build-time safety** — `NEXT_PUBLIC_` inlining, CORS, CSP, cookie attributes, outbound-webhook token leakage.

**Coverage honesty:** the 24 candidate medium/low items in section 4's tail were *not* individually re-verified — they are a triage backlog for the first post-launch hardening sprint, not launch gates.

---

## Recommended path to GO

Fix **MF-1 through MF-4** (all small, localized changes — the safe patterns already exist in the codebase for MF-1 and MF-2). Then: confirm the ALB-lock gate passes on the final plan, and put **Cloudflare in front before any public/self-serve launch**. After that, you have a clean GO for the controlled test. The Genesis cost-cap and the price-id allowlist are the top two fast-follows for week one.

Files cited are all absolute under `~/code/argo/`.
---

## Resolution — 2026-06-15 (all 4 blockers FIXED + merged + macOS-gated green)

| Blocker | Fix | PR | Merge |
|---|---|---|---|
| MF-1 audit-export/screenshot dev-secret fallback | prod boot-gate (mirrors JWT) + secret_bytes refuses dev fallback in prod + 2 secrets wired into Secrets Manager/ECS | #2969 | `96a0ab31f` |
| MF-2 cross-tenant session-approval writes | `assert_session_belongs_to_tenant` guard on all 3 handlers (copied sibling pattern) | #2968 | `d8f214faf` |
| MF-3 tenant-rename breaks audit chain | `rename_tenant` refuses to modify `audit_record` (admin metadata never rewrites the immutable chain) | #2967 | `97fa7467e` |
| MF-4 support-ticket leaks gateway token | stripped `gateway_token` from the outbound payload | #2966 | `ffe62af0b` |

**Combined macOS gate (conductor re-verified, NO THEATER):** argo-cli 34/34, argo-audit 124+ /0, `tofu validate` ✅, web vitest 3/3. Each fix's regression test confirmed fail-before/pass-after. New required prod secrets `ARGO_AUDIT_EXPORT_SECRET` + `ARGO_AUDIT_SCREENSHOT_SECRET` staged.

**Security gate: 🟢 GO.** Post-launch fast-follows (Genesis LLM-cost cap, Stripe price-id allowlist, X-Argo-Principal onboarding trust, session-TTL tuning) tracked for week one. Standing: add Cloudflare before any public/self-serve launch.
