//! Pack install, cache, and resolution logic.

use crate::dev_link::DevLinkStore;
use crate::fetcher::{FetchRequest, GitHubReleaseFetcher, PackFetcher};
use crate::manifest::{FetcherConfig, PackManifest};
use argo_types::{ArgoError, ArgoResult};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Loader configuration.
#[derive(Debug, Clone)]
pub struct PackLoaderConfig {
    /// Cache root, normally `~/.argo/packs`.
    pub cache_root: PathBuf,
    /// Release fetcher configuration.
    pub fetcher: FetcherConfig,
}

/// Installed or resolved pack row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPack {
    /// Pack name.
    pub name: String,
    /// Pack version.
    pub version: String,
    /// Root directory used at runtime.
    pub root: PathBuf,
    /// Whether this row is backed by a dev-link.
    pub dev_link: bool,
}

/// Vertical template pack loader.
///
/// ```
/// use argo_vertical_packs::{PackLoader, PackLoaderConfig};
/// let root = tempfile::tempdir().unwrap();
/// let loader = PackLoader::new(PackLoaderConfig::with_cache_dir(root.path().into()));
/// assert!(loader.list().unwrap().is_empty());
/// ```
pub struct PackLoader<F = GitHubReleaseFetcher> {
    config: PackLoaderConfig,
    fetcher: F,
}

impl PackLoaderConfig {
    /// Use the default cache root and GitHub Releases fetcher config.
    pub fn default_from_env() -> ArgoResult<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| ArgoError::Io("HOME is not set; cannot resolve ~/.argo/packs".into()))?;
        Ok(Self::with_cache_dir(home.join(".argo").join("packs")))
    }

    /// Build config for a specific cache root.
    pub fn with_cache_dir(cache_root: PathBuf) -> Self {
        let config_file = cache_root.join("config.toml");
        let fetcher = load_fetcher_config(&config_file).unwrap_or_default();
        Self {
            cache_root,
            fetcher,
        }
    }
}

impl PackLoader<GitHubReleaseFetcher> {
    /// Build a loader with the default GitHub Releases fetcher.
    pub fn new(config: PackLoaderConfig) -> Self {
        Self::with_fetcher(config, GitHubReleaseFetcher::default())
    }

    /// Build a loader from the default environment.
    pub fn default_loader() -> ArgoResult<Self> {
        Ok(Self::new(PackLoaderConfig::default_from_env()?))
    }
}

impl<F: PackFetcher> PackLoader<F> {
    /// Build a loader with an explicit fetcher.
    pub fn with_fetcher(config: PackLoaderConfig, fetcher: F) -> Self {
        Self { config, fetcher }
    }

    /// Install `name` at `version`, reusing the cache unless `force` is set.
    pub async fn install(
        &self,
        name: &str,
        version: &str,
        base_url: Option<&str>,
        force: bool,
    ) -> ArgoResult<InstalledPack> {
        let root = self.pack_root(name, version);
        if root.join("pack.toml").is_file() && !force {
            return Ok(InstalledPack::cached(name, version, root));
        }
        let request = self.request(name, version, base_url);
        std::fs::create_dir_all(&self.config.cache_root)?;
        let archive = self.config.cache_root.join(request.archive_name());
        self.fetcher.fetch(&request, &archive).await?;
        let staging = self
            .config
            .cache_root
            .join(format!(".staging-{name}-{version}"));
        remove_dir_if_exists(&staging)?;
        std::fs::create_dir_all(&staging)?;
        safe_extract_tar_gz(&archive, &staging)?;
        let source = find_pack_root(&staging)?;
        let manifest = PackManifest::load(&source)?;
        if manifest.pack.name != name || manifest.pack.version != version {
            return Err(ArgoError::Schema(format!(
                "archive manifest is {} {}, expected {name} {version}",
                manifest.pack.name, manifest.pack.version
            )));
        }
        remove_dir_if_exists(&root)?;
        copy_dir_all(&source, &root)?;
        remove_dir_if_exists(&staging)?;
        Ok(InstalledPack::cached(name, version, root))
    }

    /// Resolve a pack root, preferring a dev-link over the cache.
    pub fn resolve(&self, name: &str, version: Option<&str>) -> ArgoResult<Option<InstalledPack>> {
        if let Some(path) = DevLinkStore::new(&self.config.cache_root).resolve(name)? {
            let manifest = PackManifest::load(&path)?;
            return Ok(Some(InstalledPack {
                name: manifest.pack.name,
                version: manifest.pack.version,
                root: path,
                dev_link: true,
            }));
        }
        if let Some(version) = version {
            let root = self.pack_root(name, version);
            return Ok(root
                .join("pack.toml")
                .is_file()
                .then(|| InstalledPack::cached(name, version, root)));
        }
        Ok(self
            .list()?
            .into_iter()
            .filter(|row| row.name == name)
            .max_by(|a, b| a.version.cmp(&b.version)))
    }

    /// List cached packs.
    pub fn list(&self) -> ArgoResult<Vec<InstalledPack>> {
        let mut rows = Vec::new();
        if self.config.cache_root.is_dir() {
            for name in std::fs::read_dir(&self.config.cache_root)? {
                let name = name?;
                if !name.file_type()?.is_dir() {
                    continue;
                }
                for version in std::fs::read_dir(name.path())? {
                    let version = version?;
                    let root = version.path();
                    if root.join("pack.toml").is_file() {
                        let manifest = PackManifest::load(&root)?;
                        rows.push(InstalledPack::cached(
                            &manifest.pack.name,
                            &manifest.pack.version,
                            root,
                        ));
                    }
                }
            }
        }
        rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
        Ok(rows)
    }

    /// Link a pack name to a local checkout.
    pub fn dev_link(&self, name: &str, path: &Path) -> ArgoResult<InstalledPack> {
        DevLinkStore::new(&self.config.cache_root).set(name, path)?;
        let manifest = PackManifest::load(path)?;
        Ok(InstalledPack {
            name: manifest.pack.name,
            version: manifest.pack.version,
            root: path.to_path_buf(),
            dev_link: true,
        })
    }

    /// Remove a cached version, or all versions if `version` is `None`.
    pub fn remove(&self, name: &str, version: Option<&str>) -> ArgoResult<usize> {
        let mut removed = usize::from(DevLinkStore::new(&self.config.cache_root).remove(name)?);
        if let Some(version) = version {
            let root = self.pack_root(name, version);
            if root.exists() {
                remove_dir_if_exists(&root)?;
                removed += 1;
            }
            return Ok(removed);
        }
        let root = self.config.cache_root.join(name);
        if root.exists() {
            remove_dir_if_exists(&root)?;
            removed += 1;
        }
        Ok(removed)
    }

    fn request(&self, name: &str, version: &str, base_url: Option<&str>) -> FetchRequest {
        let mut config = self.config.fetcher.clone();
        if let Some(base_url) = base_url {
            config.base_url = base_url.to_string();
        }
        FetchRequest {
            name: name.to_string(),
            version: version.to_string(),
            config,
        }
    }

    fn pack_root(&self, name: &str, version: &str) -> PathBuf {
        self.config.cache_root.join(name).join(version)
    }
}

impl InstalledPack {
    fn cached(name: &str, version: &str, root: PathBuf) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            root,
            dev_link: false,
        }
    }
}

fn load_fetcher_config(path: &Path) -> ArgoResult<FetcherConfig> {
    let body = std::fs::read_to_string(path)?;
    #[derive(Deserialize)]
    struct Config {
        fetcher: FetcherConfig,
    }
    Ok(toml::from_str::<Config>(&body)
        .map_err(|e| ArgoError::Schema(format!("{}: {e}", path.display())))?
        .fetcher)
}

fn safe_extract_tar_gz(archive: &Path, dest: &Path) -> ArgoResult<()> {
    let file = std::fs::File::open(archive)
        .map_err(|e| ArgoError::Io(format!("open {}: {e}", archive.display())))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive
        .entries()
        .map_err(|e| ArgoError::Oci(e.to_string()))?
    {
        let mut entry = entry.map_err(|e| ArgoError::Oci(e.to_string()))?;
        let path = entry.path().map_err(|e| ArgoError::Oci(e.to_string()))?;
        if path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(ArgoError::Schema(format!(
                "unsafe archive path {}",
                path.display()
            )));
        }
        entry
            .unpack(dest.join(path.as_ref()))
            .map_err(|e| ArgoError::Oci(format!("extract archive: {e}")))?;
    }
    Ok(())
}

fn find_pack_root(root: &Path) -> ArgoResult<PathBuf> {
    if root.join("pack.toml").is_file() {
        return Ok(root.to_path_buf());
    }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.path().join("pack.toml").is_file() {
            return Ok(entry.path());
        }
    }
    Err(ArgoError::Schema(format!(
        "archive extracted under {} without pack.toml",
        root.display()
    )))
}

fn copy_dir_all(src: &Path, dst: &Path) -> ArgoResult<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if ty.is_file() {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn remove_dir_if_exists(path: &Path) -> ArgoResult<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::{FetchRequest, PackFetcher};
    use std::future::Future;
    use std::pin::Pin;

    #[derive(Default)]
    struct LocalFetcher;

    impl PackFetcher for LocalFetcher {
        fn fetch<'a>(
            &'a self,
            request: &'a FetchRequest,
            dest: &'a Path,
        ) -> Pin<Box<dyn Future<Output = ArgoResult<()>> + Send + 'a>> {
            Box::pin(async move {
                let local = request.url().trim_start_matches("file://").to_string();
                std::fs::copy(local, dest).unwrap();
                Ok(())
            })
        }
    }

    fn pack_dir(tmp: &tempfile::TempDir, name: &str, version: &str) -> PathBuf {
        let root = tmp.path().join(format!("{name}-{version}"));
        std::fs::create_dir_all(root.join("verticals").join(name)).unwrap();
        std::fs::write(
            root.join("pack.toml"),
            format!("[pack]\nname='{name}'\nversion='{version}'\n\n[[verticals]]\nid='{name}'\npath='verticals/{name}'\n"),
        )
        .unwrap();
        std::fs::write(
            root.join("verticals").join(name).join("manifest.toml"),
            "id='x'\n",
        )
        .unwrap();
        root
    }

    fn archive_pack(src: &Path, out: &Path) {
        let file = std::fs::File::create(out).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);
        tar.append_dir_all(src.file_name().unwrap(), src).unwrap();
        tar.finish().unwrap();
    }

    #[tokio::test]
    async fn installs_pack_from_archive_and_lists_it() {
        let tmp = tempfile::tempdir().unwrap();
        let src = pack_dir(&tmp, "ap", "1.0.0");
        let archive = tmp.path().join("ap-1.0.0.tar.gz");
        archive_pack(&src, &archive);
        let config = PackLoaderConfig::with_cache_dir(tmp.path().join("cache"));
        let loader = PackLoader::with_fetcher(config, LocalFetcher);
        let base = format!("file://{}", tmp.path().join("{asset}").display());
        let row = loader
            .install("ap", "1.0.0", Some(&base), false)
            .await
            .unwrap();
        assert!(row.root.join("pack.toml").is_file());
        assert_eq!(loader.list().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn cache_hit_avoids_redownload() {
        let tmp = tempfile::tempdir().unwrap();
        let src = pack_dir(&tmp, "ap", "1.0.0");
        let archive = tmp.path().join("ap-1.0.0.tar.gz");
        archive_pack(&src, &archive);
        let loader = PackLoader::with_fetcher(
            PackLoaderConfig::with_cache_dir(tmp.path().join("cache")),
            LocalFetcher,
        );
        let base = format!("file://{}", tmp.path().join("{asset}").display());
        loader
            .install("ap", "1.0.0", Some(&base), false)
            .await
            .unwrap();
        std::fs::remove_file(&archive).unwrap();
        let row = loader
            .install("ap", "1.0.0", Some(&base), false)
            .await
            .unwrap();
        assert!(row.root.join("pack.toml").is_file());
    }

    #[test]
    fn dev_link_resolution_wins_over_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let linked = pack_dir(&tmp, "ap", "2.0.0");
        let loader = PackLoader::with_fetcher(
            PackLoaderConfig::with_cache_dir(tmp.path().join("cache")),
            LocalFetcher,
        );
        loader.dev_link("ap", &linked).unwrap();
        let row = loader.resolve("ap", Some("1.0.0")).unwrap().unwrap();
        assert!(row.dev_link);
        assert_eq!(row.version, "2.0.0");
    }

    #[test]
    fn remove_deletes_cached_pack() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("cache").join("ap").join("1");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(
            cache.join("pack.toml"),
            "[pack]\nname='ap'\nversion='1'\n\n[[verticals]]\nid='ap'\npath='verticals/ap'\n",
        )
        .unwrap();
        let loader = PackLoader::with_fetcher(
            PackLoaderConfig::with_cache_dir(tmp.path().join("cache")),
            LocalFetcher,
        );
        assert_eq!(loader.remove("ap", Some("1")).unwrap(), 1);
        assert!(!cache.exists());
    }
}
