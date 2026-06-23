# DEC-061 — Origin lockdown: the AWS origin only trusts Cloudflare

- **Status:** active — **Rungs A + B DONE + VERIFIED LIVE 2026-06-23.** A: ALB SG Cloudflare-only (raw-ALB direct hit times out). B0: Cloudflare SSL `Full (strict)`. B1: secret-header origin gate live (CF stamps `X-Skycot-Origin-Auth`, ALB default→403; legit traffic 200, SPA fine). Origin now triple-layered (IP-allowlist + Full-strict + per-zone secret). B1 secret rotated 2026-06-23 (zero-downtime) after it was shown in the debug transcript — live secret is now fresh. Outstanding: rung C (Tunnel, at HA rebuild) + Always-HTTPS + gate-checks-live-AWS upgrade + App Runner SPA origin — see GO-LIVE-HARDENING-TRACKER.md.
- **Locked:** 2026-06-23 (operator: "ok do it, and record decision and update so we do not miss doing the future items")
- **Supersedes/extends:** the IaC secure default already in `infra/terraform` (FEATURE_MATRIX row "AWS edge security — ALB origin locked to Cloudflare by default"). This ADR records that the **live prod origin diverged from that default** and the staged plan to close it durably.

## Plain-English summary

Cloudflare sits in front of our servers as a guard — it filters bad traffic (the
"WAF" = Web Application Firewall), blocks floods (DDoS), throttles abusers (rate
limits), and screens bots. **But a guard on the front door only helps if there is
no unlocked back door.** Today our AWS server (the load-balancer in front of the
gateway at `api.skycot.com`) is *also* reachable directly from the open internet.
Anyone who finds its address — and those are discoverable — walks straight past
every Cloudflare protection and hits our login, billing, and (soon) autonomy
directly.

**First principle: defense that can be bypassed is not defense.** The fix is to make
the server only accept traffic that actually came through Cloudflare. There is no
durable half-measure — either that is true or it isn't.

## What we verified (2026-06-23, browser + AWS + live)

| Layer | Finding | Source |
|---|---|---|
| DNS | api/app/skycot.com/www all proxied through Cloudflare (orange-cloud) | `curl` server headers + `dig` |
| SSL/TLS mode | **`Full`** — Cloudflare→origin leg encrypted, but origin cert **not validated** (one notch below `Full (strict)`) | Cloudflare dashboard, SSL/TLS Overview |
| Authenticated Origin Pulls (mutual-TLS) | **OFF** everywhere (global / zone / per-hostname); no origin cert installed | Cloudflare dashboard, SSL/TLS → Origin Server |
| Cloudflare WAF/DDoS/bot/rate-limit | Running; mitigating ~0.59% of 2.89k req/24h — **but only for traffic that comes through Cloudflare** | Cloudflare dashboard, Security Overview |
| AWS ALB security group `sg-0example000000000` | inbound 80+443 = **`0.0.0.0/0` + `::/0`** (open to the whole internet) | `aws ec2 describe-security-groups` |
| Live bypass | hitting the raw ALB with `Host: api.skycot.com` returns a real response → **Cloudflare is bypassable** | direct curl |

Root cause: the Terraform default is secure (`front_with_cloudflare=true`), but the
bring-up escape hatch (`front_with_cloudflare=false` + `allow_open_origin_bringup=true`
in `terraform.tfvars`) was set for the cold-stranger pre-DNS smoke and **never
reverted**. The §12 go-live gate (`scripts/check-alb-origin-locked.sh`) inspects the
Terraform *plan*, not live AWS, so it never caught the live divergence.

## Decision — the three-rung ladder, staged

The end-state must enforce "the origin only trusts my Cloudflare." Three mechanisms,
ordered by strength and effort. We do them in sequence — close the live door now,
add the cryptographic lock before public launch, consider eliminating the public
origin entirely at the HA rebuild.

**A — Network-layer lock (IP allowlist) — DO NOW.**
Restrict the ALB security group's inbound 80/443 to Cloudflare's ~22 published CIDR
ranges (15 IPv4 + 7 IPv6) instead of `0.0.0.0/0`. Minutes, no app changes, no certs;
closes ~99% of the risk today. Mild residual: those ranges are shared by all
Cloudflare customers (trust is "came from a Cloudflare IP", not "came from *my*
zone"), and the list changes rarely (Cloudflare announces ahead).

**B — Identity-layer lock (Authenticated Origin Pulls / mutual-TLS) — BEFORE PUBLIC LAUNCH.**
The origin demands a client certificate only *our* Cloudflare zone holds, and rejects
anything without it. Cryptographic proof, immune to IP-list drift. Pair with SSL mode
**`Full (strict)`** + a valid origin cert. A few hours of cert + ALB-listener config.
This is the durable end-state; A is the fast first layer, not a replacement.

**C — Eliminate the public origin (Cloudflare Tunnel) — AT THE HA REBUILD.**
Run an outbound-only connector so the origin has *zero* inbound public exposure — close
80/443 to the world entirely. The strongest posture, but the biggest change; belongs to
the moment we move the DB off single-writer SQLite to run more than one task (HA). Not
worth a special trip before then.

**Recommendation (first-principles, long-term):** B is the version we don't revisit;
A is the move that ends the live exposure today; C is the cleanest shape once we're
re-architecting for HA anyway. Defense-in-depth keeps A *and* B once both are in.

## What was done this session (Step A)

- `scripts/lock-alb-origin.sh` (in `skycot-apply`) — surgical, zero-traffic-gap lock:
  ADD the 15 IPv4 + 7 IPv6 Cloudflare ranges on 80/443, THEN revoke the open rules,
  then verify + print rollback. Ordering means real Cloudflare traffic never sees a
  closed door. **Adversarially reviewed by 3 independent skeptics (correctness /
  blast-radius / rollback) → SAFE, zero blocking issues.** CIDRs byte-identical to
  Cloudflare's live published list; health checks unaffected (service SG trusts the
  ALB SG by reference, not CIDR); quota 44/60; partial-failure fails OPEN (never a
  lockout).
- `terraform.tfvars` set to the secure default (`front_with_cloudflare=true`,
  `allow_open_origin_bringup=false`) to record durable intent. (NB: `*.tfvars` is
  gitignored — the variable *defaults* in `variables.tf` are already secure, so this
  is belt-and-suspenders; durability item: annotate the tracked `terraform.tfvars.example`.)

**Operator boundary:** locking a prod security group is a production access-control
change — per the conductor's guardrails, the operator runs the reviewed script; the
conductor prepared + adversarially verified it and verifies the result read-only.

## Method note — chose a surgical CLI lock over a Terraform apply

The `skycot-apply` worktree has uncommitted edits and the live task-def `:18` was hand
-created (cpu 2048/mem 4096). A full `terraform apply` would revert the task-def and
sweep unrelated edits. A *targeted* apply was possible, but the surgical
add-then-revoke CLI change has the smallest blast radius, trivial rollback, and
reconciles cleanly with Terraform later (live CIDRs == module defaults → no SG diff on
a future refresh). Recorded so the next session doesn't "re-discover" the choice.

## Named future items (do-not-miss → tracked in GO-LIVE-HARDENING-TRACKER.md)

1. ✅ **(A) `scripts/lock-alb-origin.sh` — DONE + VERIFIED LIVE 2026-06-23.** SG Cloudflare-only; raw-ALB direct hit times out, Cloudflare path 200.
2. **(B) Authenticated Origin Pulls + SSL `Full (strict)` + origin cert** — before public launch.
3. **(C) Cloudflare Tunnel** — at the HA rebuild (with the Postgres/multi-task move).
4. **Turn on "Always Use HTTPS"** — Cloudflare logged 279 plain-HTTP requests in 24h.
5. **Fix the DMARC record error** (Cloudflare Email Security insight, Low).
6. **Reconcile Terraform worktree drift** — COMMIT the em-dash→hyphen edit (state + live
   already hold hyphens; the *committed HEAD* em-dash is the latent force-replace
   landmine — disarm by committing, never by reverting). Then a clean full apply is safe.
7. **Annotate tracked `terraform.tfvars.example`** with the secure-default reminder.
8. *(optional)* Delete the stale Route53 `api.skycot.com`→raw-ALB ALIAS (non-authoritative,
   harmless, but confusing).
9. **App Runner SPA origin** (`app.skycot.com`) has its own separate bypass story — the
   ALB lock does NOT cover it. Track a parallel lockdown (App Runner ingress / Cloudflare).

## Consequences

- Once A is run: Cloudflare's WAF/DDoS/bot/rate-limit become *enforced*, not advisory,
  for the gateway. The gateway's per-IP limiter can trust `X-Forwarded-For` (non-spoofable
  because only Cloudflare can reach the origin).
- The §12 gate should be upgraded to also check **live AWS**, not just the plan, so a
  future live divergence is caught (tracked as a hardening item).
