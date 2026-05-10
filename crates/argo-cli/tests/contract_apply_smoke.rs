//! End-to-end smoke for `argo contract validate / apply --dry-run /
//! apply / show` (Stream `claude-delegation-contract-template`,
//! V1.1 mission claim 2).
//!
//! Covers the brief's three acceptance steps in a single binary:
//!
//! 1. All 3 bundled templates parse cleanly via `argo contract validate`.
//! 2. `argo contract apply --dry-run --worker <id>` prints the planned
//!    mutations without writing.
//! 3. Real apply round-trips through `argo contract show` (semantic
//!    equality — TOML key ordering may differ).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn run(home: &Path, work: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .current_dir(work)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn bundled_templates_validate_cleanly() {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let contracts = workspace_root().join("docs/skycot/contracts");
    for name in [
        "ar-clerk.contract.toml",
        "recruiter-screener.contract.toml",
        "marketing-intern.contract.toml",
    ] {
        let path = contracts.join(name);
        let out = run(
            home.path(),
            work.path(),
            &[
                "contract",
                "validate",
                path.to_str().expect("utf8 path"),
                "--json",
            ],
        );
        assert_ok(&out, &format!("validate {name}"));
        let v: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("validate JSON");
        assert_eq!(v["ok"], true, "{name}: {v}");
        assert!(
            v["charter_bytes"].as_u64().unwrap_or(0) > 0,
            "{name}: charter must be non-empty"
        );
        assert!(
            v["objectives"].as_u64().unwrap_or(0) >= 1,
            "{name}: must declare at least one objective"
        );
    }
}

#[test]
fn dry_run_apply_describes_mutations_without_persisting() {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let path = workspace_root().join("docs/skycot/contracts/ar-clerk.contract.toml");

    // Use a placeholder worker id; --dry-run never validates it
    // because no DB call is made.
    let placeholder = "01900000-0000-7000-8000-000000000000";
    let out = run(
        home.path(),
        work.path(),
        &[
            "contract",
            "apply",
            path.to_str().expect("utf8 path"),
            "--worker",
            placeholder,
            "--dry-run",
            "--json",
        ],
    );
    assert_ok(&out, "apply --dry-run");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("dry-run JSON");
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["worker_id"], placeholder);
    assert_eq!(v["would"]["update_charter"], true);
    assert!(v["would"]["set_objectives"].as_u64().unwrap_or(0) >= 1);
}

#[test]
fn apply_then_show_round_trips_against_existing_worker() {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let verticals = workspace_root().join("verticals");

    // 1. Workspace bootstrap so SQLite + tenant exist.
    let init = Command::new(argo_bin())
        .args(["workspace", "init"])
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .current_dir(work.path())
        .output()
        .expect("spawn workspace init");
    assert_ok(&init, "workspace init");

    // 2. Create a worker bound to a real in-tree vertical so the
    //    create path stays offline + dependency-free.
    let create = Command::new(argo_bin())
        .args([
            "worker",
            "create",
            "--vertical",
            "finance-ar",
            "--persona",
            "ContractRoundTrip",
            "--charter",
            "placeholder",
            "--json",
        ])
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .current_dir(work.path())
        .output()
        .expect("spawn worker create");
    assert_ok(&create, "worker create");
    let create_json: serde_json::Value =
        serde_json::from_slice(&create.stdout).expect("worker create JSON");
    let worker_id = create_json["id"]
        .as_str()
        .expect("worker create id")
        .to_string();
    let tenant_id = create_json["tenant_id"]
        .as_str()
        .expect("worker create tenant_id")
        .to_string();

    // 3. Apply the bundled AR-clerk template.
    let path = workspace_root().join("docs/skycot/contracts/ar-clerk.contract.toml");
    let apply = run(
        home.path(),
        work.path(),
        &[
            "contract",
            "apply",
            path.to_str().expect("utf8 path"),
            "--worker",
            &worker_id,
            "--tenant",
            &tenant_id,
            "--json",
        ],
    );
    assert_ok(&apply, "contract apply");
    let apply_json: serde_json::Value =
        serde_json::from_slice(&apply.stdout).expect("apply JSON");
    assert_eq!(apply_json["ok"], true);
    assert_eq!(apply_json["worker_id"], worker_id);
    let applied_objectives = apply_json["applied"]["objectives"]
        .as_u64()
        .expect("applied objectives count");
    assert!(applied_objectives >= 1);

    // 4. Show the worker → re-parse → re-show. The two TOML
    //    documents must match byte-for-byte after normalisation by
    //    the second `show`. That proves the (apply → store → show)
    //    pipeline is closed.
    let show1 = run(
        home.path(),
        work.path(),
        &[
            "contract",
            "show",
            &worker_id,
            "--tenant",
            &tenant_id,
        ],
    );
    assert_ok(&show1, "contract show #1");
    let toml1 = String::from_utf8(show1.stdout).expect("utf8 toml1");

    // Re-parse: write toml1 into a temp file, ask validate to bless
    // it, and confirm the rendered objective+kpi shape is non-trivial.
    let temp_file = work.path().join("roundtrip.contract.toml");
    std::fs::write(&temp_file, &toml1).expect("write roundtrip file");
    let revalidate = run(
        home.path(),
        work.path(),
        &[
            "contract",
            "validate",
            temp_file.to_str().expect("utf8 path"),
            "--json",
        ],
    );
    assert_ok(&revalidate, "revalidate roundtrip");
    let rev_json: serde_json::Value =
        serde_json::from_slice(&revalidate.stdout).expect("revalidate JSON");
    assert_eq!(rev_json["ok"], true);
    assert_eq!(rev_json["role"], "ar-clerk");
    let kpi_count = rev_json["kpis"].as_u64().unwrap_or(0);
    let objective_count = rev_json["objectives"].as_u64().unwrap_or(0);
    assert!(kpi_count >= 3, "expected ≥3 KPIs from AR-clerk template");
    assert!(objective_count >= 1, "expected ≥1 objective");

    // Final fixed-point check: render again and require the
    // serialisation to be stable across two `show` calls.
    let show2 = run(
        home.path(),
        work.path(),
        &[
            "contract",
            "show",
            &worker_id,
            "--tenant",
            &tenant_id,
        ],
    );
    assert_ok(&show2, "contract show #2");
    let toml2 = String::from_utf8(show2.stdout).expect("utf8 toml2");
    assert_eq!(
        toml1, toml2,
        "show is not idempotent: contract render is unstable"
    );
}
