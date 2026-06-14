# Windows distribution / packaging (`win-distribution-packaging-smoke`)

**Question:** how is `argo` distributed on Windows, and does the single-binary
architecture (a core invariant) actually hold there?

**Verdict: 🟢 single-binary holds on Windows.** A freshly-built release
`argo.exe` is one self-contained ~78 MB executable that runs from a clean
directory with **no co-located DLLs**. The Windows distribution artifact is a
**zip of that single `argo.exe`** (no MSI, no installer, no `cargo-dist`). One
runtime-dependency flag: the binary dynamically links `VCRUNTIME140.dll` (the
VC++ redistributable) — every other import is a Windows OS component. A fully
redistributable-free single file is one `+crt-static` flag away (§6).

NO THEATER: the binary smoked below was built **from current `origin/main`**
(commit `a19eaca`) on this Windows host — not a stale/borrowed binary.

---

## 1. Environment

| Field | Value |
|---|---|
| Host | `Latitude5440` — Windows 11 Pro 10.0.26200, x86_64 |
| Target triple | `x86_64-pc-windows-msvc` |
| Toolchain | `stable-x86_64-pc-windows-msvc`, rustc `1.96.0` |
| Binary commit | `a19eaca` (= `origin/main` at build time) |
| Build wall-clock | **53m 12s** (release `lto="fat"` + `codegen-units=1`, under sibling-build contention) |

## 2. Distribution path

There is **no MSI / NSIS / installer / `cargo-dist` / WiX** config in the repo
(`grep` for `metadata.dist`, `cargo-wix`, `.msi`, `crt-static` → none; no
`dist-workspace.toml`, no `wix/`). The distribution is the **raw single
binary**, matching the single-binary core invariant.

The canonical packaging recipe lives in
[`.github/workflows/release.yml.disabled`](../../../.github/workflows/release.yml.disabled)
(disabled because GitHub Actions billing is operator-blocked for v1.0 — it has
never run; see `DISABLED_WORKFLOWS_POLICY.md`). Its Windows leg:

```yaml
- { os: windows-latest, target: x86_64-pc-windows-msvc }
  run: cargo build --release --target x86_64-pc-windows-msvc --bin argo
- name: Package (windows)
  # Compress-Archive -Path target/x86_64-pc-windows-msvc/release/argo.exe
  #                  -DestinationPath argo-x86_64-pc-windows-msvc.zip
- uses: actions/upload-artifact@v4   # + Publish Release asset
```

→ the published Windows asset is **`argo-x86_64-pc-windows-msvc.zip`**, which
contains exactly one file: `argo.exe`. (Linux ships a `.tar.gz`; macOS a
`.dmg`.) When CI billing is restored, rename the workflow to `.yml` and it
produces this artifact on every tagged release.

## 3. Build steps (on a Windows host)

The host triple already *is* `x86_64-pc-windows-msvc`, so the `--target` flag is
optional on Windows:

```powershell
cargo build --release --bin argo
# → target\release\argo.exe   (one file; nothing else required to ship)
Compress-Archive -Path target\release\argo.exe -DestinationPath argo-x86_64-pc-windows-msvc.zip
```

The release profile ([root `Cargo.toml`](../../../Cargo.toml)) is
`lto = "fat"`, `codegen-units = 1`, `strip = "symbols"` — fully optimized and
symbol-stripped, which is why the binary is large-but-single.

## 4. The binary

| Property | Value |
|---|---|
| Path | `target\release\argo.exe` |
| Size | **81,619,456 bytes (~77.8 MiB)** |
| Kind | single PE32+ executable, symbols stripped |
| `argo --version` | `argo argo 0.0.1 (a19eaca 2026-06-14)` · `profile: release` · `features: default` |

## 5. Clean-directory run (the brief's core verify)

`argo.exe` was copied **alone** into an empty directory (only `argo.exe`
present) and executed from there:

```
$ ls argo-dist-clean/      →  argo.exe   (and nothing else)

$ ./argo.exe --version
argo argo 0.0.1 (a19eaca 2026-06-14)
  target:   x86_64-pc-windows-msvc
  rustc:    rustc 1.96.0 (ac68faa20 2026-05-25)
  profile:  release
  features: default

$ ./argo.exe --help
Argo is the agent framework that one engineer can fully audit on a weekend.
Usage: argo.exe [COMMAND]
Commands: agents  gateway  ...
```

✅ **PASS** — runs standalone with no co-located DLLs; the single binary is the
whole product.

## 6. Runtime dependencies (imported DLLs)

Parsed directly from the PE import table of the release `argo.exe` (no external
tooling — `dumpbin` isn't installed; a PowerShell PE-import walk was used):

| DLL | Class | Portable on a clean Win10/11? |
|---|---|---|
| **`VCRUNTIME140.dll`** | **VC++ 2015–2022 Redistributable** | ⚠️ **the one non-OS dep** — present on most systems (and on this host: `C:\Windows\System32\VCRUNTIME140.dll`, 124 KB), but a *pristine* Windows install may lack it |
| `api-ms-win-crt-*.dll` ×11 (`runtime`,`heap`,`stdio`,`string`,`math`,`locale`,`convert`,`time`,`environment`,`utility`) | Universal CRT (UCRT) api-sets → `ucrtbase.dll` | ✅ OS component since Win10 (`ucrtbase.dll` in System32) |
| `api-ms-win-core-synch-l1-2-0.dll`, `api-ms-win-shcore-scaling-l1-1-1.dll` | Win32 api-sets | ✅ OS |
| `kernel32`, `ntdll`, `advapi32`, `user32`, `ole32`, `oleaut32`, `ws2_32`, `crypt32`, `bcrypt`, `bcryptprimitives`, `wtsapi32` | core Win32 / crypto / sockets / RDP-session | ✅ all ship with Windows |

**Flag (the one to act on):** the binary is **not** statically-CRT-linked, so it
depends on `VCRUNTIME140.dll`. Two honest options for a truly self-contained
single file with zero redistributable assumption:

1. **Document the prerequisite** — ship the zip + a one-line note "requires the
   [VC++ 2015–2022 redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe),
   pre-installed on virtually all Windows 10/11 machines." (Lowest effort.)
2. **Static-CRT build — ✅ DONE (`win-packaging-crt-static`, 2026-06-14).** Added
   to workspace-root `.cargo/config.toml`:
   ```toml
   [target.x86_64-pc-windows-msvc]
   rustflags = ["-C", "target-feature=+crt-static"]
   ```
   This statically links the CRT, dropping the `VCRUNTIME140.dll` import so the
   `.exe` runs on a bare Windows install. Cost: a larger binary (~78 MB → ~82 MB)
   and a workspace-wide build-flag change — but MSVC-target-scoped, so unix is a
   no-op (conductor re-verifies). NB: a `RUSTFLAGS` env var, if set, overrides
   config `rustflags` entirely, so a distribution build must run without it.

   **Verified on the fleet host (Latitude5440, 2026-06-14, release build `c7354f6`):**
   a PowerShell PE-import walk of the crt-static release `argo.exe` shows **NO
   `VCRUNTIME140.dll`** AND no `api-ms-win-crt-*` UCRT api-set imports (both now
   statically linked) — only OS-guaranteed DLLs remain (`kernel32`, `ntdll`,
   `advapi32`, `user32`, `ole32`, `oleaut32`, `ws2_32`, `crypt32`, `bcrypt`,
   `bcryptprimitives`, `wtsapi32`, `api-ms-win-core-synch`,
   `api-ms-win-shcore-scaling`). The same walk of a non-crt-static binary still
   imports `VCRUNTIME140.dll` + all 11 `api-ms-win-crt-*` sets, proving the flag
   is the cause. Copied alone into an empty dir, the binary runs: `argo --version`
   → exit 0 (`profile: release`); `argo doctor` → full report (non-zero exit only
   from the uninitialized-workspace warnings, not a load failure).

All the UCRT (`api-ms-win-crt-*` → `ucrtbase.dll`) and core Win32 imports are OS
components on every supported Windows version — no action needed.

## 7. End-user install/run

```powershell
# 1. Download argo-x86_64-pc-windows-msvc.zip from the GitHub Release.
# 2. Extract argo.exe anywhere (e.g. %LOCALAPPDATA%\Programs\argo\).
Expand-Archive argo-x86_64-pc-windows-msvc.zip -DestinationPath "$env:LOCALAPPDATA\Programs\argo"
# 3. (optional) add that dir to PATH.
# 4. Run it:
argo --version
argo doctor          # environment self-check
```
Prereq: the VC++ 2015–2022 x64 redistributable (present by default on essentially
all modern Windows desktops; install `vc_redist.x64.exe` only on a stripped box).

## 8. Summary

- **Single-binary invariant holds on Windows** — one `argo.exe`, runs from a
  clean dir, no sidecar files. ✅
- **Distribution = a zip of that one `argo.exe`** (`argo-x86_64-pc-windows-msvc.zip`),
  produced by the (currently CI-billing-disabled) `release.yml` Windows leg.
- **Zero non-OS runtime deps (as of `win-packaging-crt-static`, 2026-06-14).**
  The `+crt-static` build (§6 option 2, now applied in `.cargo/config.toml`)
  removed the last one (`VCRUNTIME140.dll`); the release `argo.exe` now imports
  only OS-guaranteed DLLs and runs on a bare Windows install with no VC++
  redistributable. "Zero-prereq single file" goal: met.

_Verified by the Windows fleet (Latitude5440) on the `a19eaca` release build
(fresh from `origin/main`)._
