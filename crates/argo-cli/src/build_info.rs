//! Runtime accessor for the build-time facts captured by `build.rs`.
//! Pairs with `argo --version` and the `Build Info` block in
//! `argo doctor` so operators reporting an incident can quote a
//! single line that pins their exact binary
//! (`mission-cli-version-doctor-extras`).
include!(concat!(env!("OUT_DIR"), "/build_info.rs"));

use serde::Serialize;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `argo --version`-style long form. Multi-line so clap's
/// `--version` output stays grep-friendly while still showing the
/// full provenance an operator needs in a bug report.
pub fn version_string_long() -> String {
    format!(
        "argo {VERSION} ({GIT_SHA} {GIT_DATE})\n  \
         target:   {TARGET}\n  \
         rustc:    {RUSTC_VERSION}\n  \
         profile:  {PROFILE}\n  \
         features: {features}",
        features = if FEATURES.is_empty() {
            "(none)".to_string()
        } else {
            FEATURES.join(",")
        },
    )
}

/// Stable JSON shape for the `build_info` key in
/// `argo doctor --json` and the embedded structure in any future
/// machine-consumed envelope.
#[derive(Debug, Clone, Serialize)]
pub struct BuildInfo {
    pub version: &'static str,
    pub git_sha: &'static str,
    pub git_date: &'static str,
    pub target: &'static str,
    pub rustc: &'static str,
    pub profile: &'static str,
    pub features: Vec<&'static str>,
}

impl BuildInfo {
    pub fn current() -> Self {
        Self {
            version: VERSION,
            git_sha: GIT_SHA,
            git_date: GIT_DATE,
            target: TARGET,
            rustc: RUSTC_VERSION,
            profile: PROFILE,
            features: FEATURES.to_vec(),
        }
    }
}
