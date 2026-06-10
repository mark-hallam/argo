# fix-signup-protection-xff-trust-and-default

Authored against: origin/main (stamp real SHA at claim — `git rev-parse origin/main`) on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-signup-protection-xff-trust-and-default

## Goal

Close the LAST independently XFF-spoofable per-IP cap the #2776+#2780 perimeter
sweep left behind, and fix the unsafe default that makes it spoofable.

## Why now (adversarial review of #2780, 2026-06-10 — out-of-scope residual)

The 5-route XFF-spoof class is closed (signup/waitlist/enterprise/public-pitch
via #2776; magic-link verify + helper retirement via #2780). But the magic-link
SIGNUP route has a SECOND limiter the sweep didn't cover:
- `signup_protection::check_signup` (`auth_magic_link_signup.rs:60`) →
  `check_ip` → `signup_ip_key(req, trust_proxy_header)` →
  unconditional `parse_first_xff_hop` with NO peer-trust check
  (`signup_protection.rs:181-187`).
- `trust_proxy_header` **defaults to `true`** (`argo-config/src/sections_auth.rs:30`)
  — so out of the box this hourly cap trusts the client's first XFF hop.

Severity P2 (not P1): signup is fronted by the now-spoof-proof
`signup_limiter().check(src_ip)` (`auth_magic_link_signup.rs:32`), so signup is
NOT wide open — but `signup_protection`'s independent hourly cap remains
spoofable, and the `true` default is a footgun for any other consumer.

## Fix

1. Migrate `signup_protection::check_ip` to key on the trusted `src_ip` the
   accept loop extracts (the same `src_ip` #2776/#2780 thread through), OR gate
   its XFF trust on a peer-verified flag via `proxy_guard` (honor XFF only from
   a loopback/trusted peer) — consolidate on the established trusted-src_ip
   path, do NOT add a parallel extraction.
2. Reconsider the `trust_proxy_header` default: flip to `false` (secure by
   default; opt-in for nginx-fronted deploys) unless an operator decision keeps
   it — flag for operator if changing the default has deploy implications.

## May touch

- `crates/argo-cli/src/handlers/gateway/signup_protection.rs`
- `crates/argo-cli/src/handlers/gateway/auth_magic_link_signup.rs` (thread src_ip)
- `crates/argo-config/src/sections_auth.rs` (the default — operator-flag if changed)
- their `_tests` siblings

## Acceptance / verifier

1. Spoof test: same trusted src_ip, two different forged XFF → both hit the same
   `signup_protection` bucket (forged XFF ignored).
2. `rg 'parse_first_xff_hop' crates/` shows no rate-limiter caller bypassing the
   trusted-peer check.
3. `cargo test -p argo-cli` signup/protection modules green.

## Size / owner

~30-60 LOC incl. test. Owner: claude. 🟡 P2 SECURITY — completes the perimeter
XFF-spoof sweep; DEC-034 WS5.
