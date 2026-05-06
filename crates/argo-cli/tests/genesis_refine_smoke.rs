use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

fn argo_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../target/debug/argo");
    p
}

fn serve_anthropic_once() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0; 8192];
        let n = stream.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        assert!(req.contains("x-api-key: test_key"));
        assert!(req.contains("POST /v1/messages "));
        let refine_json = serde_json::json!({
            "archetype_confirmed": true,
            "archetype_suggested": null,
            "additional_actions": [],
            "system_prompt_addendum": "Use sharper launch-readiness checks."
        })
        .to_string();
        let body = serde_json::json!({
            "content": [{"type": "text", "text": refine_json}]
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    format!("http://{addr}")
}

#[test]
fn genesis_refine_dry_run_json_prints_diff() {
    let bin = argo_bin();
    if !bin.exists() {
        eprintln!("skipping: {} not built", bin.display());
        return;
    }
    let workspace = tempfile::tempdir().unwrap();
    let verticals = workspace.path().join("verticals");
    std::fs::create_dir_all(&verticals).unwrap();
    let plan = argo_genesis::plan("be my marketing intern", "maya").unwrap();
    let out_dir = verticals.join(&plan.vertical_id);
    argo_genesis::write_vertical(&plan, &out_dir).unwrap();
    let base = serve_anthropic_once();
    let out = Command::new(bin)
        .args([
            "genesis",
            "refine",
            &plan.vertical_id,
            "--dry-run",
            "--no-judge",
            "--max-iterations",
            "1",
            "--json",
        ])
        .env("ARGO_VERTICALS_DIR", &verticals)
        .env("ANTHROPIC_API_KEY", "test_key")
        .env("SKYCOT_REFINE_BASE_URL", base)
        .current_dir(workspace.path())
        .output()
        .expect("spawn argo");
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["genesis_refine"]["dry_run"], true);
    assert_eq!(json["genesis_refine"]["judge"], false);
    assert_eq!(json["genesis_refine"]["changed"], true);
    assert!(json["genesis_refine"]["diff"]
        .as_str()
        .unwrap()
        .contains("Refined addendum"));
    assert!(
        out_dir.exists(),
        "dry-run should not move the original vertical"
    );
}
