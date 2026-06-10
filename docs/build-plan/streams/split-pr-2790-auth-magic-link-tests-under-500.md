# split-pr-2790-auth-magic-link-tests-under-500

Authored against: origin/main 70b01f9069745c02f8f7d2a564fae77e1ab5798e on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh split-pr-2790-auth-magic-link-tests-under-500

## Goal

Get `crates/argo-cli/src/handlers/gateway/auth_magic_link_tests.rs` back under
the 500-LOC hard ceiling on PR #2790's branch, so the perimeter verify-route
XFF-spoof re-land can merge. #2790's security LOGIC is adversarially-reviewed
CONFIRMED-FIXED — ONLY the file-size gate fails (505 LOC).

## Why now — HIGH PRIORITY

#2790 re-lands the verify-route throttle fix that #2780 silently DROPPED (a
stacked-squash drop). On main RIGHT NOW the verify route's 60/IP/min
token-brute-force throttle is STILL spoofable (`source_ip = signup_source_ip(req)`
at auth_magic_link_verify.rs:59). #2790 is the LAST of 6 perimeter limiters.
Its only blocker is that the 2 new spoof tests pushed auth_magic_link_tests.rs
to 505 LOC (gate-3 hard ceiling 500).

## How (pure mechanical relocation — touch no assertion)

Check out #2790's branch (`claude-fix-verify-route-xff-spoof-reland`), rebase on
current main. MOVE the 2 new verify-throttle test functions
(`verify_throttle_keys_on_trusted_src_ip_not_spoofable_xff` +
`verify_distinct_trusted_src_ips_get_separate_buckets`) out of
`auth_magic_link_tests.rs` into a NEW sibling test module
`auth_magic_link_verify_throttle_tests.rs` (wire it with `#[path=...] mod ...;` or
the crate's existing test-module convention; copy only the imports those 2 fns
need). This drops the original file under 500. Touch NOTHING else — the moved
functions' bodies must be byte-identical. Force-push to #2790's branch.

## Acceptance / verifier

1. `wc -l auth_magic_link_tests.rs` < 500; the new sibling file also < 500.
2. `cargo test -p argo-cli auth_magic_link` green — both verify-throttle tests
   still run + PASS.
3. `git diff` of the moved function bodies == empty (pure relocation).

## Size / owner

~mechanical move. Owner: claude. 🔴 P1 — unblocks the perimeter's LAST limiter
(verify-route re-land of a previously-DROPPED security fix); WS5.
