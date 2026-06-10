# fix-magic-link-verify-xff-spoof-and-retire-helper

Authored against: origin/main HEAD on 2026-06-10 (stamp the real SHA at claim — `git rev-parse origin/main`)
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-magic-link-verify-xff-spoof-and-retire-helper

## Goal

Close the LAST spoofable per-IP limiter that PR #2776 missed, and retire the
header-trusting helper entirely so the class cannot regrow.

## Why now (adversarial review of #2776, 2026-06-10, verdict INCOMPLETE)

PR #2776 fixed the spoofable X-Forwarded-For (XFF) rate-limit identity on 4
public routes (signup / waitlist / enterprise-inquiry / public-pitch), all
verified. But a 5th per-IP limiter sharing the same spoofable helper was
missed:

- `crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs:59`
  derives `source_ip = signup_source_ip(req)` =
  `parse_first_xff_hop(req)` (`auth_magic_link_audit.rs:13-16`) —
  unconditional, header-trusting — and feeds the dedicated **60
  attempts/IP/minute** verify limiter (`auth_magic_link_verify.rs:63`,
  `verify_rate_limit_check`).
- The verify route is the magic-link TOKEN BRUTE-FORCE surface, so this is a
  real abuse cap. An attacker rotates XFF to mint a fresh verify bucket per
  request, defeating the throttle.
- Partial mitigation (why this is P1 not P0): the global
  `ctx.ip_limiter.check(src_ip)` (`boot_router.rs:130`, default 120,
  spoof-proof) fires first, so a coarse real-IP cap survives — but the
  purpose-built verify throttle is defeated.
- The trusted, spoof-proof `src_ip` is ALREADY computed at the dispatch site
  (`boot_router.rs:117`, used :130) but is NOT threaded into
  `serve_verify_stream` / `verify_outcome` (`boot_router.rs:210`) — the same
  one-line thread #2776 applied to the other 4 routes was skipped here.

## Fix

1. Thread the trusted `src_ip` from `boot_router.rs:210` into
   `serve_verify_stream` → `verify_outcome`, replacing `signup_source_ip(req)`
   at `auth_magic_link_verify.rs:59` (mirror exactly how #2776 wired the
   other 4 routes — same `src_ip` value, computed once at the accept loop).
2. **Retire `signup_source_ip`** (`auth_magic_link_audit.rs:13-16`) and the
   underlying first-hop helper if it now has no remaining limiter caller —
   so no future code re-introduces the header-trusting path. If a non-limiter
   caller remains (e.g. an audit-only field), rename it loudly
   (`UNTRUSTED_xff_first_hop_for_audit_only`) with a doc comment, never feed
   it to a `check()`.
3. Grep-confirm: `rg 'signup_source_ip|parse_first_xff_hop' crates/` shows no
   remaining caller feeding a rate limiter.

## May touch

- `crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs`
- `crates/argo-cli/src/handlers/gateway/boot_router.rs` (thread src_ip)
- `crates/argo-cli/src/handlers/gateway/auth_magic_link_audit.rs` (retire/rename)
- their `_tests` siblings

## Acceptance / verifier

1. Spoof test mirroring #2776's `spoofed_xff_cannot_rotate_the_per_ip_bucket`
   but for the VERIFY route: same socket peer, two different forged XFF
   headers → the second request is throttled once the cap is hit.
2. Legit path: XFF from a trusted/loopback peer still honored (nginx-fronted
   deploy not collapsed into one bucket).
3. `rg 'signup_source_ip|parse_first_xff_hop'` → no rate-limiter caller.
4. `cargo test -p argo-cli` for the magic-link + rate-limit modules green.

## Size / owner

~30-60 LOC incl. test. Owner: claude. 🔴 P1 SECURITY — completes the #2776
perimeter-spoof class; DEC-034 WS5 perimeter surface. #2776 is HELD until this
lands so the class closes in one honest landing (no "perimeter fixed" badge
while a brute-force cap is still spoofable).
