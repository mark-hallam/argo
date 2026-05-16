//! Install pipeline for `argo self-update` — asset download, signature
//! verification against the embedded release pubkey, atomic binary
//! replacement, best-effort daemon restart, version comparison, and
//! human/JSON output emission. Split from `self_update.rs` to keep
//! that file under the 500-LOC ceiling.

use super::self_update::{Channel, USER_AGENT};
use argo_types::{ArgoError, ArgoResult};
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::json;
use std::path::{Path, PathBuf};

const PUBKEY_HEADER: &str = "-----BEGIN ARGO RELEASE PUBKEY-----";
const PUBKEY_FOOTER: &str = "-----END ARGO RELEASE PUBKEY-----";

pub(crate) const RELEASE_PUBKEY_PEM: &[u8] = include_bytes!("../../assets/release-pubkey.pem");

pub(crate) async fn download_to(url: &str, dest: &Path) -> ArgoResult<()> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| ArgoError::Provider(format!("self-update: http client: {e}")))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| ArgoError::Provider(format!("self-update: download {url}: {e}")))?;
    if !resp.status().is_success() {
        return Err(ArgoError::Provider(format!(
            "self-update: download {url}: HTTP {}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ArgoError::Provider(format!("self-update: read bytes: {e}")))?;
    std::fs::write(dest, &bytes)
        .map_err(|e| ArgoError::Provider(format!("self-update: write tempfile: {e}")))?;
    Ok(())
}

pub fn parse_pubkey_pem(pem: &[u8]) -> ArgoResult<VerifyingKey> {
    let text = std::str::from_utf8(pem)
        .map_err(|e| ArgoError::Provider(format!("self-update: pubkey utf8: {e}")))?;
    let mut body = String::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line == PUBKEY_HEADER {
            inside = true;
            continue;
        }
        if line == PUBKEY_FOOTER {
            break;
        }
        if inside && !line.is_empty() {
            body.push_str(line);
        }
    }
    if body.is_empty() {
        return Err(ArgoError::Provider(
            "self-update: pubkey PEM missing ARGO RELEASE PUBKEY block".to_string(),
        ));
    }
    let raw = base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .map_err(|e| ArgoError::Provider(format!("self-update: pubkey base64: {e}")))?;
    let arr: [u8; 32] = raw
        .try_into()
        .map_err(|_| ArgoError::Provider("self-update: pubkey not 32 bytes".to_string()))?;
    VerifyingKey::from_bytes(&arr)
        .map_err(|e| ArgoError::Provider(format!("self-update: pubkey curve: {e}")))
}

pub fn verify_signature(pubkey: &VerifyingKey, message: &[u8], signature: &[u8]) -> ArgoResult<()> {
    let arr: [u8; 64] = signature.try_into().map_err(|_| {
        ArgoError::Provider(format!(
            "self-update: signature not 64 bytes (got {})",
            signature.len()
        ))
    })?;
    let sig = Signature::from_bytes(&arr);
    pubkey
        .verify(message, &sig)
        .map_err(|e| ArgoError::Provider(format!("self-update: signature verify failed: {e}")))
}

pub(crate) fn replace_binary(new_archive: &Path, current_exe: &Path) -> ArgoResult<()> {
    let dir = current_exe.parent().ok_or_else(|| {
        ArgoError::Provider(format!(
            "self-update: current_exe has no parent: {}",
            current_exe.display()
        ))
    })?;
    let staged = extract_binary(new_archive, dir)?;
    let backup = dir.join(format!(
        "{}.bak",
        current_exe
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("argo")
    ));
    if backup.exists() {
        let _ = std::fs::remove_file(&backup);
    }
    std::fs::rename(current_exe, &backup)
        .map_err(|e| ArgoError::Provider(format!("self-update: backup current binary: {e}")))?;
    if let Err(e) = std::fs::rename(&staged, current_exe) {
        let _ = std::fs::rename(&backup, current_exe);
        return Err(ArgoError::Provider(format!(
            "self-update: install new binary: {e}"
        )));
    }
    let _ = std::fs::remove_file(&backup);
    Ok(())
}

fn extract_binary(archive: &Path, into_dir: &Path) -> ArgoResult<PathBuf> {
    let f = std::fs::File::open(archive)
        .map_err(|e| ArgoError::Provider(format!("self-update: open archive: {e}")))?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut tar = tar::Archive::new(gz);
    let staged = into_dir.join(".argo.self-update.staged");
    if staged.exists() {
        let _ = std::fs::remove_file(&staged);
    }
    let entries = tar
        .entries()
        .map_err(|e| ArgoError::Provider(format!("self-update: read tar: {e}")))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|e| ArgoError::Provider(format!("self-update: tar entry: {e}")))?;
        let path = entry
            .path()
            .map_err(|e| ArgoError::Provider(format!("self-update: tar path: {e}")))?
            .into_owned();
        if path.file_name().and_then(|s| s.to_str()) == Some("argo") {
            entry
                .unpack(&staged)
                .map_err(|e| ArgoError::Provider(format!("self-update: unpack: {e}")))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755);
                std::fs::set_permissions(&staged, perms)
                    .map_err(|e| ArgoError::Provider(format!("self-update: chmod staged: {e}")))?;
            }
            return Ok(staged);
        }
    }
    Err(ArgoError::Provider(
        "self-update: archive does not contain an 'argo' binary".to_string(),
    ))
}

pub(crate) fn restart_daemon_best_effort(exe: &Path) -> bool {
    let Ok(home) = std::env::var("ARGO_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".argo")))
    else {
        return false;
    };
    let pid_file = home.join("gateway.pid");
    let Ok(pid_str) = std::fs::read_to_string(&pid_file) else {
        return false;
    };
    let Ok(pid) = pid_str.trim().parse::<i32>() else {
        return false;
    };
    #[cfg(unix)]
    // SAFETY: pid is parsed as i32 from our gateway PID file; signal 0
    // probes liveness and SIGTERM is a valid libc signal constant.
    unsafe {
        if libc::kill(pid, 0) != 0 {
            return false;
        }
        let _ = libc::kill(pid, libc::SIGTERM);
    }
    let _ = std::fs::remove_file(&pid_file);
    std::process::Command::new(exe)
        .args(["gateway", "start"])
        .spawn()
        .is_ok()
}

#[cfg(not(unix))]
#[allow(dead_code)]
mod libc {
    pub const SIGTERM: i32 = 15;
    pub unsafe fn kill(_pid: i32, _sig: i32) -> i32 {
        -1
    }
}

pub fn is_newer(latest_tag: &str, current_version: &str) -> bool {
    let latest = strip_v(latest_tag);
    let cur = strip_v(current_version);
    match (parse_semver(latest), parse_semver(cur)) {
        (Some(l), Some(c)) => l > c,
        _ => latest != cur,
    }
}

fn strip_v(s: &str) -> &str {
    s.strip_prefix('v').unwrap_or(s)
}

fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch_str = parts.next()?;
    let patch = patch_str.split('-').next()?.parse().ok()?;
    Some((major, minor, patch))
}

pub(crate) fn emit_check(
    current: &str,
    latest: &str,
    channel: Channel,
    update_available: bool,
    as_json: bool,
) {
    if as_json {
        println!(
            "{}",
            json!({
                "current": current,
                "latest": latest,
                "channel": format!("{channel:?}").to_lowercase(),
                "update_available": update_available,
                "signature": "verified-mandatory",
            })
        );
    } else if update_available {
        println!("argo {current} → {latest} ({channel:?}) — run `argo self-update` to install.");
    } else {
        println!("argo {current} is already the latest on the {channel:?} channel.");
    }
}

pub(crate) fn emit_noop(current: &str, as_json: bool) {
    if as_json {
        println!(
            "{}",
            json!({
                "current": current,
                "latest": current,
                "update_available": false,
                "installed": false,
            })
        );
    } else {
        println!("argo {current} is already the latest. Pass --force to reinstall.");
    }
}

pub(crate) fn emit_installed(
    previous: &str,
    new_tag: &str,
    channel: Channel,
    daemon_restarted: bool,
    as_json: bool,
) {
    if as_json {
        println!(
            "{}",
            json!({
                "previous": previous,
                "installed": new_tag,
                "channel": format!("{channel:?}").to_lowercase(),
                "signature": "verified",
                "daemon_restarted": daemon_restarted,
            })
        );
    } else {
        println!(
            "argo {previous} → {new_tag} ({channel:?}) installed; signature verified; daemon_restarted={daemon_restarted}"
        );
    }
}
