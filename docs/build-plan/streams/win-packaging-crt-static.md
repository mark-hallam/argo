# win-packaging-crt-static

**Fleet:** WINDOWS. **Priority:** P2. **Depends on:** none.
**Authored against:** origin/main `c7354f636` on 2026-06-14 (reconstructed from
the BUILD_STATUS_WINDOWS.md row + DISTRIBUTION.md §6/§8 — the original brief file
was queued without a stream doc; this file closes that gap).
**Validate before claim:** `bash scripts/check-brief-freshness.sh win-packaging-crt-static`

## Goal

Make the Windows `argo.exe` a **truly zero-prerequisite single binary**. Today
the release binary dynamically links `VCRUNTIME140.dll` (the VC++ 2015–2022
redistributable) — its one non-OS runtime dependency. A pristine Windows install
may lack it. Statically linking the CRT removes that import so the `.exe` runs on
a bare Windows box with no redistributable.

This was the follow-up explicitly deferred by PR #2922 / `DISTRIBUTION.md` §6
option 2.

## Change

Add a workspace-root `.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

**Unix is unaffected.** The stanza is keyed to the `x86_64-pc-windows-msvc`
target; cargo applies target-specific `rustflags` only when building for that
exact target, so Linux/macOS builds are byte-for-byte unchanged. (Caveat: a
`RUSTFLAGS` env var, if set, overrides config `rustflags` entirely — a Windows
distribution build must run without `RUSTFLAGS` set.)

## Verifier (CHECKABLE, NO THEATER)

1. A release `argo.exe` built with the flag has **NO `VCRUNTIME140.dll`** in its
   PE import table (PowerShell PE-import walk — `dumpbin` is not installed on the
   fleet host). Contrast against a non-crt-static binary, which DOES import it.
2. The release `argo.exe`, copied **alone** into an empty directory, runs
   (`argo --version` exit 0; `argo doctor` produces its report — a non-zero
   doctor exit from an uninitialized workspace is expected, not a load failure).
3. Conductor re-verifies `+crt-static` does not break the unix targets (it is
   MSVC-target-scoped, so it should be a no-op off-Windows).

## Out of scope

Building/publishing the release zip (the `release.yml` Windows leg is
CI-billing-disabled) and the GNU toolchain (`+crt-static` is inert on GNU CRT;
the MSVC toolchain is the v1 Windows target).
