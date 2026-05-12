# Mission load-bearing wire — IP allowlist + VPN-only deployment mode

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #233. Enterprise tenants want a defense-in-depth IP gate at the gateway edge: only allow requests from operator-defined source CIDRs (typically corp VPN + office-static IPs). Today: bearer-token only.

## Scope

NEW middleware at the gateway dispatch entry:

1. **Config** `[security.ip_allowlist] enabled = true; cidrs = ["10.0.0.0/8", "192.168.1.0/24"]; trust_x_forwarded_for_hops = 1`
2. **CIDR check** runs before bearer-auth. Source IP derived from `X-Forwarded-For` after `trust_x_forwarded_for_hops` (so reverse-proxy deployments work).
3. **Deny → 403** with empty body (no body = no info-leak about what's gated)
4. **Allow → fall through** to existing bearer-auth gate
5. **Audit row** per deny (kind: `security.ip_allowlist_denied`, payload: `{ source_ip, requested_path, requested_method }`) — operator gets visibility into attempted-access patterns
6. **Bypass** in dev: `NODE_ENV != production` OR `[security.ip_allowlist] enabled = false` (default) skips entirely

This is NOT in scope: hostname-based allowlists (CIDR only); GeoIP filtering (separate brief — needs MaxMind dep); rate-limit per source IP (sibling brief); DDoS mitigation (operator's load balancer's job).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/ip_allowlist.rs` (NEW, ≤ 280 LOC) — middleware
- `crates/argo-cli/src/handlers/gateway/ip_allowlist_tests.rs` (NEW, sibling, ≤ 300 LOC) — 8 tests
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_*.rs` — narrowly scoped: ip_allowlist invocation at entry (before bearer-auth)
- `argo-config/src/sections.rs` — `[security.ip_allowlist]` section
- `docs/skycot/RUNBOOK.md` — "IP allowlist" subsection

## Files you must NOT touch

- The bearer-auth path itself (`routes_auth.rs`); the audit chain; other middleware

## Acceptance criteria

- [ ] CIDR check happens BEFORE bearer-auth (otherwise leaks 401 vs 403 distinction)
- [ ] `X-Forwarded-For` honored with `trust_x_forwarded_for_hops` clamp; default 0 (use direct peer IP)
- [ ] Deny returns 403 with empty body (no info-leak)
- [ ] Audit row per deny
- [ ] `enabled = false` (default) — middleware not invoked, zero overhead
- [ ] IPv4 + IPv6 both supported
- [ ] 8 unit tests covering: allow / deny / XFF-trust / IPv6 / malformed-CIDR-rejected / multiple-CIDRs / audit-row-shape / empty-allowlist-fails-closed-when-enabled
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-ip-allowlist-deployment-mode`
- PR title: `gateway: IP allowlist middleware (mission-ip-allowlist-deployment-mode)`

## Notes

- The XFF-hops setting is the load-bearing operator knob. Direct deployment (no proxy) → hops = 0. nginx-in-front → hops = 1. nginx-in-front + AWS-ELB → hops = 2. Document the math.
- "Fail closed" when `enabled = true && cidrs = []` — empty allowlist with the gate on means deny-all, not allow-all. Spell this out.
- The deny-with-empty-body pattern is the same as the cross-tenant audit response — 4xx codes leak less than 200-with-error-payload.
