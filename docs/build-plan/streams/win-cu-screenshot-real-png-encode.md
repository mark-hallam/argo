# win-cu-screenshot-real-png-encode

**Owner**: windows-fleet — **P2 — real screenshot encoding (CU-executor follow-up).**
**Depends on**: win-computer-use-executor-first-build MERGED (#2906).
**Authored against**: origin/main `551aa830d` on `2026-06-13`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows, push branch, IN_REVIEW; conductor reviews+merges.

## Problem (from #2910 smoke evidence, NO THEATER)
The Windows CU executor's screenshot path runs end-to-end (monitor enumeration,
DC creation, BitBlt, real dimensions captured: 1920x1080 primary) BUT
`encode_bitmap_as_png` is a STUB — it emits a 16-byte payload (8-byte PNG
signature + 8 bytes of dims), not encoded pixels. So screenshots have correct
metadata but no real image. The input primitives (move/click/type/keys) are
proven working (#2910 clipboard read-back).

## Fix
Implement real PNG encoding of the captured BitBlt buffer in
`crates/argo-executor-computer-use-windows` (the `encode_bitmap_as_png` path).
Use the `image` crate (already a workspace dep — grep) or GDI+ to encode the
DIB bytes to a real PNG. Match the macOS/Linux executor's screenshot output
contract (same CuAction screenshot return shape). Keep it behind the existing
target_os=windows cfg.

## Verify (on real Windows)
`set ARGO_NATIVE_WINDOWS_INTEGRATION=1` + the windows_smoke harness (#2910):
the screenshot primitive now returns real_pixels=true with a decodable PNG of
the captured monitor (assert PNG magic + non-trivial byte count + decodes to
the reported dimensions). Append the result to CU-EXECUTOR-SMOKE.md. Branch + IN_REVIEW.
