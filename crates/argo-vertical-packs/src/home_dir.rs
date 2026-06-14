//! Cross-platform home-directory resolution for the pack cache root.
//!
//! Extracted to its own module so [`crate::loader`] stays under the 500-LOC
//! ceiling. Mirrors the #2904 pattern in `argo-config`/`argo-credentials`:
//! `HOME` is the unix convention and wins when set (so unix behaviour is
//! byte-identical, and a git-bash shell on Windows that exports `HOME` keeps
//! working); on native Windows `HOME` is usually unset, so we fall back to
//! `USERPROFILE` (e.g. `C:\Users\alice`). Without this the pack loader
//! hard-errored ("HOME is not set") on native Windows, breaking every
//! `argo packs` subcommand. See `win-vertical-packs-home-userprofile-fallback`.

use std::ffi::OsString;
use std::path::PathBuf;

/// Resolve the user's home directory cross-platform: `HOME`, else `USERPROFILE`.
///
/// Returns `None` only when neither is set to a non-empty value.
pub(crate) fn home_dir() -> Option<PathBuf> {
    home_dir_from(std::env::var_os("HOME"), std::env::var_os("USERPROFILE"))
}

/// Pure core of [`home_dir`] — env values passed explicitly so the precedence
/// can be asserted without mutating process-global environment. Empty values
/// are treated as unset so a stray `HOME=` does not win.
fn home_dir_from(home: Option<OsString>, userprofile: Option<OsString>) -> Option<PathBuf> {
    home.filter(|h| !h.is_empty())
        .or_else(|| userprofile.filter(|h| !h.is_empty()))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_wins_when_set() {
        let got = home_dir_from(Some("/home/alice".into()), Some("C:\\Users\\alice".into()));
        assert_eq!(got, Some(PathBuf::from("/home/alice")));
    }

    #[test]
    fn falls_back_to_userprofile_when_home_unset() {
        // Native Windows: HOME unset → USERPROFILE instead of the prior hard error.
        let got = home_dir_from(None, Some("C:\\Users\\alice".into()));
        assert_eq!(got, Some(PathBuf::from("C:\\Users\\alice")));
    }

    #[test]
    fn empty_home_is_ignored_and_userprofile_used() {
        let got = home_dir_from(Some("".into()), Some("C:\\Users\\bob".into()));
        assert_eq!(got, Some(PathBuf::from("C:\\Users\\bob")));
    }

    #[test]
    fn none_when_neither_set() {
        assert_eq!(home_dir_from(None, None), None);
        assert_eq!(home_dir_from(Some("".into()), Some("".into())), None);
    }
}
