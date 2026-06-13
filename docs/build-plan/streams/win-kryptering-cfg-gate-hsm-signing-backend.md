# win-kryptering-cfg-gate-hsm-signing-backend

**Owner**: windows-fleet — **P1 WINDOWS BLOCKER — the only thing stopping the `argo` binary linking on Windows.**
**Depends on**: none.
**Authored against**: origin/main `68bce5fbd` on `2026-06-13`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows, push branch, IN_REVIEW; conductor reviews+merges. This touches the signing/crypto chain — conductor MUST re-verify the unix HSM path is unchanged before merge.

## Problem (from the #2900 Windows build inventory — VERIFIED on real Windows)
`cargo build --workspace` on Windows fails to compile `argo-cli` because its
dependency `kryptering` (PKCS#11/HSM signing) won't build on Windows:
`E0277 Ulong: From<u64> not satisfied` at kryptering pkcs11/mod.rs:147 + :610.
Root cause: PKCS#11 `CK_ULONG` is **32-bit on Windows** (so `cryptoki 0.7`
gives `From<u32>`, not `From<u64>`); kryptering casts to u64 then `.into()`,
which only resolves on unix. Dependency chain (cargo tree -i kryptering):
`argo-cli → bergshamra-dsig / bergshamra-keys → bergshamra-crypto → kryptering → cryptoki 0.7`.
argo-cli is the ONLY workspace crate that pulls it in. **This masks all of
argo-cli's own Windows errors** — nothing downstream type-checks until it builds.

## Fix (first-principles)
Feature-gate the PKCS#11/HSM signing backend so it is OFF on Windows (or behind
an off-by-default `hsm`/`pkcs11` feature), with a non-HSM signing path used on
Windows. Trace `bergshamra-crypto`'s use of `kryptering`: ideally
`#[cfg(not(target_os = "windows"))]` (or a feature) on the PKCS#11 module +
its re-exports, with the software-key signing path (already used in dev/demo)
as the Windows default. The HSM path stays the unix/enterprise default —
**do NOT remove or weaken it on unix** (it's the enterprise key-custody path).
NO THEATER: if Windows simply can't do HSM signing yet, the cfg(windows) branch
says so and falls back to the documented software-key path, not a fake HSM.

## Verify
- On Windows: `cargo build -p argo-cli` links; then `cargo build --workspace`
  reaches the next real errors (re-inventory argo-cli's own Windows errors,
  previously masked — append to BUILD-ERRORS.md).
- Conductor re-verifies on macOS/unix: the HSM/PKCS#11 signing path still
  compiles + its tests pass (bergshamra-dsig/keys/crypto), unchanged.
Push branch + IN_REVIEW.
