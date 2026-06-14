//! Tests for the workspace `root_path` comparison-key canonicalization
//! (`win-store-canonicalize-workspace-root`).
//!
//! The unix half asserts the normalizer is a **byte-for-byte no-op** (so no
//! existing unix tenant re-mints); the Windows half asserts that backslash /
//! forward-slash / drive-case / `\\?\` / trailing-separator spellings of the
//! SAME directory canonicalize to one key and resolve to the SAME
//! `(TenantId, WorkspaceId)`.

use super::canonicalize_root_key;

// --- unix: the normalizer must NOT change anything (no re-mint) ---

#[cfg(not(windows))]
#[test]
fn unix_canonicalize_is_byte_identical_passthrough() {
    for s in [
        "/tmp/x",
        "/Users/operator/Projects/acme",
        "/tmp/x/", // a trailing slash stays — unix keys are passed through verbatim
        "relative/path",
        "",
    ] {
        assert_eq!(canonicalize_root_key(s), s, "unix must be a no-op for {s:?}");
    }
}

// --- windows: every spelling of the same dir collapses to one key ---

#[cfg(windows)]
#[test]
fn windows_variants_collapse_to_one_key() {
    let canonical = "c:/Users/x/proj";
    for variant in [
        r"C:\Users\x\proj",       // backslashes + uppercase drive
        "C:/Users/x/proj",        // forward slashes + uppercase drive
        r"c:\Users\x\proj",       // backslashes + lowercase drive
        r"\\?\C:\Users\x\proj",   // \\?\ verbatim prefix
        r"C:\Users\x\proj\",      // trailing backslash
        "C:/Users/x/proj/",       // trailing forward slash
    ] {
        assert_eq!(
            canonicalize_root_key(variant),
            canonical,
            "variant {variant:?} should canonicalize to {canonical:?}"
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_drive_root_is_not_truncated() {
    // A bare drive root keeps its trailing slash (don't reduce `c:/` to `c:`).
    assert_eq!(canonicalize_root_key(r"C:\"), "c:/");
    assert_eq!(canonicalize_root_key("C:/"), "c:/");
}

#[cfg(windows)]
#[test]
fn windows_non_drive_paths_pass_through_separator_only() {
    // No drive letter (e.g. a UNC-ish or relative spelling): only separators
    // normalize; nothing is lowercased.
    assert_eq!(canonicalize_root_key(r"server\share\Proj"), "server/share/Proj");
}

// --- windows: the end-to-end data-integrity guarantee ---

#[cfg(windows)]
#[tokio::test]
async fn ensure_workspace_resolves_windows_path_variants_to_same_tenant() {
    let store = crate::SessionStore::open(":memory:")
        .await
        .expect("open in-memory store");
    store.migrate().await.expect("migrate");

    // First spelling mints the (tenant, workspace).
    let (t0, w0) = store
        .ensure_workspace("acme", r"C:\Users\x\proj")
        .await
        .expect("first ensure_workspace");

    // Every other spelling of the SAME directory must REUSE that row, not mint
    // a phantom tenant.
    for variant in [
        "C:/Users/x/proj",
        r"c:\Users\x\proj",
        r"\\?\C:\Users\x\proj",
        r"C:\Users\x\proj\",
    ] {
        let (t, w) = store
            .ensure_workspace("acme", variant)
            .await
            .unwrap_or_else(|e| panic!("ensure_workspace({variant:?}): {e}"));
        assert_eq!(t, t0, "variant {variant:?} must resolve to the same tenant");
        assert_eq!(w, w0, "variant {variant:?} must resolve to the same workspace");
    }

    // Sanity: a genuinely different directory still mints a distinct workspace.
    let (_t_other, w_other) = store
        .ensure_workspace("other", r"C:\Users\x\other")
        .await
        .expect("different dir");
    assert_ne!(w_other, w0, "a different directory must not collapse onto proj");
}
