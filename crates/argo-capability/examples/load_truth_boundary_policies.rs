//! Sanity-check example: load a truth-boundary policy file from disk
//! and print a one-line summary. Useful for editors authoring new
//! tenant policies.
//!
//! ```text
//! cargo run -p argo-capability --example load_truth_boundary_policies -- \
//!   policies/truth-boundary/skycot-public-pitch.toml
//! ```

use argo_capability::truth_boundary_policies::load_policy;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(arg) = std::env::args().nth(1) else {
        eprintln!("usage: load_truth_boundary_policies <path-to-policy.toml>");
        return ExitCode::from(2);
    };
    let path = PathBuf::from(arg);
    match load_policy(&path) {
        Ok(p) => {
            println!(
                "loaded policy id={} tenant={} classes={}",
                p.id,
                p.tenant_id,
                p.claim_classes.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
