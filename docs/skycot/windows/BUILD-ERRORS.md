# Windows workspace build — error inventory

**Brief**: `win-build-smoke-and-error-inventory` (P1 FOUNDATION — discovery).
**Fleet**: WINDOWS. This is the deliverable; "green" is not the goal — the
complete, grouped, verified error inventory is.

## Build environment (verified this run)

| Field | Value |
|---|---|
| Machine | `Latitude5440` — Windows 11 Pro 10.0.26200 |
| Host triple | `x86_64-pc-windows-msvc` (MSVC toolchain) |
| rustc | 1.96.0 (ac68faa20 2026-05-25), LLVM 22.1.2 |
| cargo | 1.96.0 (30a34c682 2026-05-25) |
| Command | `cargo build --workspace --keep-going --message-format=human` |
| Built against | `origin/main` @ `7dc715650` (2026-06-13) |
| Result | **EXIT_CODE=101** — errors in **2 of ~95** workspace crates |

> **Why `--keep-going`.** The plain `cargo build --workspace` halts scheduling
> on the *first* error, which is `kryptering`. Because `kryptering` is a
> *dependency* of `argo-cli`, that single failure masks every downstream error.
> `--keep-going` forces cargo to compile every crate it *can*, so the inventory
> below is the **full** set of independent compile failures — not just the first.

## Headline finding

The first-ever Windows build is **far healthier than the queued briefs assumed.**
Of ~95 workspace crates, **exactly two fail to compile**:

1. **`kryptering`** — a *third-party* dependency reached only through `argo-cli`'s
   HSM/PKCS#11 signing chain. **No queued brief covers it → NEW.**
2. **`argo-executor-computer-use-windows`** — the crown-jewel crate's first real
   Windows compile. **Covered by `win-computer-use-executor-first-build`.**

**Every other crate compiled** (warnings only). The four speculative
`win-portability-*-unix-perms` "won't-compile" briefs are **stale**: the unix
file-perm / signal code they target is *already* `#[cfg(unix)]`/`#[cfg(not(unix))]`
gated and compiled clean on Windows. See "Stale briefs" below — re-validate
before claiming.

---

## Crate 1 — `kryptering` (third-party) → blocks `argo-cli` only

**Status**: hard blocker. **Brief: NEW** (none queued covers it).

| # | Error | Location |
|---|---|---|
| 1 | `E0277` the trait bound `Ulong: From<u64>` is not satisfied | `kryptering-0.1.0/src/pkcs11/mod.rs:147` (`s_len: s_len.into()`) |
| 2 | `E0277` the trait bound `Ulong: From<u64>` is not satisfied | `kryptering-0.1.0/src/pkcs11/mod.rs:610` (`Attribute::ValueLen((self.key_len as u64).into())`) |

**Root cause.** PKCS#11's `CK_ULONG` is **32-bit on Windows** (the Windows
PKCS#11 ABI) but 64-bit on unix. So `cryptoki 0.7.0` implements
`impl From<CK_ULONG> for Ulong` as `From<u32>` on Windows, **not** `From<u64>`.
`kryptering` casts to `u64` then `.into()` — which only resolves where
`Ulong: From<u64>` (unix). On Windows it won't compile.

**Dependency chain** (`cargo tree -i kryptering`):
```
argo-cli → bergshamra-dsig / bergshamra-keys → bergshamra-crypto → kryptering → cryptoki 0.7
```
`argo-cli` is the **only** workspace crate that pulls this in.

**Impact.** The main `argo` binary (`argo-cli`) cannot link on Windows. This
also **masks all of argo-cli's own source errors** — cargo can't type-check a
crate whose dependency won't build (see "Honest gaps").

**Suggested fix (one-liner).** Feature-gate the PKCS#11/HSM signing backend OFF
on Windows — make `bergshamra-crypto`'s pkcs11 path an opt-in feature that
`argo-cli` disables under `[target.'cfg(windows)']`, falling back to the
software-key signing path; *or* bump/patch `cryptoki` to a release that handles
the Windows `CK_ULONG` width. All three crates are external, so the practical
lever is **dropping the HSM dependency on Windows**, not editing our code.
→ **Recommend authoring `win-portability-argo-cli-pkcs11-hsm-windows`.**

---

## Crate 2 — `argo-executor-computer-use-windows` → 10 errors

**Status**: first-ever Windows compile of the crown jewel.
**Brief: `win-computer-use-executor-first-build`** (owns every fix below).

### Group A — missing `windows` crate feature `Win32_Security` (2 errors)

| # | Error | Location |
|---|---|---|
| 1 | `E0432` unresolved import `windows::Win32::Security` (item gated behind feature `Win32_Security`) | `src/preflight/probes.rs:13` |
| 2 | `E0432` no `OpenProcessToken` in `Win32::System::Threading` (gated behind `Win32_Security`) | `src/preflight/probes.rs:16` |

**Root cause.** The `windows = "0.58"` feature array in
`argo-executor-computer-use-windows/Cargo.toml` does not list `Win32_Security`,
so `Security::*` and `OpenProcessToken` are configured out.
**Fix one-liner.** Add `"Win32_Security"` to that crate's `windows` features.

### Group B — `windows` 0.58 API drift (8 errors)

| # | Error | Location | Fix one-liner |
|---|---|---|---|
| 3 | `E0432` no `MONITORINFOF_PRIMARY` in `Win32::Graphics::Gdi` | `src/screenshot.rs:100` | Constant moved/renamed in 0.58 — import from its new module or compare `dwFlags & 1` (the `MONITORINFOF_PRIMARY` value). |
| 4 | `E0599` no method `is_invalid` on `IUIAutomationElement` | `src/a11y.rs:90` | COM interfaces aren't HANDLEs — drop `is_invalid()`; check the `Result`/`Option` returned by the call that produced the element. |
| 5 | `E0599` no method `is_invalid` on `IUIAutomationElement` | `src/a11y.rs:127` | Same as #4. |
| 6 | `E0308` mismatched types: `hwnd.0 == 0` (expected `*mut c_void`, found integer) | `src/events.rs:346` | In 0.58 `HWND.0` is a pointer — use `hwnd.is_invalid()` or `hwnd.0.is_null()`. |
| 7 | `E0271` type mismatch `Option<HANDLE>: Param<HANDLE>` for `WTSQuerySessionInformationW(Some(WTS_CURRENT_SERVER_HANDLE), …)` | `src/preflight/probes.rs:56` | Pass `WTS_CURRENT_SERVER_HANDLE` **directly** (not wrapped in `Some()`). |
| 8 | `E0277` `Option<HANDLE>: CanInto<HANDLE>` not satisfied (same call) | `src/preflight/probes.rs:56` | Same as #7 (cascades from the `Some(...)` wrap). |
| 9 | `E0277` `Option<HANDLE>: Interface` not satisfied (same call) | `src/preflight/probes.rs:56` | Same as #7 (cascades from the `Some(...)` wrap). |
| 10 | `E0599` no method `as_bool` on `Result<(), windows_result::Error>` | `src/screenshot.rs:203` | In 0.58 the GDI call returns `Result`, not `BOOL` — use `if ok.is_ok()`. |

> Errors 7–9 are three diagnostics for the **same** call site (`probes.rs:56`);
> fixing the `Some(...)` wrap clears all three.

**Why inventory-only (not fixed here).** Per the brief, fixes >10 lines or
non-obvious are left to the owning brief. Group B is genuine `windows`-crate API
porting (COM semantics, pointer types) and belongs to
`win-computer-use-executor-first-build`. This discovery brief does not touch it.

---

## Stale briefs — already `#[cfg]`-gated, compiled clean (re-validate before claiming)

The `--keep-going` build compiled the following crates **with no errors**,
contradicting the briefs' "won't-compile" premise. Each was authored
speculatively on macOS and never validated on Windows; the gating was already
present. Per `DRIFT_PREVENTION.md` ("recovery briefs re-validate intent"),
recommend the conductor close these **STALE-FIXED** after a unix-side re-check.

| Brief | Evidence (already gated) | Build verdict |
|---|---|---|
| `win-portability-cred-credentials-unix-perms` | `argo-config/src/cred/io.rs:16-17,258-259,267-269`; `argo-credentials/src/master_key.rs:369/383`, `src/lib.rs:56-57,217-218,228-229,286-304` — all `#[cfg(unix)]`/`#[cfg(not(unix))]` | **argo-config + argo-credentials compiled clean** |
| `win-portability-runtime-tool-unix` | `argo-runtime/src/oom_restart.rs:232/376/385` gated (platform-matrix doc explicitly: "Windows → falls through to `OtherSignal`"); `argo-tool/src/path_guard.rs` unix only in `#[cfg(unix)] #[test]` fns; `process_registry_tail.rs:97-110` gated | **argo-runtime + argo-tool compiled clean** |
| `win-portability-cli-handler-unix-perms` | spot-checked `connector_authorize_state.rs:11-12`, `doctor_install_hooks.rs`, `bootstrap_token.rs`, `onboard_stages.rs`, `self_update_install.rs`, `migrate_encrypt.rs` — all `#[cfg(unix)]`-gated | **UNVERIFIABLE on Windows** — argo-cli is blocked by the `kryptering` dep (Crate 1); spot-checks suggest already-gated but cargo cannot confirm until kryptering is resolved |
| `win-portability-tmp-and-home-paths` | runtime path resolution (`/tmp`, `~/.argo`) — **not a compile error** | **not surfaced by `cargo build`**; this is a *runtime* portability brief, out of scope for a compile inventory — keep as runtime work |

Independently confirmed already-gated (were never in a brief, no action needed):
`argo-skill/src/sign.rs:337-354` and `argo-mcp-connector-gmail/src/auth.rs:108-125`.

---

## Minor (non-blocking, NOT Windows-specific)

`argo-executor-computer-use-macos` emits 9 warnings — dead code / unused imports
(`truncate_stderr`, `build_script`, `keys_script`, `scroll_keycode`,
`parse_bounds`). These are platform-agnostic (present on macOS too), not Windows
issues. Optional tidy-up; not in scope here.

---

## Honest gaps — what this inventory does NOT cover (NO THEATER)

1. **`argo-cli`'s own source errors are masked.** Because `kryptering` (its
   transitive dep) fails, cargo never type-checks argo-cli itself. Its true
   Windows compile status — and the real status of
   `win-portability-cli-handler-unix-perms` — **cannot be enumerated until the
   Crate 1 blocker is cleared.** Spot-checks indicate the handlers are already
   gated, but that is source inspection, not a build result.
2. **Compile only, not tests.** This is `cargo build` (lib + bin). Test-only
   unix usages (`dispatch_tests.rs`, `oom_restart_tests.rs`, `path_guard.rs`
   tests, `fs_root_cross_tenant.rs`, `document_csv_tests.rs`, the gmail/skill
   test modules) are **not** exercised here — they belong to
   `win-test-suite-run-and-failure-inventory` + `win-line-ending-and-path-sep-test-audit` (P2).
3. **Runtime portability not covered.** Hardcoded `/tmp` and `~/.argo`
   expansion compile fine; they fail (if at all) at *runtime*, which a build
   cannot detect. Owned by `win-portability-tmp-and-home-paths`.

## Recommended queue actions (for the conductor)

1. **Author `win-portability-argo-cli-pkcs11-hsm-windows`** (NEW, P1) — feature-gate
   the bergshamra/kryptering HSM path off on Windows. This is the **gating**
   blocker: it must land before `argo-cli` (and therefore the cli-handler brief)
   can be Windows-verified.
2. **Keep `win-computer-use-executor-first-build`** as-is — its 10-error
   inventory is above; ready to claim.
3. **Re-validate + likely close STALE-FIXED**: `win-portability-cred-credentials-unix-perms`,
   `win-portability-runtime-tool-unix` (build-confirmed already-gated).
4. **Re-order `win-portability-cli-handler-unix-perms` to depend on** the new
   pkcs11 brief (can't be Windows-verified before it).
