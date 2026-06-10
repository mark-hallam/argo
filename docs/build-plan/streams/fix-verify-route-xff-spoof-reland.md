# fix-verify-route-xff-spoof-reland

Authored against: origin/main (stamp real SHA at claim — `git rev-parse origin/main`) on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-verify-route-xff-spoof-reland

## Goal

RE-LAND the magic-link verify-route XFF-spoof fix that PR #2780 was supposed to
ship but which did NOT integrate into main (stacked-squash-merge drop). The
verify route's 60/IP/min token-brute-force throttle is STILL spoofable on main.

## Why now (conductor /loop discovery 2026-06-10 — NO-THEATER correction)

PR #2780 ("verify route keys per-IP throttle on trusted src_ip + retire
signup_source_ip") shows MERGED, but its merge commit `12fea8c31` is NOT
reachable from origin/main — it was stacked on #2776 and the squash-merge after
#2776 landed dropped #2780's delta. Ground truth on origin/main:
- `auth_magic_link_verify.rs:59` = `let source_ip = signup_source_ip(req);`
  (the SPOOFABLE helper — `parse_first_xff_hop`, no peer-trust check).
- `signup_source_ip` (`auth_magic_link_audit.rs:14`) still exists + is imported
  by `auth_magic_link.rs:46` and `auth_magic_link_verify.rs:2`.
The 4 form routes (signup/waitlist/enterprise/public-pitch) DID land on trusted
src_ip via #2776 — only the verify route + helper-retirement is missing.

The original (correct) fix exists in commit `12fea8c31` for reference — it
threads the trusted `src_ip` from the accept loop through
`serve_verify_stream` → `verify_outcome` and sets `let source_ip = src_ip;`.

## Fix (re-implement against CURRENT main)

1. Thread the trusted `src_ip` (already extracted at `boot_router.rs` accept
   loop, used for the signup route) into `serve_verify_stream` → `verify_outcome`
   (`auth_magic_link_verify.rs`), replacing `let source_ip = signup_source_ip(req)`
   at line 59 with `let source_ip = src_ip;`. Mirror EXACTLY how the merged
   signup route (`auth_magic_link_signup.rs`, `rate_limit_ip::signup_limiter().check(src_ip)`)
   already does it.
2. RETIRE `signup_source_ip` (`auth_magic_link_audit.rs:14`) + drop its imports
   (`auth_magic_link.rs:46`, `auth_magic_link_verify.rs:2`) once no caller
   remains. `rg 'signup_source_ip|parse_first_xff_hop' crates/` → no rate-limiter
   caller (the trust_xff-gated `extract_ip` path in rate_limit_ip.rs is the
   legitimate one and stays).
3. Do NOT re-stack on another PR — author against current main as a STANDALONE.

## Acceptance / verifier

1. `git show HEAD:crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs`
   line ~59 uses `src_ip`, NOT `signup_source_ip(req)`.
2. Spoof test (mirror the signup route's): same trusted src_ip, 2 different
   forged XFF → 2nd request throttled (forged XFF ignored).
3. `rg 'signup_source_ip'` → 0 non-test callers (helper retired).
4. `cargo test -p argo-cli` magic-link modules green.

## Size / owner

~40-80 LOC incl. test. Owner: claude. 🔴 P1 SECURITY — re-lands the verify-route
fix that #2780 failed to integrate; DEC-034 WS5. Standalone (no stacking).
