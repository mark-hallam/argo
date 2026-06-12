# pricing-enterprise-byok-provider-keys

**Owner**: claude or codex — **SECURITY-SENSITIVE (tenant credentials) → conductor adversarial review before merge.**
**Depends on**: launch-cred-file-write-lock MERGED (same file, locked writes first).
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-enterprise-byok-provider-keys

## Problem (DEC-053 Enterprise tier)

Enterprise = BYOK: the tenant's own Anthropic/OpenAI key bills their
provider account directly (zero token COGS). Today provider keys are
process-global env (`ANTHROPIC_API_KEY`) — no per-tenant key resolution
exists in the transport path.

## Fix

(1) Per-tenant provider-key storage: reuse the tenant-scoped credential
store (`cred::set_tenant`/`load_tenant` — tg2's hardened path; keys
`ANTHROPIC_API_KEY`/`OPENAI_API_KEY` under the tenant section; NEVER in
audit/logs/responses — mirror tg2's secret discipline). Owner-RBAC
gateway route to set/clear (`POST /api/v1/tenant/provider-keys`,
body-only secret, structured errors) + /settings surface (masked, set/
clear only, LOCKED BRAND). (2) Transport resolution: at the
`build_transport` seam, tenant key (if set) WINS over process env;
fail-closed per-tenant (a tenant whose own key is invalid gets an honest
provider error, never silently falls back to the platform key — that
fallback would un-do the margin model; make fallback an explicit
per-tenant config bit defaulting OFF for enterprise tier). (3) Spend
attribution still records usage (meter stays true) but SpendGate ceilings
do not bill BYOK usage.

## Acceptance
1. Tests: tenant with own key → transport uses it (mock provider asserts
   key); without → platform key; invalid tenant key → honest error, NO
   silent fallback when fallback bit off; secret never in response/audit
   (negative pins, tg2 pattern).
2. Cross-tenant: tenant A's key never used for tenant B (negative pin).
3. clippy + vitest + tsc clean.

~200-300 LOC. **🟠 P2 — enterprise margin mechanics; public-flip blocker
for the enterprise tier only.**
