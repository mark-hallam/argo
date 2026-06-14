//! Shared hidden-stdin reader for interactive secret prompts.
//!
//! `argo cred set` and `argo connector authorize` both read a secret
//! (provider key / API key / OAuth token) from the terminal. Echo suppression
//! used to be duplicated in each handler with a unix `stty -echo` path and a
//! `#[cfg(not(unix))]` NO-OP — so on native Windows the secret was rendered in
//! PLAINTEXT on the console (shoulder-surf / screen-share / scrollback exposure).
//!
//! This is the single seam both handlers use. The unix path is byte-identical to
//! the old `stty` pair; the Windows path clears `ENABLE_ECHO_INPUT` on the
//! console via the Win32 Console API (one marked FFI shim). When stdin is NOT a
//! console (piped, e.g. `echo secret | argo cred set KEY`) `GetConsoleMode`
//! fails and echo suppression is correctly a no-op — there is no console echo to
//! suppress. See `win-cli-console-echo-suppression-secrets`.

use argo_types::{ArgoError, ArgoResult};
use std::io::Write;

/// Prompt on stderr, read one line from stdin with terminal echo suppressed,
/// and return the trimmed secret. Echo is restored before returning.
pub(crate) fn read_hidden_value(label: &str) -> ArgoResult<String> {
    eprint!("value for {label}: ");
    std::io::stderr()
        .flush()
        .map_err(|e| ArgoError::Provider(format!("stderr flush: {e}")))?;
    let echo_disabled = disable_echo();
    let mut value = String::new();
    let read_result = std::io::stdin()
        .read_line(&mut value)
        .map_err(|e| ArgoError::Io(format!("read secret from stdin: {e}")));
    if echo_disabled {
        let _ = enable_echo();
        eprintln!();
    }
    read_result?;
    Ok(value.trim_end_matches(['\r', '\n']).to_string())
}

#[cfg(unix)]
fn disable_echo() -> bool {
    stty("-echo")
}

#[cfg(unix)]
fn enable_echo() -> bool {
    stty("echo")
}

#[cfg(unix)]
fn stty(arg: &str) -> bool {
    std::process::Command::new("stty")
        .arg(arg)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn disable_echo() -> bool {
    win::set_console_echo(false)
}

#[cfg(windows)]
fn enable_echo() -> bool {
    win::set_console_echo(true)
}

#[cfg(not(any(unix, windows)))]
fn disable_echo() -> bool {
    false
}

#[cfg(not(any(unix, windows)))]
fn enable_echo() -> bool {
    false
}

#[cfg(windows)]
mod win {
    use windows::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE, ENABLE_ECHO_INPUT,
        STD_INPUT_HANDLE,
    };

    /// Pure bit math (testable without a real console): clear `ENABLE_ECHO_INPUT`
    /// while leaving every other console-mode bit untouched.
    pub(super) fn without_echo(mode: CONSOLE_MODE) -> CONSOLE_MODE {
        CONSOLE_MODE(mode.0 & !ENABLE_ECHO_INPUT.0)
    }

    /// Pure bit math: set `ENABLE_ECHO_INPUT` (restore echo).
    pub(super) fn with_echo(mode: CONSOLE_MODE) -> CONSOLE_MODE {
        CONSOLE_MODE(mode.0 | ENABLE_ECHO_INPUT.0)
    }

    /// Toggle console echo on the stdin handle. Returns `true` when the mode was
    /// changed (so the caller knows to restore). A no-op returning `false` when
    /// stdin is not a console (piped/redirected — `GetConsoleMode` fails), which
    /// is correct: there is no console echo to suppress for scripted input.
    pub(super) fn set_console_echo(enable: bool) -> bool {
        // FFI SHIM: the only `unsafe` is the three Win32 Console calls below.
        // All take a handle we just obtained and a by-ref/by-value mode; no
        // memory aliasing. Errors are mapped to `false` (treated as "no console").
        unsafe {
            let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) else {
                return false;
            };
            let mut mode = CONSOLE_MODE(0);
            if GetConsoleMode(handle, &mut mode).is_err() {
                return false;
            }
            let new_mode = if enable {
                with_echo(mode)
            } else {
                without_echo(mode)
            };
            SetConsoleMode(handle, new_mode).is_ok()
        }
    }
}

#[cfg(test)]
mod tests {
    /// On Windows the echo-bit math must clear ONLY `ENABLE_ECHO_INPUT` and
    /// leave the other mode bits (e.g. `ENABLE_LINE_INPUT`) intact — so line
    /// reading still works while typed characters are masked. Pure (no real
    /// console needed), so it runs under `cargo test` even with piped stdin.
    /// `win-cli-console-echo-suppression-secrets`.
    #[cfg(windows)]
    #[test]
    fn without_echo_clears_only_the_echo_bit() {
        use super::win::{with_echo, without_echo};
        use windows::Win32::System::Console::{
            CONSOLE_MODE, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT,
        };
        let original = CONSOLE_MODE(ENABLE_ECHO_INPUT.0 | ENABLE_LINE_INPUT.0);
        let masked = without_echo(original);
        assert_eq!(masked.0 & ENABLE_ECHO_INPUT.0, 0, "echo bit must be cleared");
        assert_eq!(
            masked.0 & ENABLE_LINE_INPUT.0,
            ENABLE_LINE_INPUT.0,
            "line-input bit must be preserved (line reading still works)"
        );
        // Restoring re-sets the echo bit, recovering the original mode.
        assert_eq!(with_echo(masked).0, original.0, "restore must round-trip");
    }
}
