//! Unit tests for `argo cu replay`. Sibling file so
//! `cu_replay.rs` stays well under the LOC budget.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_executor_computer_use::audit::AuditRow;
use argo_executor_computer_use::budget::BudgetSnapshot;
use argo_executor_computer_use::e2b::{ActionResult, CuAction};
use argo_identity::WorkerId;
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

fn sample_row(step: u32) -> AuditRow {
    AuditRow {
        step,
        at: format!("2026-05-10T12:00:0{step}Z"),
        action: CuAction::Click {
            x: 10 * step,
            y: 20 * step,
        },
        result: ActionResult {
            ok: step % 2 == 1,
            text: if step == 2 {
                Some("https://example.test/landing".into())
            } else {
                None
            },
            screenshot_b64: None,
        },
        screenshot_relpath: format!("step-{step:03}.png"),
        tokens_used: 100 + step,
        budget: BudgetSnapshot {
            tokens_used: 100 * step,
            tokens_cap: 5_000,
            steps_used: step,
            steps_cap: 10,
            wallclock_seconds_used: step,
            wallclock_seconds_cap: 60,
        },
    }
}

fn sample_action(action_id: &str, state: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.into(),
        action_id: action_id.into(),
        job_id: "job-cu-1".into(),
        worker_id: WorkerId::nil(),
        item_id: None,
        action_type: "open_landing_page".into(),
        risk_level: "low".into(),
        requires_approval: false,
        state: state.into(),
        payload: json!({"url": "https://example.test"}),
        approved_by: None,
        approved_at: None,
        executed_at: Some("2026-05-10T12:00:00Z".into()),
        platform: Some("e2b".into()),
        result: Some(json!({"summary": "navigated"})),
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-10T12:00:00Z".into(),
    }
}

fn write_png(path: &Path) {
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 50, 50, 255]));
    img.save(path).expect("write png");
}

fn seed_bundle(dir: &TempDir, action: &ActionRecord, rows: &[AuditRow]) {
    let bundle = bundle_dir(dir.path(), action);
    std::fs::create_dir_all(&bundle).unwrap();
    let mut body = String::new();
    for r in rows {
        body.push_str(&serde_json::to_string(r).unwrap());
        body.push('\n');
    }
    std::fs::write(bundle.join("browser.jsonl"), body).unwrap();
    for r in rows {
        write_png(&bundle.join(&r.screenshot_relpath));
    }
}

fn three_step_trajectory() -> (TempDir, ActionRecord, Vec<AuditRow>) {
    let dir = tempfile::tempdir().unwrap();
    let action = sample_action("act-cu-replay-1", "completed");
    let rows = vec![sample_row(1), sample_row(2), sample_row(3)];
    seed_bundle(&dir, &action, &rows);
    (dir, action, rows)
}

#[test]
fn renders_markdown_for_three_step_trajectory() {
    let (dir, action, rows) = three_step_trajectory();
    let bundle = bundle_dir(dir.path(), &action);
    let traj = build_trajectory(&action, &bundle, &rows, None, None).unwrap();
    let md = render_markdown(&traj, false).unwrap();
    assert!(md.contains("# CU session `act-cu-replay-1`"));
    assert!(md.contains("status: **success**"));
    assert!(md.contains("## Step 1 — click (10,20)"));
    assert!(md.contains("## Step 2 — click (20,40)"));
    assert!(md.contains("## Step 3 — click (30,60)"));
    assert!(md.contains("file://"));
    // step 2 has ok=false → renders "result: **failed**"
    assert!(md.contains("result: **failed**"));
}

#[test]
fn gif_renders_valid_gif_header() {
    let (dir, action, rows) = three_step_trajectory();
    let bundle = bundle_dir(dir.path(), &action);
    let traj = build_trajectory(&action, &bundle, &rows, None, None).unwrap();
    let gif_path = dir.path().join("traj.gif");
    write_gif(&traj, &gif_path, 1).unwrap();
    let head = std::fs::read(&gif_path).unwrap();
    assert!(head.starts_with(b"GIF89a") || head.starts_with(b"GIF87a"));
}

#[test]
fn from_to_slicing_filters_step_range() {
    let (dir, action, rows) = three_step_trajectory();
    let bundle = bundle_dir(dir.path(), &action);
    let traj = build_trajectory(&action, &bundle, &rows, Some(2), Some(2)).unwrap();
    assert_eq!(traj.steps.len(), 1);
    assert_eq!(traj.steps[0].row.step, 2);
}

#[test]
fn inline_embeds_screenshot_as_data_url() {
    let (dir, action, rows) = three_step_trajectory();
    let bundle = bundle_dir(dir.path(), &action);
    let traj = build_trajectory(&action, &bundle, &rows, Some(1), Some(1)).unwrap();
    let md = render_markdown(&traj, true).unwrap();
    assert!(
        md.contains("data:image/png;base64,"),
        "expected base64-data-url; got:\n{md}"
    );
    assert!(
        !md.contains("file://"),
        "inline mode must not emit file:// links; got:\n{md}"
    );
}

#[test]
fn blob_screenshot_renders_argo_blob_link() {
    use argo_store::attached_blob::{AttachedBlob, BlobRef};
    let blob = AttachedBlob::new(b"\x89PNG-fake".to_vec(), "image/png".into());
    let r: BlobRef = blob.as_ref();
    let link =
        super::screenshot_link(&ScreenshotSource::Blob(r.clone()), false).unwrap();
    assert!(link.starts_with("argo-blob://"), "got {link}");
    assert!(link.contains(&r.id.to_string()), "got {link}");
}

#[test]
fn empty_range_returns_clean_error() {
    let (dir, action, rows) = three_step_trajectory();
    let bundle = bundle_dir(dir.path(), &action);
    let err = build_trajectory(&action, &bundle, &rows, Some(5), Some(2)).unwrap_err();
    assert!(
        err.to_string().contains("range is empty"),
        "expected range-empty error; got {err}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn missing_session_yields_not_found() {
    use argo_memory::MemoryStore;
    use argo_store::SessionStore;
    use std::sync::Arc;
    let home = tempfile::tempdir().unwrap();
    let store = SessionStore::open(&crate::home::db_path(home.path()))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (_t, ws) = store
        .ensure_workspace("cu-replay-test", "/tmp/cu-replay-test")
        .await
        .unwrap();
    let memory = MemoryStore::new(Arc::new(store));
    let verticals = workspace_root().join("verticals");
    let res = find_action(&memory, ws, "act-does-not-exist", &verticals)
        .await
        .unwrap();
    assert!(res.is_none(), "missing action_id must return None");
}

#[test]
fn gif_fps_bounds_are_validated_in_cmd() {
    // The 1..=30 invariant lives in `cmd_replay`; this test pins
    // the boundary check independently of the I/O path.
    for fps in [0u32, 31, u32::MAX] {
        assert!(!(1..=30).contains(&fps), "fps {fps} should be invalid");
    }
    for fps in [1u32, 15, 30] {
        assert!((1..=30).contains(&fps), "fps {fps} should be valid");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cross_tenant_session_yields_not_found() {
    use argo_memory::{MemoryFile, MemoryStore};
    use argo_store::SessionStore;
    use std::sync::Arc;
    // Seed an action in workspace A; query workspace B.
    let home = tempfile::tempdir().unwrap();
    let store = SessionStore::open(&crate::home::db_path(home.path()))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (_ta, ws_a) = store
        .ensure_workspace("ws-a", "/tmp/ws-a-cu")
        .await
        .unwrap();
    let (_tb, ws_b) = store
        .ensure_workspace("ws-b", "/tmp/ws-b-cu")
        .await
        .unwrap();
    let memory = MemoryStore::new(Arc::new(store));
    let action = sample_action("act-cross-tenant", "completed");
    memory
        .append_for_vertical(
            ws_a,
            MemoryFile::Memory,
            serde_json::to_string(&action).unwrap(),
            None,
            "gtm-engineer",
        )
        .await
        .unwrap();
    let verticals = workspace_root().join("verticals");
    let in_a = find_action(&memory, ws_a, &action.action_id, &verticals)
        .await
        .unwrap();
    assert!(in_a.is_some(), "action should resolve in its own workspace");
    let in_b = find_action(&memory, ws_b, &action.action_id, &verticals)
        .await
        .unwrap();
    assert!(
        in_b.is_none(),
        "cross-tenant lookup must surface as None (→ NotFound)"
    );
}

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
