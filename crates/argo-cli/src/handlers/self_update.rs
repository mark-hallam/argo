//! `argo self-update` — verified binary upgrade.
//!
//! Queries GitHub Releases for a newer build matching the current
//! platform, downloads the asset + detached Ed25519 signature,
//! verifies against the embedded release public key, and atomically
//! replaces the running binary. If the gateway daemon was running,
//! it is best-effort restarted afterward.
//!
//! Signature verification is **mandatory**: there is no opt-out flag.
//! `--force` bypasses the "already on latest" short-circuit but never
//! the signature check.
//!
//! The embedded pubkey lives at
//! `crates/argo-cli/assets/release-pubkey.pem` and ships in every
//! binary via `include_bytes!`. Operators rotate it by replacing the
//! file and shipping a new build.

use argo_types::{ArgoError, ArgoResult};
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::json;
use std::path::{Path, PathBuf};

const RELEASE_PUBKEY_PEM: &[u8] =
    include_bytes!("../../assets/release-pubkey.pem");

const PUBKEY_HEADER: &str = "-----BEGIN ARGO RELEASE PUBKEY-----";
const PUBKEY_FOOTER: &str = "-----END ARGO RELEASE PUBKEY-----";

const GITHUB_API_RELEASES: &str =
    "https://api.github.com/repos/mark-hallam/Argo-Agent/releases";
const USER_AGENT: &str = concat!("argo-cli/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

impl Channel {
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "stable" => Ok(Channel::Stable),
            "beta" => Ok(Channel::Beta),
            other => Err(ArgoError::Provider(format!(
                "self-update: unknown channel '{other}' (want 'stable' or 'beta')"
            ))),
        }
    }

    pub fn allows_prerelease(self) -> bool {
        matches!(self, Channel::Beta)
    }
}

#[derive(Debug, Clone)]
pub struct Args {
    pub check: bool,
    pub channel: String,
    pub force: bool,
    pub json: bool,
}

pub async fn run(args: Args) -> ArgoResult<()> {
    let channel = Channel::parse(&args.channel)?;
    let current = env!("CARGO_PKG_VERSION").to_string();

    if args.check {
        let latest = fetch_latest_release(channel).await?;
        let update_available = is_newer(&latest.tag, &current);
        emit_check(&current, &latest.tag, channel, update_available, args.json);
        return Ok(());
    }

    let latest = fetch_latest_release(channel).await?;
    let update_available = is_newer(&latest.tag, &current);
    if !update_available && !args.force {
        emit_noop(&current, args.json);
        return Ok(());
    }

    let target = expected_asset_name();
    let asset = pick_asset(&latest.assets, &target)?;
    let sig_asset = pick_signature_asset(&latest.assets, &target).ok_or_else(|| {
        ArgoError::Provider(format!(
            "self-update: no detached signature ({target}.sig) for asset {target} in release {}",
            latest.tag
        ))
    })?;

    let tmp_dir = tempfile::tempdir()
        .map_err(|e| ArgoError::Provider(format!("self-update: tempdir: {e}")))?;
    let asset_path = tmp_dir.path().join(&target);
    let sig_path = tmp_dir.path().join(format!("{target}.sig"));
    download_to(&asset.url, &asset_path).await?;
    download_to(&sig_asset.url, &sig_path).await?;

    let asset_bytes = std::fs::read(&asset_path)
        .map_err(|e| ArgoError::Provider(format!("self-update: read asset: {e}")))?;
    let sig_bytes = std::fs::read(&sig_path)
        .map_err(|e| ArgoError::Provider(format!("self-update: read sig: {e}")))?;
    let pubkey = parse_pubkey_pem(RELEASE_PUBKEY_PEM)?;
    verify_signature(&pubkey, &asset_bytes, &sig_bytes)?;

    let current_exe = std::env::current_exe()
        .map_err(|e| ArgoError::Provider(format!("self-update: current_exe: {e}")))?;
    replace_binary(&asset_path, &current_exe)?;
    let restarted = restart_daemon_best_effort(&current_exe);

    emit_installed(
        &current,
        &latest.tag,
        channel,
        restarted,
        args.json,
    );
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct Release {
    pub tag: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone)]
pub(crate) struct Asset {
    pub name: String,
    pub url: String,
}

async fn fetch_latest_release(channel: Channel) -> ArgoResult<Release> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| ArgoError::Provider(format!("self-update: http client: {e}")))?;
    let resp = client
        .get(GITHUB_API_RELEASES)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| ArgoError::Provider(format!("self-update: github releases: {e}")))?;
    if !resp.status().is_success() {
        return Err(ArgoError::Provider(format!(
            "self-update: github releases: HTTP {}",
            resp.status()
        )));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ArgoError::Provider(format!("self-update: parse releases: {e}")))?;
    parse_releases(&body, channel)
}

pub(crate) fn parse_releases(body: &serde_json::Value, channel: Channel) -> ArgoResult<Release> {
    let arr = body.as_array().ok_or_else(|| {
        ArgoError::Provider("self-update: github releases: not a JSON array".to_string())
    })?;
    let allow_pre = channel.allows_prerelease();
    for item in arr {
        let draft = item.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
        let pre = item.get("prerelease").and_then(|v| v.as_bool()).unwrap_or(false);
        if draft {
            continue;
        }
        if pre && !allow_pre {
            continue;
        }
        let tag = item
            .get("tag_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ArgoError::Provider("self-update: release missing tag_name".to_string())
            })?
            .to_string();
        let assets = item
            .get("assets")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|asset| {
                        let name = asset.get("name").and_then(|v| v.as_str())?.to_string();
                        let url = asset
                            .get("browser_download_url")
                            .and_then(|v| v.as_str())?
                            .to_string();
                        Some(Asset { name, url })
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Ok(Release { tag, assets });
    }
    Err(ArgoError::Provider(format!(
        "self-update: no releases for channel {channel:?}"
    )))
}

pub(crate) fn pick_asset(assets: &[Asset], target: &str) -> ArgoResult<Asset> {
    assets
        .iter()
        .find(|a| a.name == target)
        .cloned()
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "self-update: no asset named {target} in release (have: {:?})",
                assets.iter().map(|a| &a.name).collect::<Vec<_>>()
            ))
        })
}

pub(crate) fn pick_signature_asset(assets: &[Asset], target: &str) -> Option<Asset> {
    let want = format!("{target}.sig");
    assets.iter().find(|a| a.name == want).cloned()
}

pub fn expected_asset_name() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = std::env::consts::ARCH;
    format!("argo-{os}-{arch}.tar.gz")
}

async fn download_to(url: &str, dest: &Path) -> ArgoResult<()> {
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

pub fn verify_signature(
    pubkey: &VerifyingKey,
    message: &[u8],
    signature: &[u8],
) -> ArgoResult<()> {
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

fn replace_binary(new_archive: &Path, current_exe: &Path) -> ArgoResult<()> {
    let dir = current_exe.parent().ok_or_else(|| {
        ArgoError::Provider(format!(
            "self-update: current_exe has no parent: {}",
            current_exe.display()
        ))
    })?;
    let staged = extract_binary(new_archive, dir)?;
    let backup = dir.join(format!(
        "{}.bak",
        current_exe.file_name().and_then(|s| s.to_str()).unwrap_or("argo")
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
        let mut entry = entry
            .map_err(|e| ArgoError::Provider(format!("self-update: tar entry: {e}")))?;
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
                std::fs::set_permissions(&staged, perms).map_err(|e| {
                    ArgoError::Provider(format!("self-update: chmod staged: {e}"))
                })?;
            }
            return Ok(staged);
        }
    }
    Err(ArgoError::Provider(
        "self-update: archive does not contain an 'argo' binary".to_string(),
    ))
}

fn restart_daemon_best_effort(exe: &Path) -> bool {
    let Ok(home) = std::env::var("ARGO_HOME").map(PathBuf::from).or_else(|_| {
        std::env::var("HOME").map(|h| PathBuf::from(h).join(".argo"))
    }) else {
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

fn emit_check(
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

fn emit_noop(current: &str, as_json: bool) {
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

fn emit_installed(
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

