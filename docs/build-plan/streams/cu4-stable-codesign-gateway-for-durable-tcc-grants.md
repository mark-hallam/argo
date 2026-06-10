# cu4-stable-codesign-gateway-for-durable-tcc-grants

**Owner**: claude (build/ops) — operator does the one-time grant.
**Depends on**: none (orthogonal; needed for any desktop-drive demo).
**Authored against**: origin/main `5f81ae776` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh cu4-stable-codesign-gateway-for-durable-tcc-grants

## Goal (DEC-050 G4)

Make macOS Accessibility + Screen-Recording grants to the gateway SURVIVE a
rebuild, so the desktop-drive demo isn't re-broken every time the binary is
recompiled.

## Why now (verified 2026-06-11)

`codesign -dv /tmp/argo-shared-target/release/argo` → `Signature=adhoc`
(linker-signed). macOS TCC keys ad-hoc binaries by code-hash; every rebuild
changes it, orphaning the 2026-05-26 grant. Confirmed: the 06:02 rebuild
orphaned perms.

## Fix

1. Sign the gateway binary with a STABLE identity (a self-signed dev cert in a
   keychain, or a Developer ID) in `restart-live-gateway.sh` /
   the release build, so cdhash/identity is stable across rebuilds:
   `codesign --force --sign "<stable-identity>" --identifier com.skycot.argo <bin>`.
2. Optionally wrap the executor's TCC-touching calls in a stable signed helper
   app bundle so the grant attaches to the bundle, not the rebuilt CLI.
3. Runbook: the one-time operator grant (System Settings → Privacy & Security →
   Accessibility + Screen Recording → add the signed binary), done ONCE.

## Acceptance / verifier

1. After a rebuild, `claim_5_live_macos_cu_smoke_test` PASSES (not skips)
   without a re-grant — proves the grant survived.
2. `codesign -dv` shows the stable identity (not adhoc).

## Size / owner / priority

~30-60 LOC (script) + a one-time operator grant. **🟠 P2 — DEC-050 G4.**
Gates every desktop-drive (cu3) demo's reliability.
