//! Structural pin for v1.x-deferred substrate caller sets.
//!
//! Per CLAUDE.md `v1.0 strategic decisions` (operator-locked 2026-05-14):
//! - `argo-executor-computer-use-macos` is the ONLY OS-native executor v1.0
//!   hardens.
//! - `argo-sandbox`, `argo-executor-computer-use` (E2B),
//!   `argo-executor-computer-use-linux`, and `argo-executor-computer-use-windows`
//!   are DEFERRED-v1.x — they stay shipped, just not v1-polished.
//!
//! This pin asserts that the production caller set for each deferred
//! substrate has NOT grown beyond the allowlist snapshot taken on
//! `origin/main` `140db7c03` (2026-05-24). If a worker accidentally promotes
//! a deferred substrate via a "small refactor" PR, this test fails loud
//! with an actionable diff and a two-path remediation message.
//!
//! ## Detection scope
//!
//! - **Forbidden caller crates** (`FORBIDDEN_CALLER_CRATES`): `argo-cli`,
//!   `argo-agent-runner`, `argo-orchestrator` — the three top-level
//!   production crates the brief names. Other workspace crates
//!   (e.g. `argo-tool`'s use of the `Sandbox` trait, or per-OS executors
//!   depending on the shared substrate) are intentionally NOT pinned;
//!   intra-substrate plumbing is not promotion.
//! - Walk only `crates/<caller>/src/`. The `crates/<caller>/tests/`
//!   directories hold integration tests, which the brief explicitly
//!   allows ("the only references are tests and capability declarations").
//! - **Anchor**: a substring search for `<rust_path>::` (path-style
//!   reference) or `extern crate <rust_path>` (legacy form). The `::`
//!   separator means `argo_executor_computer_use::` does NOT match
//!   `argo_executor_computer_use_linux::` (next byte is `_`, not `:`), so
//!   the four prefix-overlapping rust-paths are detected independently.
//!
//! ## Allowlist semantics
//!
//! Each pin entry lists the EXACT set of files under
//! `crates/<forbidden-caller>/src/` that reference the substrate today.
//! Any divergence — additions OR removals — fails the test:
//! - **Added** caller = a v1.x → v1.0 promotion that must be
//!   operator-blessed.
//! - **Removed** caller = a deletion that needs follow-up (the pin and
//!   reality have drifted).
//!
//! ## When intentionally flipping a deferral
//!
//! 1. Update CLAUDE.md `v1.0 strategic decisions` (operator-decisional).
//! 2. Update the `PINS` table in this file.
//! 3. Re-run `cargo test -p argo-sandbox --test v1_deferred_status_pin`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The forbidden production caller crates from the brief. Adding a new
/// reference to any deferred substrate from src/ of these crates is the
/// drift signal.
const FORBIDDEN_CALLER_CRATES: &[&str] = &[
    "argo-cli",
    "argo-agent-runner",
    "argo-orchestrator",
];

/// A single deferred-substrate pin entry.
struct DeferredPin {
    /// Crate as it appears on disk (kebab-case).
    crate_dir: &'static str,
    /// Crate as it appears in Rust paths (snake_case).
    rust_path: &'static str,
    /// Allowed production caller files, paths relative to the workspace
    /// root, sorted ascending for deterministic diff output.
    allowed_callers: &'static [&'static str],
}

/// Pinned set snapshot vs `origin/main` `140db7c03` (2026-05-24). Each
/// `allowed_callers` list is the EXACT set of files under
/// `crates/<forbidden-caller>/src/` referencing the substrate.
const PINS: &[DeferredPin] = &[
    DeferredPin {
        crate_dir: "argo-sandbox",
        rust_path: "argo_sandbox",
        allowed_callers: &[
            "crates/argo-cli/src/handlers/worker_actions_executor_code.rs",
        ],
    },
    DeferredPin {
        crate_dir: "argo-executor-computer-use",
        rust_path: "argo_executor_computer_use",
        allowed_callers: &[
            "crates/argo-cli/src/handlers/action_replay_executors.rs",
            "crates/argo-cli/src/handlers/cu_replay.rs",
            "crates/argo-cli/src/handlers/cu_replay_tests.rs",
            "crates/argo-cli/src/handlers/cu_screenshots.rs",
            "crates/argo-cli/src/handlers/sandbox_e2b.rs",
            "crates/argo-cli/src/handlers/worker_actions_executor_cu_linux.rs",
            "crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs",
            "crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs",
        ],
    },
    DeferredPin {
        crate_dir: "argo-executor-computer-use-linux",
        rust_path: "argo_executor_computer_use_linux",
        allowed_callers: &[
            "crates/argo-cli/src/handlers/worker_actions_executor_cu_linux.rs",
        ],
    },
    DeferredPin {
        crate_dir: "argo-executor-computer-use-windows",
        rust_path: "argo_executor_computer_use_windows",
        allowed_callers: &[
            "crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs",
        ],
    },
];

/// Returns the workspace root (parent of `crates/`). `argo-sandbox`
/// itself lives at `crates/argo-sandbox`, so the workspace root is one
/// level up from `CARGO_MANIFEST_DIR`.
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(PathBuf::from)
        .expect("argo-sandbox manifest must sit at crates/argo-sandbox/")
}

/// Recursively collects every `.rs` file under `dir`, returning paths
/// relative to `relative_to`. Missing directories return an empty list.
fn collect_rs_files(dir: &Path, relative_to: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            out.extend(collect_rs_files(&path, relative_to));
        } else if file_type.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("rs")
        {
            if let Ok(rel) = path.strip_prefix(relative_to) {
                out.push(rel.to_path_buf());
            }
        }
    }
    out
}

/// Returns true if `contents` references `rust_path` as a Rust path
/// (`<rust_path>::`) or via `extern crate <rust_path>` (with a
/// non-identifier boundary, so prefix-overlapping paths don't collide).
fn file_references_path(contents: &str, rust_path: &str) -> bool {
    let path_anchor = format!("{rust_path}::");
    if contents.contains(&path_anchor) {
        return true;
    }
    let extern_prefix = format!("extern crate {rust_path}");
    for line in contents.lines() {
        if let Some(start) = line.find(&extern_prefix) {
            let after = start + extern_prefix.len();
            let next = line.as_bytes().get(after).copied();
            if matches!(next, None | Some(b';') | Some(b' ') | Some(b'\t')) {
                return true;
            }
        }
    }
    false
}

/// Scans every `.rs` file under `crates/<caller>/src/` for each forbidden
/// caller crate, returning the workspace-relative paths that reference
/// `pin.rust_path`.
fn observed_callers_for(pin: &DeferredPin, root: &Path) -> BTreeSet<String> {
    let mut observed = BTreeSet::new();
    for caller in FORBIDDEN_CALLER_CRATES {
        let caller_src = root.join("crates").join(caller).join("src");
        if !caller_src.exists() {
            continue;
        }
        for rel in collect_rs_files(&caller_src, root) {
            let abs = root.join(&rel);
            let contents = match std::fs::read_to_string(&abs) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if file_references_path(&contents, pin.rust_path) {
                observed.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    observed
}

#[test]
fn deferred_crates_have_no_surprise_callers() {
    let root = workspace_root();
    let mut drift_blocks: Vec<String> = Vec::new();

    for pin in PINS {
        let observed = observed_callers_for(pin, &root);
        let expected: BTreeSet<String> = pin
            .allowed_callers
            .iter()
            .map(|s| (*s).to_owned())
            .collect();

        let added: Vec<&String> = observed.difference(&expected).collect();
        let removed: Vec<&String> = expected.difference(&observed).collect();

        if added.is_empty() && removed.is_empty() {
            continue;
        }

        let mut block = format!(
            "v1.x-deferred crate `{}` drift: {} added caller(s), {} \
             removed caller(s):\n",
            pin.crate_dir,
            added.len(),
            removed.len(),
        );
        for path in &added {
            block.push_str(&format!(
                "  + {path}  (added since v1.0 deferral)\n"
            ));
        }
        for path in &removed {
            block.push_str(&format!(
                "  - {path}  (in pin but no longer references {})\n",
                pin.rust_path
            ));
        }
        drift_blocks.push(block);
    }

    if !drift_blocks.is_empty() {
        let combined = drift_blocks.join("\n");
        panic!(
            "\n=== v1.x-deferred caller drift detected ===\n\n{combined}\n\
             Either:\n  \
             (a) Remove the new caller — the substrate is DEFERRED-v1.x \
             per CLAUDE.md `v1.0 strategic decisions`.\n  \
             (b) Update CLAUDE.md to flip the deferral, then update the \
             PINS table in `crates/argo-sandbox/tests/v1_deferred_status_pin.rs`.\n"
        );
    }
}

#[test]
fn pinned_deferred_crates_exist_on_disk() {
    let root = workspace_root();
    let missing: Vec<&str> = PINS
        .iter()
        .filter(|pin| !root.join("crates").join(pin.crate_dir).exists())
        .map(|pin| pin.crate_dir)
        .collect();
    assert!(
        missing.is_empty(),
        "Pinned deferred crate(s) missing from workspace: {missing:?}. \
         If a substrate was intentionally deleted, remove it from PINS \
         and update CLAUDE.md `v1.0 strategic decisions` together."
    );
}

#[test]
fn forbidden_caller_crates_exist_on_disk() {
    let root = workspace_root();
    let missing: Vec<&str> = FORBIDDEN_CALLER_CRATES
        .iter()
        .filter(|caller| !root.join("crates").join(caller).exists())
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "Forbidden caller crate(s) missing from workspace: {missing:?}. \
         Either the crate was renamed (update FORBIDDEN_CALLER_CRATES) or \
         deleted (remove from this list and update the brief's scope)."
    );
}

#[test]
fn path_anchor_avoids_prefix_collision() {
    // Regression guard for the `argo_executor_computer_use::` vs
    // `argo_executor_computer_use_linux::` prefix-collision concern. If
    // `file_references_path` ever loosens the anchor, these assertions
    // catch it before drift detection silently breaks.
    assert!(file_references_path(
        "use argo_executor_computer_use::e2b::E2bClient;",
        "argo_executor_computer_use",
    ));
    assert!(!file_references_path(
        "use argo_executor_computer_use_linux::LinuxComputerUse;",
        "argo_executor_computer_use",
    ));
    assert!(file_references_path(
        "use argo_executor_computer_use_linux::LinuxComputerUse;",
        "argo_executor_computer_use_linux",
    ));
    assert!(file_references_path(
        "extern crate argo_sandbox;",
        "argo_sandbox",
    ));
    assert!(!file_references_path(
        "// unrelated comment mentioning argo_sandboxy somewhere",
        "argo_sandbox",
    ));
}
