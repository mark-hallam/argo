//! Cold-boot regression for the canonical `/api/v1/verticals`
//! endpoint. Previously the endpoint joined `workflow_definition_index`
//! and returned an empty array on a fresh tenant, which forced the
//! SPA's `/onboarding/pick-vertical` wizard onto its hardcoded
//! `FALLBACK_VERTICALS` list with the `(offline · N verticals)` hint.
//!
//! After `fix-gateway-verticals-api-v1-endpoint-empty-not-seeded-from-packs`,
//! the endpoint reads the on-disk pack catalogue (same source as the
//! legacy `/verticals`) and returns ≥ N rows whenever the gateway has
//! N packs loaded — no per-tenant install required.
mod common;

use common::{argo_bin, await_listening, body_of, http_get, pick_free_port};
use std::io::BufReader;
use std::path::Path;
use std::process::{Command, Stdio};

// Smoke-test DB key (32 zero bytes base64'd). Matches the constant
// used across the rest of the gateway-smoke test harness.
const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

const MIN_TOML: &str = "schema_version = \"1\"\n\n\
    [vertical]\n\
    id = \"{id}\"\n\
    name = \"{name}\"\n\
    version = \"0.1.0\"\n\
    description = \"{desc}\"\n\
    tags = [\"{id}\"]\n";

fn write_seven_packs(verticals_dir: &Path) {
    let packs = [
        ("gtm-engineer", "GTM Engineer", "Lead enrichment + sequencing."),
        ("marketing-intern", "Marketing Intern", "Content drafting and posts."),
        ("marketplace-seller", "Marketplace Seller", "Inventory and listings."),
        ("recruiter", "Recruiter", "Resume screening + outreach."),
        ("sales-sdr", "Sales SDR", "Outbound prospecting."),
        ("customer-support", "Customer Support", "Ticket triage."),
        ("finance-ar", "Accounts Receivable", "Invoicing + dunning."),
    ];
    for (id, name, desc) in packs {
        let dir = verticals_dir.join(id);
        std::fs::create_dir_all(&dir).unwrap();
        let toml = MIN_TOML
            .replace("{id}", id)
            .replace("{name}", name)
            .replace("{desc}", desc);
        std::fs::write(dir.join("vertical.toml"), toml).unwrap();
    }
}

fn workspace_init(home: &Path, root: &Path) {
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(root)
        .args(["workspace", "init"])
        .output()
        .expect("spawn workspace init");
    assert!(
        out.status.success(),
        "workspace init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn boot_gateway(
    home: &Path,
    workspace_root: &Path,
    verticals: &Path,
    max_requests: u32,
) -> (
    std::process::Child,
    BufReader<std::process::ChildStderr>,
    u16,
) {
    workspace_init(home, workspace_root);
    let port = pick_free_port();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .env("ANTHROPIC_API_KEY", "sk-ant-test-only-not-real")
        .current_dir(workspace_root)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);
    (child, stderr, port)
}

#[test]
fn api_v1_verticals_returns_pack_catalogue_on_cold_boot() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_seven_packs(verticals.path());
    let (mut child, _stderr, port) = boot_gateway(home.path(), root.path(), verticals.path(), 4);

    let resp = http_get(port, "/api/v1/verticals");
    let _ = child.kill();

    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "expected 200 OK from /api/v1/verticals on cold boot, got:\n{resp}"
    );
    let body = body_of(&resp);
    let parsed: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("body not JSON: {e}\nbody={body}"));
    let rows = parsed["verticals"]
        .as_array()
        .unwrap_or_else(|| panic!("expected `verticals` array, got:\n{body}"));
    // Acceptance criterion: returns ≥ 7 verticals.
    assert!(
        rows.len() >= 7,
        "expected ≥ 7 catalogue entries on cold boot, got {} — body={body}",
        rows.len()
    );
    // Sanity: rows carry the fields the SPA's `VerticalRow` shape
    // consumes (`web/lib/api/verticals.ts`).
    let first = &rows[0];
    for field in [
        "id",
        "name",
        "version",
        "description",
        "headline",
        "tags",
        "workflow_count",
        "action_count",
        "role_label",
        "kpi_count",
        "skill_count",
    ] {
        assert!(
            first.get(field).is_some(),
            "missing field `{field}` on row: {first}"
        );
    }
}
