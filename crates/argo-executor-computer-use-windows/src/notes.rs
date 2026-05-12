//! Windows-platform caveats consumed by the runbook.
//!
//! Each caveat is a `(slug, headline, body)` row that the
//! operator-facing runbook generator can render. Splitting them
//! out here keeps the runbook scannable: each entry is the
//! shortest accurate sentence about one platform peculiarity an
//! operator might hit.
//!
//! Caveats are platform-static — every Windows host suffers from
//! the same set — so the rows are `const` and the lookup is
//! `O(n)`. If the list grows past a couple dozen, swap for a
//! `phf::Map`.

use serde::{Deserialize, Serialize};

/// One Windows-platform caveat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caveat {
    /// Stable, machine-readable slug.
    pub slug: &'static str,
    /// One-line headline for the runbook section title.
    pub headline: &'static str,
    /// Body paragraph(s) for the runbook section.
    pub body: &'static str,
}

/// The full list of Windows caveats, in operator-priority order
/// (UAC + secure-desktop first because those bite earliest).
pub const ALL: &[Caveat] = &[
    Caveat {
        slug: "dpi-awareness",
        headline: "DPI awareness must be declared at process start",
        body: "Without `dpi_aware = true` in [executor.computer_use.windows] the Windows compositor reports virtual pixel dimensions instead of physical pixels; screenshots come back blurry and click coordinates land off-target on high-density displays. Declare per-monitor v2 at process start; mid-process toggling is unreliable.",
    },
    Caveat {
        slug: "secure-desktop",
        headline: "Secure desktop silently blocks SendInput",
        body: "When a UAC prompt is visible, the workstation is locked, or the user has hit Ctrl-Alt-Del, the OS routes input to a separate secure desktop. SendInput from a user-mode process silently no-ops (no error code, no exception). The preflight axis `SecureDesktop` detects this state; the runtime path raises `WindowsCuError::InputBlocked` after a failed SendInput.",
    },
    Caveat {
        slug: "uac-elevation-parity",
        headline: "UAC-elevated processes can't inject into non-elevated UIs (and vice-versa)",
        body: "Windows enforces UIPI (User Interface Privilege Isolation): a medium-integrity process cannot send input to a high-integrity window, and vice versa. If argo runs at one integrity level and the target app at another, every SendInput silently fails. Preflight axis `UacConsent` records argo's own integrity level; the runtime path raises `WindowsCuError::IntegrityMismatch` after a failed cross-IL probe.",
    },
    Caveat {
        slug: "multi-monitor-coords",
        headline: "Multi-monitor uses virtual-screen coordinates",
        body: "The virtual screen's origin is the top-left of the primary monitor. Monitors to the left or above have negative virtual coordinates. `MOUSEEVENTF_ABSOLUTE` normalises against `[0, 65535]` across the entire virtual screen (NOT per-monitor). The executor's `normalise_mouse_coords` helper does the conversion.",
    },
    Caveat {
        slug: "no-rdp-passthrough",
        headline: "Don't run argo over RDP for computer-use",
        body: "Argo's native Windows executor is for desktops it runs on directly. Operators driving a remote Windows host should install argo on the remote, not on the local viewer. Indirect-input passthrough (RDP / Citrix) is intentionally out-of-scope: input timing, focus, and clipboard semantics diverge enough that v1 wouldn't ship a reliable abstraction.",
    },
    Caveat {
        slug: "com-init",
        headline: "UIAutomation requires CoInitializeEx on the calling thread",
        body: "All UIAutomation helpers in `a11y::` are COM clients. Callers must call `CoInitializeEx(COINIT_MULTITHREADED)` on each thread before invoking them. `WindowsCuExecutor::warm_up` handles this lazily; direct `a11y::` consumers must do it themselves.",
    },
];

/// Look up one caveat by slug.
#[must_use]
pub fn find(slug: &str) -> Option<&'static Caveat> {
    ALL.iter().find(|c| c.slug == slug)
}

/// Whether the caveat list still covers every required failure
/// class (used as a regression test).
#[must_use]
pub fn covers_required_slugs() -> bool {
    const REQUIRED: &[&str] = &[
        "dpi-awareness",
        "secure-desktop",
        "uac-elevation-parity",
        "multi-monitor-coords",
        "no-rdp-passthrough",
        "com-init",
    ];
    REQUIRED.iter().all(|r| find(r).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_caveats_have_unique_slugs() {
        let mut seen = std::collections::HashSet::new();
        for c in ALL {
            assert!(seen.insert(c.slug), "duplicate slug: {}", c.slug);
        }
    }

    #[test]
    fn all_caveats_have_nonempty_bodies() {
        for c in ALL {
            assert!(!c.headline.is_empty());
            assert!(c.body.len() > 40, "caveat `{}` body too short", c.slug);
        }
    }

    #[test]
    fn find_returns_matching_caveat() {
        let c = find("secure-desktop").unwrap();
        assert_eq!(c.slug, "secure-desktop");
    }

    #[test]
    fn find_returns_none_for_unknown_slug() {
        assert!(find("not-a-slug").is_none());
    }

    #[test]
    fn required_slugs_are_covered() {
        assert!(covers_required_slugs());
    }
}
