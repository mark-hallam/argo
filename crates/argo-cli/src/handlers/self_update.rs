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
//!
//! Post-fetch install pipeline (download, signature verification,
//! binary replacement, daemon restart, version comparison, output
//! emission) lives in the sibling `self_update_install` module to
//! keep both files under the 500-LOC ceiling.

use argo_types::{ArgoError, ArgoResult};

pub use super::self_update_install::{is_newer, parse_pubkey_pem, verify_signature};

use super::self_update_install::{
    download_to, emit_check, emit_installed, emit_noop, replace_binary, restart_daemon_best_effort,
    RELEASE_PUBKEY_PEM,
};

const GITHUB_API_RELEASES: &str = "https://api.github.com/repos/mark-hallam/Argo-Agent/releases";
pub(crate) const USER_AGENT: &str = concat!("argo-cli/", env!("CARGO_PKG_VERSION"));

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

    emit_installed(&current, &latest.tag, channel, restarted, args.json);
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
        let pre = item
            .get("prerelease")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
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
