# fix-public-rate-limit-xff-spoof

Authored against: origin/main c359fc5ca on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-public-rate-limit-xff-spoof

## Goal

Stop the per-IP rate limiters on the anonymous public routes from trusting a
client-controlled `X-Forwarded-For` (XFF) first hop. Today an attacker can
rotate the XFF header to defeat every anonymous abuse cap.

## Why now (security audit 2026-06-10, P1 — confirmed vulnerability)

- `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs:199-208`
  (`signup_limiter`) and `routes_pitch_ask_public.rs:84-94`
  (`public_pitch_limiter`) hardcode `trust_xff=true`.
- The IP comes from `auth_magic_link_audit.rs:14-16` `signup_source_ip` →
  `parse_first_xff_hop` (`rate_limit_ip.rs:227-243`) which takes the FIRST,
  client-supplied XFF hop with no trusted-peer check.
- Shared by: signup (5/h cap → unlimited magic-link emails + tenant
  provisioning), `routes_waitlist.rs:63`, `routes_enterprise_inquiry.rs:67`,
  and the public pitch route (LLM cost).
- The SAFE pattern already exists: `proxy_guard.rs:119-133` honors XFF only
  when the direct peer is loopback / in `trusted_proxies`.

## Fix

Route the public limiters' IP extraction through the trusted-peer logic
(`proxy_guard::extract_real_ip` or an equivalent shared helper): honor XFF
only from a trusted direct peer; otherwise use the socket/direct peer IP.
Do NOT add a parallel extraction helper — consolidate on `proxy_guard`'s.
Keep the limiter types/buckets unchanged.

## May touch

- `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs`
- `crates/argo-cli/src/handlers/gateway/auth_magic_link_audit.rs` (the
  `signup_source_ip` helper — re-point, keep signature if possible)
- `crates/argo-cli/src/handlers/gateway/proxy_guard.rs` (only if a small
  `pub(super)` re-export is needed)
- call sites: `routes_pitch_ask_public.rs`, `routes_waitlist.rs`,
  `routes_enterprise_inquiry.rs`, magic-link signup route
- their `_tests` siblings

## Acceptance / verifier

1. Unit tests: (a) spoofed XFF from an UNTRUSTED peer does NOT rotate the
   limiter bucket (two requests, different XFF, same socket peer → second is
   rate-limited once cap hit); (b) XFF from a trusted/loopback peer IS
   honored (preserves nginx-fronted deploys); (c) existing limiter tests
   still green.
2. `cargo test -p argo-cli rate_limit` + the 4 route test modules green.
3. Boundary note in PR if any deploy doc needs a `trusted_proxies` mention
   (deploy/nginx fronting → loopback peer is already trusted).

## Size / owner

~60-120 LOC including tests. Owner: claude. 🔴 P1 SECURITY — DEC-044
public-preview gate item (perimeter hardening).
