//! End-to-end smoke for `argo cu replay`. Seeds a 3-step CU
//! trajectory (action ledger row + on-disk `browser.jsonl` +
//! step PNGs), then drives the binary to render markdown + GIF
//! and asserts the outputs.

mod common;

use common::argo_bin;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn run(home: &Path, work: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work)
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_png(path: &Path) {
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([10, 200, 30, 255]));
    img.save(path).expect("write png");
}

fn append_step(body: &mut String, step: u32) {
    let row = json!({
        "step": step,
        "at": format!("2026-05-10T12:00:0{step}Z"),
        "action": {"kind": "click", "x": 10 * step, "y": 20 * step},
        "result": {"ok": true},
        "screenshot_relpath": format!("step-{step:03}.png"),
        "tokens_used": 100 + step,
        "budget": {
            "tokens_used": 100 * step,
            "tokens_cap": 5000,
            "steps_used": step,
            "steps_cap": 10,
            "wallclock_seconds_used": step,
            "wallclock_seconds_cap": 60
        }
    });
    body.push_str(&row.to_string());
    body.push('\n');
}

#[test]
fn cu_replay_writes_markdown_and_gif() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    assert_success(&run(home.path(), work.path(), &["workspace", "init"]));

    let action_id = "act-cu-smoke-1";
    let job_id = "job-cu-smoke";
    let body = json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": job_id,
        "action_type": "open_landing_page",
        "risk_level": "low",
        "requires_approval": false,
        "state": "completed",
        "payload": {"url": "https://example.test"},
        "created_at": "2026-05-10T12:00:00Z"
    })
    .to_string();
    assert_success(&run(
        home.path(),
        work.path(),
        &[
            "memory",
            "append",
            "--vertical",
            "gtm-engineer",
            "memory",
            &body,
        ],
    ));

    let bundle = work
        .path()
        .join(".skycot")
        .join("outputs")
        .join(job_id)
        .join("actions")
        .join(action_id);
    std::fs::create_dir_all(&bundle).unwrap();
    let mut browser = String::new();
    for step in 1..=3 {
        append_step(&mut browser, step);
        write_png(&bundle.join(format!("step-{step:03}.png")));
    }
    std::fs::write(bundle.join("browser.jsonl"), browser).unwrap();

    let md_path = work.path().join("traj.md");
    let gif_path = work.path().join("traj.gif");
    assert_success(&run(
        home.path(),
        work.path(),
        &[
            "cu",
            "replay",
            action_id,
            "--out",
            md_path.to_str().unwrap(),
            "--gif",
            gif_path.to_str().unwrap(),
        ],
    ));

    let md = std::fs::read_to_string(&md_path).unwrap();
    assert!(md.contains("# CU session `act-cu-smoke-1`"), "md: {md}");
    assert!(md.contains("## Step 1 — click (10,20)"), "md: {md}");
    assert!(md.contains("## Step 3 — click (30,60)"), "md: {md}");
    assert!(md.contains("status: **success**"), "md: {md}");

    let gif = std::fs::read(&gif_path).unwrap();
    assert!(gif.starts_with(b"GIF89a") || gif.starts_with(b"GIF87a"));

    // Cross-check the JSON / NDJSON surface in the same fixture so
    // the integration test exercises both formats.
    let ndjson_out = run(
        home.path(),
        work.path(),
        &["cu", "replay", action_id, "--format", "json"],
    );
    assert_success(&ndjson_out);
    let stdout = String::from_utf8(ndjson_out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    // header + 3 steps = 4 NDJSON lines
    assert_eq!(lines.len(), 4, "ndjson lines: {lines:?}");
    let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(header["event"], "session");
    assert_eq!(header["session_id"], action_id);
    assert_eq!(header["steps"], 3);
}
