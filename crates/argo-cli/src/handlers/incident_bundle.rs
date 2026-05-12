//! `argo incident bundle` — snapshot diagnostic state into a tar.gz
//! for on-call sharing (Stream `mission-incident-response-runbook`).
//! Captures doctor / audit / restarts / metrics / archives / build
//! identity / redacted argo.toml / log tail. Read-only on system
//! state; section failures record `failed:` but do not fail the
//! bundle. Credential redaction matches the cassette convention.

use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_SINCE: &str = "1h";
const REDACTED: &str = "<REDACTED>";

#[derive(Debug, Serialize)]
struct Manifest {
    schema_version: u16,
    generated_at: String,
    argo_version: String,
    since: String,
    sections: Vec<SectionMeta>,
}

#[derive(Debug, Serialize)]
struct SectionMeta {
    path: String,
    status: String,
    bytes: u64,
}

pub struct BundleArgs {
    pub since: String,
    pub output: Option<PathBuf>,
    pub log_tail: Option<PathBuf>,
}

struct Sec {
    path: &'static str,
    body: Vec<u8>,
    status: String,
}

impl Sec {
    fn ok(path: &'static str, body: Vec<u8>) -> Self {
        Self { path, body, status: "ok".into() }
    }
    fn failed(path: &'static str, reason: String) -> Self {
        let body = format!("section capture failed: {reason}\n").into_bytes();
        Self { path, body, status: format!("failed: {reason}") }
    }
}

fn default_output() -> PathBuf {
    PathBuf::from(format!("incident-{}.tar.gz", Utc::now().format("%Y%m%dT%H%M%SZ")))
}

fn capture_subcommand(path: &'static str, args: &[&str]) -> Sec {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return Sec::failed(path, format!("current_exe: {e}")),
    };
    let out = match Command::new(&exe).args(args).output() {
        Ok(o) => o,
        Err(e) => return Sec::failed(path, format!("spawn: {e}")),
    };
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Sec::failed(path, format!("exit {}: {stderr}", out.status));
    }
    Sec::ok(path, out.stdout)
}

fn capture_build_identity() -> Sec {
    let mut body = format!("argo_version={}\n", env!("CARGO_PKG_VERSION"));
    match Command::new("git").args(["rev-parse", "HEAD"]).output() {
        Ok(o) if o.status.success() => {
            body.push_str("git_head=");
            body.push_str(String::from_utf8_lossy(&o.stdout).trim());
            body.push('\n');
        }
        Ok(o) => body.push_str(&format!("git_head=<unavailable: exit {}>\n", o.status)),
        Err(e) => body.push_str(&format!("git_head=<unavailable: {e}>\n")),
    }
    Sec::ok("build-identity.txt", body.into_bytes())
}

fn locate_argo_toml() -> PathBuf {
    if let Ok(home) = std::env::var("ARGO_HOME") {
        let candidate = PathBuf::from(&home).join(".argo").join("argo.toml");
        if candidate.exists() {
            return candidate;
        }
        let direct = PathBuf::from(&home).join("argo.toml");
        if direct.exists() {
            return direct;
        }
    }
    PathBuf::from(".argo/argo.toml")
}

fn capture_argo_toml() -> Sec {
    let path = locate_argo_toml();
    match fs::read_to_string(&path) {
        Ok(raw) => Sec::ok("argo-toml.redacted.toml", redact_toml(&raw).into_bytes()),
        Err(e) => Sec::failed("argo-toml.redacted.toml", format!("read {}: {e}", path.display())),
    }
}

/// Redact sensitive `key = "value"` lines. Case-insensitive
/// substring match on the key against
/// `api_key|secret|password|token|bearer|cred`. Mirrors the cassette
/// fixtures pattern (`argo-test-cassette::redact_body_json`).
pub fn redact_toml(raw: &str) -> String {
    let needles = ["api_key", "secret", "password", "token", "bearer", "cred"];
    let mut out = String::with_capacity(raw.len());
    for line in raw.lines() {
        let trimmed = line.trim_start();
        if let Some(eq) = trimmed.find('=') {
            let key = trimmed[..eq].trim().to_ascii_lowercase();
            if !key.is_empty() && needles.iter().any(|n| key.contains(n)) {
                let lead = line.len() - trimmed.len();
                out.push_str(&line[..lead]);
                out.push_str(trimmed[..eq].trim_end());
                out.push_str(" = \"");
                out.push_str(REDACTED);
                out.push_str("\"\n");
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn capture_log_tail(log_tail: Option<&Path>) -> Sec {
    let Some(path) = log_tail else {
        return Sec::ok("log-tail.txt", b"no log path provided (pass --log-tail <path>)\n".to_vec());
    };
    match fs::read_to_string(path) {
        Ok(raw) => {
            let tail: Vec<&str> = raw.lines().rev().take(1000).collect();
            let body = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
            Sec::ok("log-tail.txt", body.into_bytes())
        }
        Err(e) => Sec::failed("log-tail.txt", format!("read {}: {e}", path.display())),
    }
}

fn append_bytes<W: std::io::Write>(builder: &mut tar::Builder<W>, path: &str, bytes: &[u8]) -> ArgoResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_cksum();
    builder
        .append_data(&mut header, path, bytes)
        .map_err(|e| ArgoError::Io(format!("tar append {path}: {e}")))?;
    Ok(())
}

/// Build the bundle and write the tar.gz. Returns the final path.
pub fn run(args: BundleArgs) -> ArgoResult<PathBuf> {
    let since = if args.since.trim().is_empty() {
        DEFAULT_SINCE.to_string()
    } else {
        args.since
    };
    let output = args.output.unwrap_or_else(default_output);
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| ArgoError::Io(format!("mkdir {}: {e}", parent.display())))?;
    }
    let sections = vec![
        capture_subcommand("doctor.json", &["doctor", "--json"]),
        capture_subcommand("audit-tail.json", &["audit", "query", "--since-minutes", "60", "--json"]),
        capture_subcommand("worker-restarts.json", &["worker", "restarts", "--since", &since, "--json"]),
        capture_subcommand("gateway-metrics.json", &["gateway", "metrics-json"]),
        capture_subcommand("audit-archives.json", &["audit", "list-archives", "--json"]),
        capture_build_identity(),
        capture_argo_toml(),
        capture_log_tail(args.log_tail.as_deref()),
    ];
    let manifest = Manifest {
        schema_version: 1,
        generated_at: Utc::now().to_rfc3339(),
        argo_version: env!("CARGO_PKG_VERSION").to_string(),
        since,
        sections: sections
            .iter()
            .map(|s| SectionMeta { path: s.path.to_string(), status: s.status.clone(), bytes: s.body.len() as u64 })
            .collect(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| ArgoError::Schema(format!("manifest.json: {e}")))?;
    let file = File::create(&output)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", output.display())))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    append_bytes(&mut builder, "manifest.json", &manifest_bytes)?;
    for s in &sections {
        append_bytes(&mut builder, s.path, &s.body)?;
    }
    let encoder = builder.into_inner().map_err(|e| ArgoError::Io(format!("tar finalize: {e}")))?;
    encoder.finish().map_err(|e| ArgoError::Io(format!("gzip finalize: {e}")))?;
    Ok(output)
}
