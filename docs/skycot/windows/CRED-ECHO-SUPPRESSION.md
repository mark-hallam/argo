# Console echo suppression for secret prompts on Windows (`win-cli-console-echo-suppression-secrets`)

**Question:** do `argo cred set` and `argo connector authorize` mask the secret
the operator types on the **Windows** console? (Before this fix: NO — the
`#[cfg(not(unix))] disable_echo()` arm was a no-op, so the provider key / API key
/ OAuth token was rendered in **plaintext** as typed.)

**Verdict: 🟢 echo suppression now wired for Windows** via the Win32 Console API,
shared by both prompts. Unix `stty` path byte-identical. The bit-math is unit-
tested on Windows; the interactive type-and-eyeball confirmation is operator-
gated (NO THEATER — see §4).

NO THEATER: every claim below is a real result from this Windows host; the one
step that cannot be driven autonomously is named as a GAP, not faked.

---

## 1. The bug (before — findings 5 + 6 of portability-audit-2)

`crates/argo-cli/src/handlers/cred_set.rs` and
`crates/argo-cli/src/handlers/connector_authorize_input.rs` each had their own
copy of:
```rust
#[cfg(unix)]      fn disable_echo() -> bool { stty -echo }   // suppresses echo
#[cfg(not(unix))] fn disable_echo() -> bool { false }        // NO-OP on Windows
```
So on native Windows the secret typed at `value for <KEY>: ` was echoed in
plaintext (shoulder-surf / screen-share / terminal-scrollback exposure).

## 2. The fix

One shared seam — `crates/argo-cli/src/handlers/secret_input.rs` — used by BOTH
prompts (the two per-file copies are deleted, no parallel abstraction):
- **unix:** `stty -echo` / `stty echo` (byte-identical to before).
- **Windows:** a marked FFI shim clears `ENABLE_ECHO_INPUT` on the stdin console
  via `GetStdHandle`→`GetConsoleMode`→`SetConsoleMode` (the already-vendored
  `windows` 0.58 crate; no new external dependency). Restored after the read.
- **non-console stdin** (piped, e.g. `echo secret | argo cred set KEY`):
  `GetConsoleMode` fails → suppression is a correct no-op (no console echo to
  suppress); the read still works.

## 3. Verified on real Windows (`x86_64-pc-windows-msvc`, against `origin/main ce99d4e23`)

- `cargo test -p argo-cli --bin argo secret_input` → **1 passed / 0 failed**:
  `without_echo_clears_only_the_echo_bit` — proves the Windows mode math clears
  `ENABLE_ECHO_INPUT` while preserving `ENABLE_LINE_INPUT` (so line reading still
  works) and that restore round-trips. Pure (no real console needed), so it runs
  under `cargo test` even with piped stdin.
- argo-cli COMPILES clean with the new `[target.'cfg(windows)'] windows = "0.58"`
  Console feature (5 warnings present are PRE-EXISTING macos dead-code, not this
  change).
- `cargo clippy -p argo-cli --bin argo` — clean on `secret_input.rs` /
  `cred_set.rs` / `connector_authorize_input.rs`.

## 4. Honest GAP (NO THEATER) — interactive masking is operator-gated

A true "type a secret at a live console and confirm it does NOT appear" check
**was not performed autonomously**: the fleet's shell tools run with a *piped*
(non-console) stdin, so (a) the interactive prompt can't be driven by typing, and
(b) on a pipe `GetConsoleMode` fails and suppression correctly no-ops — so a
piped run cannot exercise (or demonstrate) console masking either. This is the
same class of operator-gate as the live-desktop CU run.

What IS proven autonomously: the exact OS mechanism (clearing
`ENABLE_ECHO_INPUT` on the console input mode) is the standard Windows
echo-masking primitive, and the bit manipulation + restore is unit-tested above.

**Operator runbook to confirm masking manually (1 min):**
```
> argo cred set TEST_SECRET_KEY
value for TEST_SECRET_KEY:   <type some characters — they must NOT appear>
stored TEST_SECRET_KEY
> argo cred rm TEST_SECRET_KEY   # cleanup
```
If characters appear, that is a real failure — report it; do not paper over it.

## 5. Verdict

`argo cred set` / `argo connector authorize` now suppress console echo on Windows
through one shared, unit-tested seam; the unix path is unchanged. The only
unverified step (interactive eyeball) is operator-gated and documented with a
runbook rather than faked.

_Verified by the Windows fleet (Latitude5440). Shared seam:
`crates/argo-cli/src/handlers/secret_input.rs`. Conductor re-verifies the unix
`stty` path (byte-identical) + a local `argo cred set` masking check on macOS._
