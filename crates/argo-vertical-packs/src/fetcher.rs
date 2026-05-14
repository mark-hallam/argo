//! GitHub Release and local archive fetch support.

use crate::manifest::{render_template, FetcherConfig};
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

/// A request to download one pack archive.
#[derive(Debug, Clone)]
pub struct FetchRequest {
    /// Pack name.
    pub name: String,
    /// Pack version.
    pub version: String,
    /// Fetcher configuration.
    pub config: FetcherConfig,
}

/// Fetches a pack archive into a local path.
///
/// ```
/// use argo_vertical_packs::PackFetcher;
/// fn accepts_fetcher(_: &dyn PackFetcher) {}
/// ```
pub trait PackFetcher {
    /// Fetch the archive to `dest`.
    fn fetch<'a>(
        &'a self,
        request: &'a FetchRequest,
        dest: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ArgoResult<()>> + Send + 'a>>;
}

/// HTTP fetcher for GitHub release tarballs.
pub struct GitHubReleaseFetcher {
    client: reqwest::Client,
}

impl GitHubReleaseFetcher {
    /// Build a fetcher with the default reqwest client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for GitHubReleaseFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl PackFetcher for GitHubReleaseFetcher {
    fn fetch<'a>(
        &'a self,
        request: &'a FetchRequest,
        dest: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ArgoResult<()>> + Send + 'a>> {
        Box::pin(async move {
            let url = request.url();
            if let Some(local) = url.strip_prefix("file://") {
                std::fs::copy(local, dest).map_err(|e| {
                    ArgoError::Io(format!(
                        "copy local archive {local} to {}: {e}",
                        dest.display()
                    ))
                })?;
                return Ok(());
            }

            let mut http = self.client.get(url);
            if let Some(token) = request.github_token() {
                http = http.bearer_auth(token);
            }
            let response = http
                .send()
                .await
                .map_err(|e| ArgoError::Provider(format!("download pack archive: {e}")))?;
            if !response.status().is_success() {
                return Err(ArgoError::Provider(format!(
                    "download pack archive: HTTP {}",
                    response.status()
                )));
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|e| ArgoError::Provider(format!("read pack archive body: {e}")))?;
            tokio::fs::write(dest, bytes)
                .await
                .map_err(|e| ArgoError::Io(format!("write {}: {e}", dest.display())))
        })
    }
}

impl FetchRequest {
    /// Construct the release archive URL.
    pub fn url(&self) -> String {
        let asset = render_template(&self.config.asset_template, &self.name, &self.version, "");
        if self.config.base_url.contains('{') {
            return render_template(&self.config.base_url, &self.name, &self.version, &asset);
        }
        let base = self.config.base_url.trim_end_matches('/');
        let tag = render_template(&self.config.tag_template, &self.name, &self.version, &asset);
        format!("{base}/{tag}/{asset}")
    }

    /// Return the first configured GitHub token present in the environment.
    pub fn github_token(&self) -> Option<String> {
        self.config
            .token_env
            .iter()
            .find_map(|key| std::env::var(key).ok().filter(|v| !v.trim().is_empty()))
    }

    /// Default archive filename for cache staging.
    pub fn archive_name(&self) -> PathBuf {
        PathBuf::from(render_template(
            &self.config.asset_template,
            &self.name,
            &self.version,
            "",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_default_github_release_url() {
        let request = FetchRequest {
            name: "ap".into(),
            version: "1.2.3".into(),
            config: FetcherConfig::default(),
        };
        assert!(request.url().ends_with("/ap-v1.2.3/ap-1.2.3.tar.gz"));
    }

    #[test]
    fn direct_template_url_replaces_placeholders() {
        let request = FetchRequest {
            name: "ap".into(),
            version: "1".into(),
            config: FetcherConfig {
                base_url: "file:///tmp/{asset}".into(),
                ..FetcherConfig::default()
            },
        };
        assert_eq!(request.url(), "file:///tmp/ap-1.tar.gz");
    }
}
